mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::post;
use serde_json::{json, Value};
use sqlx::PgPool;

fn valid_payload() -> Value {
    let future_date = (Utc::now() + Duration::days(30)).date_naive();
    json!({
        "name": "Ada Lovelace",
        "phone": "+31 6 1234 5678",
        "email": "ada@example.com",
        "event_type": "wedding",
        "event_date": future_date.to_string(),
        "guest_count": 50,
        "location": "Amsterdam, NL",
        "services_required": ["catering"],
        "message": null,
        "website": "",
    })
}

#[sqlx::test]
async fn catering_enquiry_create_succeeds_with_valid_payload(pool: PgPool) {
    let (status, body) = post(pool, "/api/catering-enquiries", valid_payload()).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].is_i64());
}

#[sqlx::test]
async fn catering_enquiry_create_requires_no_session(pool: PgPool) {
    let (status, _body) = post(pool, "/api/catering-enquiries", valid_payload()).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_an_empty_name(pool: PgPool) {
    let mut payload = valid_payload();
    payload["name"] = json!("");
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "name");
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_an_invalid_phone(pool: PgPool) {
    let mut payload = valid_payload();
    payload["phone"] = json!("1234567890");
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "phone"));
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_an_unknown_event_type(pool: PgPool) {
    let mut payload = valid_payload();
    payload["event_type"] = json!("quincea\u{f1}era");
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "event_type"));
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_a_past_event_date(pool: PgPool) {
    let mut payload = valid_payload();
    let yesterday = (Utc::now() - Duration::days(1)).date_naive();
    payload["event_date"] = json!(yesterday.to_string());
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "event_date" && f["message"] == "must_be_future"));
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_a_zero_guest_count(pool: PgPool) {
    let mut payload = valid_payload();
    payload["guest_count"] = json!(0);
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "guest_count"));
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_an_empty_services_required(pool: PgPool) {
    let mut payload = valid_payload();
    payload["services_required"] = json!([]);
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "services_required"));
}

#[sqlx::test]
async fn catering_enquiry_create_rejects_an_unknown_service(pool: PgPool) {
    let mut payload = valid_payload();
    payload["services_required"] = json!(["catering", "bouncy_castle"]);
    let (status, body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "services_required"));
}

#[sqlx::test]
async fn catering_enquiry_create_accepts_a_null_message(pool: PgPool) {
    let mut payload = valid_payload();
    payload["message"] = Value::Null;
    let (status, _body) = post(pool, "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn catering_enquiry_create_persists_services_required_as_submitted(pool: PgPool) {
    let mut payload = valid_payload();
    payload["services_required"] = json!(["catering", "event_rental"]);
    let (status, body) = post(pool.clone(), "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::CREATED);

    let id = body["id"].as_i64().unwrap();
    let services: Vec<String> = sqlx::query_scalar!(
        "SELECT services_required FROM catering_enquiries WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(services, vec!["catering".to_string(), "event_rental".to_string()]);
}

#[sqlx::test]
async fn catering_enquiry_create_honeypot_filled_reports_success_but_persists_nothing(pool: PgPool) {
    let mut payload = valid_payload();
    payload["website"] = json!("http://spam.example");
    let (status, _body) = post(pool.clone(), "/api/catering-enquiries", payload).await;
    assert_eq!(status, StatusCode::CREATED);

    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM catering_enquiries")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count, 0);
}
