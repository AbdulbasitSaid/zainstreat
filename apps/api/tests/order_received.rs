mod common;

use axum::http::StatusCode;
use common::{build_test_app, get, post, post_on_with_header};
use serde_json::json;
use sqlx::PgPool;

/// Returns the order's id and its public token.
async fn insert_order(pool: &PgPool, status: &str, delivery_type: &str) -> (i64, String) {
    let row = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type, subtotal, total, status)
        VALUES ('Ada', 'c@example.com', '+31612345678', $2, 12.50, 12.50, $1)
        RETURNING id, public_token
        "#,
        status,
        delivery_type,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    (row.id, row.public_token.to_string())
}

async fn completed_at(pool: &PgPool, id: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT completed_at FROM orders WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn customer_completes_a_ready_order(pool: PgPool) {
    for (status, delivery_type) in [("ready_for_collection", "pickup"), ("out_for_delivery", "delivery")] {
        let (id, token) = insert_order(&pool, status, delivery_type).await;

        let (code, body) =
            post(pool.clone(), &format!("/api/orders/by-token/{token}/received"), json!({})).await;
        assert_eq!(code, StatusCode::OK, "{status}");
        assert_eq!(body["status"], "completed");
        assert_eq!(body["completed_by"], "customer");
        assert!(body["completed_at"].is_string());
        assert!(completed_at(&pool, id).await.is_some());
    }
}

/// A double click must not re-stamp the completion.
#[sqlx::test]
async fn second_confirmation_is_a_no_op(pool: PgPool) {
    let (id, token) = insert_order(&pool, "ready_for_collection", "pickup").await;
    let uri = format!("/api/orders/by-token/{token}/received");

    post(pool.clone(), &uri, json!({})).await;
    let first = completed_at(&pool, id).await;

    let (code, body) = post(pool.clone(), &uri, json!({})).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert_eq!(completed_at(&pool, id).await, first);
}

#[sqlx::test]
async fn orders_not_yet_handed_over_conflict(pool: PgPool) {
    for status in ["awaiting_payment", "payment_claimed", "processing", "cancelled"] {
        let (id, token) = insert_order(&pool, status, "pickup").await;

        let (code, body) =
            post(pool.clone(), &format!("/api/orders/by-token/{token}/received"), json!({})).await;
        assert_eq!(code, StatusCode::CONFLICT, "{status}");
        assert_eq!(body["error"], "invalid_status_transition");
        assert!(completed_at(&pool, id).await.is_none());
    }
}

#[sqlx::test]
async fn unknown_and_malformed_tokens_404(pool: PgPool) {
    let (code, _) = post(
        pool.clone(),
        "/api/orders/by-token/00000000-0000-0000-0000-000000000000/received",
        json!({}),
    )
    .await;
    assert_eq!(code, StatusCode::NOT_FOUND);

    let (code, _) = post(pool, "/api/orders/by-token/not-a-uuid/received", json!({})).await;
    assert_eq!(code, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn token_response_carries_the_tracking_timestamps(pool: PgPool) {
    let (_id, token) = insert_order(&pool, "awaiting_payment", "pickup").await;

    let (code, body) = get(pool, &format!("/api/orders/by-token/{token}")).await;
    assert_eq!(code, StatusCode::OK);
    for field in ["payment_claimed_at", "payment_confirmed_at", "ready_at", "completed_at", "cancelled_at", "completed_by"] {
        assert!(body[field].is_null(), "{field}");
    }
}

// Burst size 5, same limiter as payment-claim (rate_limit.rs).
#[sqlx::test]
async fn confirmation_is_rate_limited(pool: PgPool) {
    let (_id, token) = insert_order(&pool, "ready_for_collection", "pickup").await;
    let app = build_test_app(pool).await;
    let uri = format!("/api/orders/by-token/{token}/received");

    for _ in 0..5 {
        let (code, _) = post_on_with_header(app.clone(), &uri, json!({}), "x-forwarded-for", "198.51.100.7").await;
        assert_eq!(code, StatusCode::OK);
    }
    let (code, _) = post_on_with_header(app, &uri, json!({}), "x-forwarded-for", "198.51.100.7").await;
    assert_eq!(code, StatusCode::TOO_MANY_REQUESTS);
}
