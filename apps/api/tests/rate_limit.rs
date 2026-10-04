mod common;

use axum::http::StatusCode;
use common::{build_test_app, post_on_with_header};
use serde_json::json;
use sqlx::PgPool;

fn valid_contact_payload() -> serde_json::Value {
    json!({
        "name": "Ada Lovelace",
        "email": "ada@example.com",
        "phone": "+31 6 1234 5678",
        "subject": "general",
        "message": "Hello there.",
        "website": "",
    })
}

fn valid_catering_payload() -> serde_json::Value {
    let future_date = (chrono::Utc::now() + chrono::Duration::days(30)).date_naive();
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

// Burst size is 5 (rate_limit.rs's `enquiry_rate_limiter`), refilling one
// token every 30s — far slower than this test runs, so the 6th request in
// quick succession always lands on an exhausted bucket.
#[sqlx::test]
async fn catering_enquiry_create_rate_limits_after_burst_size_requests(pool: PgPool) {
    let app = build_test_app(pool).await;

    for _ in 0..5 {
        let (status, _body) = post_on_with_header(
            app.clone(),
            "/api/catering-enquiries",
            valid_catering_payload(),
            "x-forwarded-for",
            "198.51.100.1",
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (status, _body) = post_on_with_header(
        app.clone(),
        "/api/catering-enquiries",
        valid_catering_payload(),
        "x-forwarded-for",
        "198.51.100.1",
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn catering_enquiry_create_rate_limits_independently_per_forwarded_for_value(pool: PgPool) {
    let app = build_test_app(pool).await;

    for _ in 0..5 {
        let (status, _body) = post_on_with_header(
            app.clone(),
            "/api/catering-enquiries",
            valid_catering_payload(),
            "x-forwarded-for",
            "1.1.1.1",
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (exhausted_status, _body) = post_on_with_header(
        app.clone(),
        "/api/catering-enquiries",
        valid_catering_payload(),
        "x-forwarded-for",
        "1.1.1.1",
    )
    .await;
    assert_eq!(exhausted_status, StatusCode::TOO_MANY_REQUESTS);

    let (fresh_status, _body) = post_on_with_header(
        app.clone(),
        "/api/catering-enquiries",
        valid_catering_payload(),
        "x-forwarded-for",
        "2.2.2.2",
    )
    .await;
    assert_eq!(fresh_status, StatusCode::CREATED);
}

#[sqlx::test]
async fn contact_message_create_rate_limits_after_burst_size_requests(pool: PgPool) {
    let app = build_test_app(pool).await;

    for _ in 0..5 {
        let (status, _body) = post_on_with_header(
            app.clone(),
            "/api/contact-messages",
            valid_contact_payload(),
            "x-forwarded-for",
            "203.0.113.9",
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (status, _body) = post_on_with_header(
        app.clone(),
        "/api/contact-messages",
        valid_contact_payload(),
        "x-forwarded-for",
        "203.0.113.9",
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}
