mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{get_with_cookie, insert_admin, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

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

async fn insert_catering_enquiry(pool: &PgPool, name: &str, event_type: &str, guest_count: i32) -> i64 {
    let event_date = (Utc::now() + Duration::days(30)).date_naive();
    sqlx::query_scalar!(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required, message)
        VALUES ($1, '+31612345678', 'c@example.com', $2, $3, $4, 'Amsterdam', ARRAY['catering'], NULL)
        RETURNING id
        "#,
        name,
        event_type,
        crate_chrono_date_to_offset(event_date),
        guest_count,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

// Test-local mirror of the app's own `chrono_date_to_offset` — the test
// binary has no access to that `pub(crate)` helper, and this is the only
// place outside the app itself that needs to bind a `NaiveDate` parameter.
fn crate_chrono_date_to_offset(date: chrono::NaiveDate) -> time::Date {
    use chrono::Datelike;
    time::Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).unwrap(),
        date.day() as u8,
    )
    .unwrap()
}

async fn insert_contact_message(pool: &PgPool, name: &str, subject: &str) -> i64 {
    sqlx::query_scalar!(
        r#"
        INSERT INTO contact_messages (name, email, phone, subject, message)
        VALUES ($1, 'c@example.com', '+31612345678', $2, 'Hello there.')
        RETURNING id
        "#,
        name,
        subject,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn catering_enquiries_list_requires_a_session(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/catering-enquiries", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn catering_enquiries_list_orders_newest_first(pool: PgPool) {
    let cookie = login(&pool).await;
    insert_catering_enquiry(&pool, "First", "wedding", 10).await;
    insert_catering_enquiry(&pool, "Second", "birthday", 20).await;
    insert_catering_enquiry(&pool, "Third", "corporate", 30).await;

    let (status, body) =
        get_with_cookie(pool, "/api/admin/catering-enquiries", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let enquiries = body["enquiries"].as_array().unwrap();
    assert_eq!(enquiries[0]["name"], "Third");
    assert_eq!(enquiries[1]["name"], "Second");
    assert_eq!(enquiries[2]["name"], "First");
}

#[sqlx::test]
async fn catering_enquiry_detail_requires_a_session(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/catering-enquiries/1", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn catering_enquiry_detail_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/catering-enquiries/999999", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn catering_enquiry_detail_returns_every_field(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_catering_enquiry(&pool, "Full Detail", "wedding", 100).await;

    let (status, body) = get_with_cookie(
        pool,
        &format!("/api/admin/catering-enquiries/{id}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["phone"], "+31612345678");
    assert_eq!(body["location"], "Amsterdam");
    assert_eq!(body["services_required"], json!(["catering"]));
    assert!(body["message"].is_null());
}

#[sqlx::test]
async fn contact_messages_list_requires_a_session(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/contact-messages", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn contact_message_detail_requires_a_session(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/contact-messages/1", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn contact_message_detail_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/contact-messages/999999", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn contact_message_detail_returns_full_message(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_contact_message(&pool, "Grace Hopper", "menu").await;

    let (status, body) = get_with_cookie(
        pool,
        &format!("/api/admin/contact-messages/{id}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Hello there.");
    assert_eq!(body["subject"], "menu");
}

#[sqlx::test]
async fn admin_list_pagination_respects_limit_and_offset(pool: PgPool) {
    let cookie = login(&pool).await;
    insert_catering_enquiry(&pool, "Alpha", "wedding", 10).await;
    insert_catering_enquiry(&pool, "Beta", "birthday", 20).await;
    insert_catering_enquiry(&pool, "Gamma", "corporate", 30).await;

    let (status, body) = get_with_cookie(
        pool,
        "/api/admin/catering-enquiries?limit=1&offset=1",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    let enquiries = body["enquiries"].as_array().unwrap();
    assert_eq!(enquiries.len(), 1);
    assert_eq!(enquiries[0]["name"], "Beta");
}
