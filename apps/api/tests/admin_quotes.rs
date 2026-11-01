mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{get_with_cookie, insert_admin, post_with_cookie, put_with_cookie, session_cookie};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn insert_enquiry(pool: &PgPool, locale: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required, locale)
        VALUES ('Amina', '0612345678', 'amina@example.com', 'wedding', CURRENT_DATE + 30, 80,
                'Zaal De Linde, Utrecht', ARRAY['catering'], $1)
        RETURNING id
        "#,
    )
    .bind(locale)
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

fn in_days(days: i64) -> String {
    (Utc::now() + Duration::days(days)).date_naive().to_string()
}

fn valid_quote() -> Value {
    json!({
        "delivery_type": "on_site",
        "delivery_address": "Zaal De Linde, Utrecht",
        "notes": "Halal, no pork.",
        "valid_until": in_days(14),
        "items": [
            { "description": "Jollof rice per guest", "quantity": 80, "unit_price": "12.00" },
            { "description": "Serving staff", "quantity": 2, "unit_price": "125.00" },
        ],
    })
}

async fn save(pool: &PgPool, cookie: &str, enquiry_id: i64, body: Value) -> (StatusCode, Value) {
    put_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry_id}/quote"),
        body,
        Some(cookie),
    )
    .await
}

async fn send(pool: &PgPool, cookie: &str, enquiry_id: i64) -> (StatusCode, Value) {
    let (status, body, _) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry_id}/quote/send"),
        json!({}),
        Some(cookie),
    )
    .await;
    (status, body)
}

fn field_errors(body: &Value) -> Vec<(String, String)> {
    body["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["field"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[sqlx::test]
async fn saving_creates_a_draft_with_the_computed_total(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "nl").await;
    let cookie = login(&pool).await;

    let (status, body) = save(&pool, &cookie, enquiry, valid_quote()).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "draft");
    assert_eq!(body["display_status"], "draft");
    assert_eq!(body["version"], 1);
    assert_eq!(body["total"], "1210.00");
    assert_eq!(body["delivery_type"], "on_site");
    assert_eq!(body["items"][0]["description"], "Jollof rice per guest");
    assert_eq!(body["items"][0]["subtotal"], "960.00");
    assert_eq!(body["items"][1]["description"], "Serving staff");
    // The share link is in the enquiry's language (Decision 14).
    assert!(body["quote_url"].as_str().unwrap().contains("/nl/quote/"));
}

#[sqlx::test]
async fn saving_again_replaces_the_lines_and_bumps_the_version(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;
    save(&pool, &cookie, enquiry, valid_quote()).await;

    let mut fewer = valid_quote();
    fewer["items"] =
        json!([{ "description": "Puff puff tray", "quantity": 3, "unit_price": "20" }]);
    let (status, body) = save(&pool, &cookie, enquiry, fewer).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], 2);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"], "60.00");
}

#[sqlx::test]
async fn pickup_quotes_drop_the_address(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;
    let mut pickup = valid_quote();
    pickup["delivery_type"] = json!("pickup");

    let (status, body) = save(&pool, &cookie, enquiry, pickup).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["delivery_address"].is_null());
}

