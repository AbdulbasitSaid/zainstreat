mod common;

use axum::http::StatusCode;
use common::{get, post};
use serde_json::json;
use sqlx::PgPool;

/// Returns the order's id and its public token.
async fn insert_order(pool: &PgPool, status: &str) -> (i64, String) {
    let row = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type, subtotal, total, status)
        VALUES ('Ada', 'c@example.com', '+31612345678', 'pickup', 12.50, 12.50, $1)
        RETURNING id, public_token
        "#,
        status,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO order_items (order_id, item_name, unit_price, quantity, subtotal)
        VALUES ($1, 'Jollof Rice', 12.50, 1, 12.50)
        "#,
        row.id,
    )
    .execute(pool)
    .await
    .unwrap();

    (row.id, row.public_token.to_string())
}

async fn claimed_at(pool: &PgPool, id: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT payment_claimed_at FROM orders WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn order_is_readable_by_its_token(pool: PgPool) {
    let (id, token) = insert_order(&pool, "awaiting_payment").await;

    let (status, body) = get(pool, &format!("/api/orders/by-token/{token}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["public_token"], token);
    assert_eq!(body["payment_reference"], format!("ZTM-{id}"));
    assert_eq!(body["status"], "awaiting_payment");
    assert_eq!(body["items"][0]["item_name"], "Jollof Rice");
}

#[sqlx::test]
async fn unknown_and_malformed_tokens_both_404(pool: PgPool) {
    let (status, body) = get(
        pool.clone(),
        "/api/orders/by-token/00000000-0000-0000-0000-000000000000",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");

    let (status, _body) = get(pool.clone(), "/api/orders/by-token/not-a-uuid").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _body) = post(
        pool,
        "/api/orders/by-token/not-a-uuid/payment-claim",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn claim_moves_an_awaiting_order_to_payment_claimed(pool: PgPool) {
    let (id, token) = insert_order(&pool, "awaiting_payment").await;

    let (status, body) = post(
        pool.clone(),
        &format!("/api/orders/by-token/{token}/payment-claim"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "payment_claimed");
    assert!(claimed_at(&pool, id).await.is_some());
}

/// A double click must not re-stamp the claim (or re-email the owner).
#[sqlx::test]
async fn second_claim_is_a_no_op(pool: PgPool) {
    let (id, token) = insert_order(&pool, "awaiting_payment").await;
    let uri = format!("/api/orders/by-token/{token}/payment-claim");

    post(pool.clone(), &uri, json!({})).await;
    let first = claimed_at(&pool, id).await;

    let (status, body) = post(pool.clone(), &uri, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "payment_claimed");
    assert_eq!(claimed_at(&pool, id).await, first);
}

#[sqlx::test]
async fn claim_leaves_a_confirmed_order_alone(pool: PgPool) {
    let (id, token) = insert_order(&pool, "processing").await;

    let (status, body) = post(
        pool.clone(),
        &format!("/api/orders/by-token/{token}/payment-claim"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "processing");
    assert!(claimed_at(&pool, id).await.is_none());
}

#[sqlx::test]
async fn claim_on_a_cancelled_order_conflicts(pool: PgPool) {
    let (_id, token) = insert_order(&pool, "cancelled").await;

    let (status, body) = post(
        pool,
        &format!("/api/orders/by-token/{token}/payment-claim"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "invalid_status_transition");
}
