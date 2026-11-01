mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{
    build_test_app, get, get_with_cookie, insert_admin, patch_with_cookie, post, post_on_with_header,
    post_with_cookie, put_with_cookie, session_cookie,
};
use serde_json::{json, Value};
use sqlx::PgPool;

async fn insert_enquiry(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required, locale)
        VALUES ('Amina', '0612345678', 'amina@example.com', 'wedding', CURRENT_DATE + 30, 80,
                'Zaal De Linde, Utrecht', ARRAY['catering'], 'nl')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn login(pool: &PgPool) -> String {
    insert_admin(pool, "owner@example.com", "correct horse battery staple").await;
    let (_status, _body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    session_cookie(&set_cookie.expect("login should set a session cookie")).to_string()
}

fn quote_body(delivery_type: &str) -> Value {
    json!({
        "delivery_type": delivery_type,
        "delivery_address": "Zaal De Linde, Utrecht",
        "notes": null,
        "valid_until": (Utc::now() + Duration::days(14)).date_naive().to_string(),
        "items": [
            { "description": "Jollof rice per guest", "quantity": 80, "unit_price": "12.00" },
            { "description": "Serving staff", "quantity": 2, "unit_price": "125.00" },
        ],
    })
}

struct Seeded {
    cookie: String,
    enquiry: i64,
    token: String,
}

impl Seeded {
    fn uri(&self, action: &str) -> String {
        format!("/api/quotes/by-token/{}/{action}", self.token)
    }

    async fn save(&self, pool: &PgPool) -> Value {
        let (status, body) = put_with_cookie(
            pool.clone(),
            &format!("/api/admin/catering-enquiries/{}/quote", self.enquiry),
            quote_body("on_site"),
            Some(&self.cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        body
    }

    async fn send(&self, pool: &PgPool) {
        let (status, _, _) = post_with_cookie(
            pool.clone(),
            &format!("/api/admin/catering-enquiries/{}/quote/send", self.enquiry),
            json!({}),
            Some(&self.cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
}

/// A quote for an on-site event, saved and optionally sent through the admin API.
async fn seed(pool: &PgPool, sent: bool) -> Seeded {
    let cookie = login(pool).await;
    let enquiry = insert_enquiry(pool).await;
    let mut seeded = Seeded { cookie, enquiry, token: String::new() };
    let quote = seeded.save(pool).await;
    seeded.token = quote["quote_url"].as_str().unwrap().rsplit('/').next().unwrap().to_string();
    if sent {
        seeded.send(pool).await;
    }
    seeded
}

async fn order_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM orders").fetch_one(pool).await.unwrap()
}

#[sqlx::test]
async fn the_public_quote_never_exposes_contact_details(pool: PgPool) {
    let seeded = seed(&pool, true).await;

    let (status, body) = get(pool.clone(), &format!("/api/quotes/by-token/{}", seeded.token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_status"], "sent");
    assert_eq!(body["version"], 1);
    assert_eq!(body["customer_name"], "Amina");
    assert_eq!(body["total"], "1210.00");
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    let text = body.to_string();
    assert!(!text.contains("amina@example.com") && !text.contains("0612345678"));
}

#[sqlx::test]
async fn unknown_and_malformed_tokens_404(pool: PgPool) {
    let (status, _) = get(pool.clone(), "/api/quotes/by-token/not-a-uuid").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) =
        get(pool.clone(), "/api/quotes/by-token/00000000-0000-0000-0000-000000000000").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        pool.clone(),
        "/api/quotes/by-token/00000000-0000-0000-0000-000000000000/accept",
        json!({ "version": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Phase 24: a quote on a French enquiry creates a French order.
#[sqlx::test]
async fn accepting_a_french_quote_creates_a_french_order(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    sqlx::query("UPDATE catering_enquiries SET locale = 'fr'").execute(&pool).await.unwrap();

    let (status, _) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::OK);

    let locale: String = sqlx::query_scalar("SELECT locale FROM orders").fetch_one(&pool).await.unwrap();
    assert_eq!(locale, "fr");
}

#[sqlx::test]
async fn a_draft_cannot_be_accepted(pool: PgPool) {
    let seeded = seed(&pool, false).await;

    let (status, body) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "invalid_status_transition");
    assert_eq!(order_count(&pool).await, 0);
}

#[sqlx::test]
async fn accepting_creates_one_awaiting_payment_order(pool: PgPool) {
    let seeded = seed(&pool, true).await;

    let (status, body) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::OK);
    let order_token = body["order_token"].as_str().unwrap().to_string();

    let (_, order) = get(pool.clone(), &format!("/api/orders/by-token/{order_token}")).await;
    assert_eq!(order["status"], "awaiting_payment");
    assert_eq!(order["delivery_type"], "on_site");
    assert_eq!(order["delivery_address"], "Zaal De Linde, Utrecht");
    assert_eq!(order["customer_email"], "amina@example.com");
    assert_eq!(order["total"], "1210.00");
    assert_eq!(order["delivery_fee"].as_str().unwrap().parse::<f64>().unwrap(), 0.0);
    assert!(order["notes"].as_str().unwrap().starts_with("Catering quote #"));
    assert_eq!(order["items"][0]["item_name"], "Jollof rice per guest");
    assert_eq!(order["items"][1]["item_name"], "Serving staff");

    let row = sqlx::query!(
        r#"
        SELECT o.locale, q.status, q.order_id,
               (SELECT COUNT(*) FROM order_items i WHERE i.order_id = o.id AND i.menu_item_id IS NULL) as "lines!"
        FROM catering_quotes q JOIN orders o ON o.id = q.order_id
        "#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.locale, "nl");
    assert_eq!(row.status, "accepted");
    assert_eq!(row.lines, 2);

    // The public quote now points at the order.
    let (_, quote) = get(pool.clone(), &format!("/api/quotes/by-token/{}", seeded.token)).await;
    assert_eq!(quote["display_status"], "accepted");
    assert_eq!(quote["order_token"], order_token);
}

#[sqlx::test]
async fn accepting_again_returns_the_same_order(pool: PgPool) {
    let seeded = seed(&pool, true).await;

    let (_, first) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;
    // Even from a page showing an older version: the retry is idempotent.
    let (status, second) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 0 })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["order_token"], second["order_token"]);
    assert_eq!(order_count(&pool).await, 1);
}

#[sqlx::test]
async fn racing_accepts_create_one_order(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    let uri = seeded.uri("accept");

    let (a, b) = tokio::join!(
        post(pool.clone(), &uri, json!({ "version": 1 })),
        post(pool.clone(), &uri, json!({ "version": 1 })),
    );

    assert_eq!((a.0, b.0), (StatusCode::OK, StatusCode::OK));
    assert_eq!(a.1["order_token"], b.1["order_token"]);
    assert_eq!(order_count(&pool).await, 1);
}

#[sqlx::test]
async fn an_expired_quote_cannot_be_accepted(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    sqlx::query("UPDATE catering_quotes SET valid_until = CURRENT_DATE - 2").execute(&pool).await.unwrap();

    let (status, body) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "quote_expired");
    assert_eq!(order_count(&pool).await, 0);
}

/// Phase 23 Decision 13: a page still showing the old version can't accept
/// the one the owner revised and resent.
#[sqlx::test]
async fn a_stale_page_cannot_accept_or_decline_a_revised_quote(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    seeded.save(&pool).await;
    seeded.send(&pool).await;

    for action in ["accept", "decline"] {
        let (status, body) = post(pool.clone(), &seeded.uri(action), json!({ "version": 1 })).await;
        assert_eq!(status, StatusCode::CONFLICT, "{action}");
        assert_eq!(body["error"], "quote_changed", "{action}");
    }
    assert_eq!(order_count(&pool).await, 0);

    let (status, _) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 2 })).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn declining_is_idempotent_and_blocks_accepting(pool: PgPool) {
    let seeded = seed(&pool, true).await;

    let (status, body) =
        post(pool.clone(), &seeded.uri("decline"), json!({ "version": 1, "reason": "Over budget" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_status"], "declined");

    let (status, _) = post(pool.clone(), &seeded.uri("decline"), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::OK);
    let reason: Option<String> = sqlx::query_scalar("SELECT decline_reason FROM catering_quotes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reason.as_deref(), Some("Over budget"));

    let (status, _) = post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // The owner can revise a declined quote; it goes back to draft.
    let quote = seeded.save(&pool).await;
    assert_eq!(quote["status"], "draft");
    assert!(quote["decline_reason"].is_null());
}

#[sqlx::test]
async fn an_overlong_decline_reason_is_rejected(pool: PgPool) {
    let seeded = seed(&pool, true).await;

    let (status, body) =
        post(pool.clone(), &seeded.uri("decline"), json!({ "version": 1, "reason": "x".repeat(501) })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "reason");
}

#[sqlx::test]
async fn an_accepted_quote_is_final(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;

    let (status, _) = post(pool.clone(), &seeded.uri("decline"), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{}/quote", seeded.enquiry),
        quote_body("pickup"),
        Some(&seeded.cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _, _) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{}/quote/send", seeded.enquiry),
        json!({}),
        Some(&seeded.cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

/// The created order runs the Phase 20/22 lifecycle, on its on-site branch.
#[sqlx::test]
async fn the_order_from_an_on_site_quote_follows_the_on_site_branch(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    post(pool.clone(), &seeded.uri("accept"), json!({ "version": 1 })).await;
    let order_id: i64 = sqlx::query_scalar("SELECT order_id FROM catering_quotes").fetch_one(&pool).await.unwrap();
    let detail_uri = format!("/api/admin/orders/{order_id}");

    let (_, detail) = get_with_cookie(pool.clone(), &detail_uri, Some(&seeded.cookie)).await;
    assert_eq!(detail["catering_enquiry_id"], seeded.enquiry);

    for next in ["processing", "serving_on_site", "completed"] {
        let (_, detail) = get_with_cookie(pool.clone(), &detail_uri, Some(&seeded.cookie)).await;
        assert_eq!(detail["next_statuses"], json!([next, "cancelled"]), "before {next}");
        let (status, _) = patch_with_cookie(
            pool.clone(),
            &format!("/api/admin/orders/{order_id}/status"),
            json!({ "status": next }),
            Some(&seeded.cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "-> {next}");
    }
}

#[sqlx::test]
async fn accepting_is_rate_limited(pool: PgPool) {
    let seeded = seed(&pool, true).await;
    let app = build_test_app(pool).await;
    let uri = seeded.uri("accept");

    for _ in 0..5 {
        let (code, _) =
            post_on_with_header(app.clone(), &uri, json!({ "version": 1 }), "x-forwarded-for", "198.51.100.7").await;
        assert_eq!(code, StatusCode::OK);
    }
    let (code, _) = post_on_with_header(app, &uri, json!({ "version": 1 }), "x-forwarded-for", "198.51.100.7").await;
    assert_eq!(code, StatusCode::TOO_MANY_REQUESTS);
}