#[sqlx::test]
async fn invalid_quotes_are_rejected_field_by_field(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;

    let mut cases: Vec<(Value, &str, &str)> = Vec::new();

    let mut no_items = valid_quote();
    no_items["items"] = json!([]);
    cases.push((no_items, "items", "empty"));

    let mut too_many = valid_quote();
    too_many["items"] = json!(
        (0..51)
            .map(|_| json!({ "description": "x", "quantity": 1, "unit_price": "1" }))
            .collect::<Vec<_>>()
    );
    cases.push((too_many, "items", "too_many"));

    let mut negative = valid_quote();
    negative["items"][0]["unit_price"] = json!("-1.00");
    cases.push((negative, "items[0].unit_price", "out_of_range"));

    let mut too_precise = valid_quote();
    too_precise["items"][0]["unit_price"] = json!("1.005");
    cases.push((too_precise, "items[0].unit_price", "too_precise"));

    let mut blank_description = valid_quote();
    blank_description["items"][1]["description"] = json!("  ");
    cases.push((blank_description, "items[1].description", "required"));

    let mut zero_quantity = valid_quote();
    zero_quantity["items"][0]["quantity"] = json!(0);
    cases.push((zero_quantity, "items[0].quantity", "out_of_range"));

    let mut no_address = valid_quote();
    no_address["delivery_type"] = json!("delivery");
    no_address["delivery_address"] = json!(null);
    cases.push((no_address, "delivery_address", "required"));

    let mut expired = valid_quote();
    expired["valid_until"] = json!(in_days(-2));
    cases.push((expired, "valid_until", "must_be_future"));

    let mut free = valid_quote();
    free["items"] = json!([{ "description": "Tasting", "quantity": 1, "unit_price": "0" }]);
    cases.push((free, "total", "must_be_positive"));

    for (body, field, message) in cases {
        let (status, response) = save(&pool, &cookie, enquiry, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(response["error"], "validation_error");
        assert!(
            field_errors(&response).contains(&(field.to_string(), message.to_string())),
            "{field}/{message}: {response}"
        );
    }
}

#[sqlx::test]
async fn sending_marks_it_sent_and_saving_pulls_it_back_to_draft(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;
    save(&pool, &cookie, enquiry, valid_quote()).await;

    let (status, body) = send(&pool, &cookie, enquiry).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["quote"]["status"], "sent");
    assert!(body["quote"]["sent_at"].is_string());
    // No SES in tests: admin is told to share the link instead.
    assert_eq!(body["email_queued"], false);

    let (_, body) = save(&pool, &cookie, enquiry, valid_quote()).await;
    assert_eq!(body["status"], "draft");
    assert_eq!(body["version"], 2);
}

#[sqlx::test]
async fn sending_an_out_of_date_quote_is_rejected(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;
    save(&pool, &cookie, enquiry, valid_quote()).await;
    sqlx::query("UPDATE catering_quotes SET valid_until = CURRENT_DATE - 2")
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = send(&pool, &cookie, enquiry).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        field_errors(&body),
        vec![("valid_until".to_string(), "must_be_future".to_string())]
    );
}

#[sqlx::test]
async fn unknown_enquiries_and_missing_quotes_404(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;

    let (status, _) = save(&pool, &cookie, 999_999, valid_quote()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&pool, &cookie, enquiry).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn quote_routes_require_a_session(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "en").await;

    let (status, _) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry}/quote"),
        valid_quote(),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry}/quote/send"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn enquiry_detail_and_list_carry_the_quote(pool: PgPool) {
    let enquiry = insert_enquiry(&pool, "nl").await;
    let other = insert_enquiry(&pool, "en").await;
    let cookie = login(&pool).await;

    let (_, detail) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(detail["locale"], "nl");
    assert!(detail["quote"].is_null());

    save(&pool, &cookie, enquiry, valid_quote()).await;
    send(&pool, &cookie, enquiry).await;

    let (_, detail) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/catering-enquiries/{enquiry}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(detail["quote"]["status"], "sent");
    assert_eq!(detail["quote"]["items"].as_array().unwrap().len(), 2);

    let (_, list) =
        get_with_cookie(pool.clone(), "/api/admin/catering-enquiries", Some(&cookie)).await;
    let status_of = |id: i64| {
        list["enquiries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == id)
            .unwrap()["quote_status"]
            .clone()
    };
    assert_eq!(status_of(enquiry), "sent");
    assert!(status_of(other).is_null());

    // Expiry is computed, not stored.
    sqlx::query("UPDATE catering_quotes SET valid_until = CURRENT_DATE - 2")
        .execute(&pool)
        .await
        .unwrap();
    let (_, list) =
        get_with_cookie(pool.clone(), "/api/admin/catering-enquiries", Some(&cookie)).await;
    let expired = list["enquiries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == enquiry)
        .unwrap()
        .clone();
    assert_eq!(expired["quote_status"], "expired");
}
