mod common;

use axum::http::StatusCode;
use common::{get_with_cookie, insert_admin, patch_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

/// `order_items.menu_item_id` is a nullable FK, so an order can be seeded
/// without any categories/menu_items rows.
async fn insert_order(pool: &PgPool, customer: &str, status: &str, total: &str) -> i64 {
    let total: rust_decimal::Decimal = total.parse().unwrap();
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             notes, subtotal, total, status)
        VALUES ($1, 'c@example.com', '+31612345678', 'pickup', 'no onions', $2, $2, $3)
        RETURNING id
        "#,
        customer,
        total,
        status,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO order_items
            (order_id, item_name, option_label, unit_price, quantity, subtotal)
        VALUES ($1, 'Jollof Rice', '2 L', $2, 1, $2)
        "#,
        id,
        total,
    )
    .execute(pool)
    .await
    .unwrap();

    id
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

#[sqlx::test]
async fn orders_list_requires_a_session(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/orders", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn order_detail_requires_a_session(pool: PgPool) {
    let (status, _body) = get_with_cookie(pool, "/api/admin/orders/1", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn status_update_requires_a_session(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "new", "10.00").await;

    let (status, _body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "cancelled" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let row_status: String = sqlx::query_scalar!("SELECT status FROM orders WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_status, "new");
}

#[sqlx::test]
async fn orders_list_returns_newest_first_with_envelope(pool: PgPool) {
    insert_order(&pool, "Ada", "new", "10.00").await;
    insert_order(&pool, "Bo", "new", "20.00").await;
    let newest = insert_order(&pool, "Cy", "new", "42.00").await;

    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(pool, "/api/admin/orders", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["limit"], 25);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["orders"][0]["id"], newest);
    assert_eq!(body["orders"][0]["total"], "42.00");
}

#[sqlx::test]
async fn orders_list_filters_by_status(pool: PgPool) {
    insert_order(&pool, "Ada", "new", "10.00").await;
    insert_order(&pool, "Bo", "new", "20.00").await;
    insert_order(&pool, "Cy", "ready", "30.00").await;

    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/orders?status=ready", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["status"], "ready");
}

#[sqlx::test]
async fn orders_list_paginates(pool: PgPool) {
    insert_order(&pool, "Ada", "new", "10.00").await;
    insert_order(&pool, "Bo", "new", "20.00").await;
    insert_order(&pool, "Cy", "new", "30.00").await;

    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/orders?limit=2&offset=2", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 2);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn orders_list_rejects_an_unknown_status(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/orders?status=banana", Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    assert_eq!(body["fields"][0]["field"], "status");
}

#[sqlx::test]
async fn orders_list_rejects_an_out_of_range_limit(pool: PgPool) {
    let cookie = login(&pool).await;

    let (status, body) =
        get_with_cookie(pool.clone(), "/api/admin/orders?limit=0", Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "limit");

    let (status, body) =
        get_with_cookie(pool, "/api/admin/orders?limit=101", Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "limit");
}

#[sqlx::test]
async fn order_detail_returns_items(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "new", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, &format!("/api/admin/orders/{id}"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["item_name"], "Jollof Rice");
    assert_eq!(body["items"][0]["option_label"], "2 L");
    assert_eq!(body["notes"], "no onions");
    assert!(body["customer_email"].is_string());
}

#[sqlx::test]
async fn order_detail_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, "/api/admin/orders/999999", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn status_update_changes_status_and_bumps_updated_at(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "new", "10.00").await;

    let before: (String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "SELECT status, created_at, updated_at FROM orders WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "preparing" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "preparing");

    let after: (String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "SELECT status, created_at, updated_at FROM orders WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(after.0, "preparing");
    assert!(after.2 > after.1);
    assert_ne!(before.0, after.0);
}

#[sqlx::test]
async fn status_update_allows_moving_backward(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "ready", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool,
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "new" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "new");
}

#[sqlx::test]
async fn status_update_rejects_an_unknown_status(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "new", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "banana" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    assert_eq!(body["fields"][0]["field"], "status");

    let row_status: String = sqlx::query_scalar!("SELECT status FROM orders WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_status, "new");
}

#[sqlx::test]
async fn status_update_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool,
        "/api/admin/orders/999999/status",
        json!({ "status": "ready" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn summary_counts_every_status_and_recent_orders(pool: PgPool) {
    insert_order(&pool, "Ada", "new", "10.00").await;
    insert_order(&pool, "Bo", "new", "20.00").await;
    insert_order(&pool, "Cy", "ready", "30.00").await;

    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(pool, "/api/admin/orders/summary", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);

    let counts = body["counts"].as_object().unwrap();
    assert_eq!(counts.len(), 6);
    assert_eq!(counts["new"], 2);
    assert_eq!(counts["ready"], 1);
    assert_eq!(counts["confirmed"], 0);
    assert_eq!(counts["preparing"], 0);
    assert_eq!(counts["completed"], 0);
    assert_eq!(counts["cancelled"], 0);

    assert_eq!(body["today"], 3);
    assert!(body["recent"].as_array().unwrap().len() <= 5);
}
