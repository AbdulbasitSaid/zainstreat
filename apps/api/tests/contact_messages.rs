mod common;

use axum::http::StatusCode;
use common::post;
use serde_json::{json, Value};
use sqlx::PgPool;

fn valid_payload() -> Value {
    json!({
        "name": "Ada Lovelace",
        "email": "ada@example.com",
        "phone": "+31 6 1234 5678",
        "subject": "general",
        "message": "I'd like to ask about your menu.",
        "website": "",
    })
}

#[sqlx::test]
async fn contact_message_create_succeeds_with_valid_payload(pool: PgPool) {
    let (status, _body) = post(pool, "/api/contact-messages", valid_payload()).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn contact_message_create_rejects_an_unknown_subject(pool: PgPool) {
    let mut payload = valid_payload();
    payload["subject"] = json!("refund");
    let (status, body) = post(pool, "/api/contact-messages", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "subject"));
}

#[sqlx::test]
async fn contact_message_create_rejects_an_empty_message(pool: PgPool) {
    let mut payload = valid_payload();
    payload["message"] = json!("   ");
    let (status, body) = post(pool, "/api/contact-messages", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "message" && f["message"] == "required"));
}

#[sqlx::test]
async fn contact_message_create_rejects_an_overlong_message(pool: PgPool) {
    let mut payload = valid_payload();
    payload["message"] = json!("a".repeat(1001));
    let (status, body) = post(pool, "/api/contact-messages", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "message" && f["message"] == "too_long"));
}

#[sqlx::test]
async fn contact_message_create_accepts_event_rental_subject(pool: PgPool) {
    let mut payload = valid_payload();
    payload["subject"] = json!("event_rental");
    let (status, _body) = post(pool, "/api/contact-messages", payload).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn contact_message_create_honeypot_filled_reports_success_but_persists_nothing(pool: PgPool) {
    let mut payload = valid_payload();
    payload["website"] = json!("http://spam.example");
    let (status, _body) = post(pool.clone(), "/api/contact-messages", payload).await;
    assert_eq!(status, StatusCode::CREATED);

    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM contact_messages")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count, 0);
}
