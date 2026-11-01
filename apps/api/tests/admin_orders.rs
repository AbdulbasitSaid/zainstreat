mod common;

use axum::http::StatusCode;
use common::{get_with_cookie, insert_admin, patch_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

/// `order_items.menu_item_id` is a nullable FK, so an order can be seeded
/// without any categories/menu_items rows.
async fn insert_order(pool: &PgPool, customer: &str, status: &str, total: &str) -> i64 {
    insert_order_of_type(pool, customer, status, total, "pickup").await
}

async fn insert_order_of_type(
    pool: &PgPool,
    customer: &str,
    status: &str,
    total: &str,
    delivery_type: &str,
) -> i64 {
    let total: rust_decimal::Decimal = total.parse().unwrap();
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             notes, subtotal, total, status)
        VALUES ($1, 'c@example.com', '+31612345678', $4, 'no onions', $2, $2, $3)
        RETURNING id
        "#,
        customer,
        total,
        status,
        delivery_type,
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
    let id = insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;

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
    assert_eq!(row_status, "awaiting_payment");
}

#[sqlx::test]
async fn orders_list_returns_newest_first_with_envelope(pool: PgPool) {
    insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;
    insert_order(&pool, "Bo", "awaiting_payment", "20.00").await;
    let newest = insert_order(&pool, "Cy", "awaiting_payment", "42.00").await;

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
    insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;
    insert_order(&pool, "Bo", "awaiting_payment", "20.00").await;
    insert_order(&pool, "Cy", "ready_for_collection", "30.00").await;

    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(
        pool,
        "/api/admin/orders?status=ready_for_collection",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["status"], "ready_for_collection");
    assert!(
        body["orders"][0]["payment_reference"]
            .as_str()
            .unwrap()
            .starts_with("ZTM-")
    );
}

#[sqlx::test]
async fn orders_list_paginates(pool: PgPool) {
    insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;
    insert_order(&pool, "Bo", "awaiting_payment", "20.00").await;
    insert_order(&pool, "Cy", "awaiting_payment", "30.00").await;

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

    let (status, body) = get_with_cookie(pool, "/api/admin/orders?limit=101", Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "limit");
}

#[sqlx::test]
async fn order_detail_returns_items(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) =
        get_with_cookie(pool, &format!("/api/admin/orders/{id}"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["item_name"], "Jollof Rice");
    assert_eq!(body["items"][0]["option_label"], "2 L");
    assert_eq!(body["notes"], "no onions");
    assert!(body["customer_email"].is_string());
    assert_eq!(body["payment_reference"], format!("ZTM-{id}"));
    assert!(body["payment_claimed_at"].is_null());
    assert!(body["payment_confirmed_at"].is_null());
}

#[sqlx::test]
async fn order_detail_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(pool, "/api/admin/orders/999999", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn confirming_payment_moves_to_processing_and_stamps_it(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "payment_claimed", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "processing" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "processing");

    let after: (
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT status, created_at, updated_at, payment_confirmed_at FROM orders WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(after.0, "processing");
    assert!(after.2 > after.1);
    assert!(after.3.is_some());
}

#[sqlx::test]
async fn payment_can_be_confirmed_without_a_customer_claim(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool,
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": "processing" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "processing");
}

/// Walks both branches end to end through the API (Phase 20 Decision 3).
#[sqlx::test]
async fn pickup_and_delivery_follow_their_own_branch(pool: PgPool) {
    let cookie = login(&pool).await;

    for (delivery_type, handover) in [
        ("pickup", "ready_for_collection"),
        ("delivery", "out_for_delivery"),
        ("on_site", "serving_on_site"),
    ] {
        let id =
            insert_order_of_type(&pool, "Ada", "awaiting_payment", "10.00", delivery_type).await;
        for next in ["processing", handover, "completed"] {
            // The detail's `next_statuses` is what the admin UI renders as
            // buttons, so it must offer exactly the step being taken.
            let (_, detail) = get_with_cookie(
                pool.clone(),
                &format!("/api/admin/orders/{id}"),
                Some(&cookie),
            )
            .await;
            assert_eq!(
                detail["next_statuses"],
                json!([next, "cancelled"]),
                "{delivery_type}: before {next}"
            );

            let (status, body) = patch_with_cookie(
                pool.clone(),
                &format!("/api/admin/orders/{id}/status"),
                json!({ "status": next }),
                Some(&cookie),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{delivery_type}: -> {next}");
            assert_eq!(body["status"], next);
        }

        let (_, detail) = get_with_cookie(
            pool.clone(),
            &format!("/api/admin/orders/{id}"),
            Some(&cookie),
        )
        .await;
        assert_eq!(
            detail["next_statuses"],
            json!([]),
            "{delivery_type}: completed is terminal"
        );
    }
}

#[sqlx::test]
async fn status_update_rejects_invalid_transitions(pool: PgPool) {
    let pickup = insert_order(&pool, "Ada", "processing", "10.00").await;
    let delivery = insert_order_of_type(&pool, "Bo", "processing", "10.00", "delivery").await;
    let awaiting = insert_order(&pool, "Cy", "awaiting_payment", "10.00").await;
    let completed = insert_order(&pool, "Di", "completed", "10.00").await;
    let cancelled = insert_order(&pool, "Ed", "cancelled", "10.00").await;

    let cookie = login(&pool).await;
    for (id, target, expected_status) in [
        (pickup, "out_for_delivery", "processing"),
        (delivery, "ready_for_collection", "processing"),
        (awaiting, "payment_claimed", "awaiting_payment"),
        (awaiting, "completed", "awaiting_payment"),
        (completed, "cancelled", "completed"),
        (cancelled, "processing", "cancelled"),
    ] {
        let (status, body) = patch_with_cookie(
            pool.clone(),
            &format!("/api/admin/orders/{id}/status"),
            json!({ "status": target }),
            Some(&cookie),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CONFLICT,
            "{expected_status} -> {target}"
        );
        assert_eq!(body["error"], "invalid_status_transition");

        let row_status: String = sqlx::query_scalar!("SELECT status FROM orders WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_status, expected_status);
    }
}

#[sqlx::test]
async fn status_update_rejects_an_unknown_status(pool: PgPool) {
    let id = insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;

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
    assert_eq!(row_status, "awaiting_payment");
}

#[sqlx::test]
async fn status_update_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = patch_with_cookie(
        pool,
        "/api/admin/orders/999999/status",
        json!({ "status": "cancelled" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn summary_counts_every_status_and_recent_orders(pool: PgPool) {
    insert_order(&pool, "Ada", "awaiting_payment", "10.00").await;
    insert_order(&pool, "Bo", "awaiting_payment", "20.00").await;
    insert_order(&pool, "Cy", "ready_for_collection", "30.00").await;

    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(pool, "/api/admin/orders/summary", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);

    let counts = body["counts"].as_object().unwrap();
    assert_eq!(counts.len(), 8);
    assert_eq!(counts["awaiting_payment"], 2);
    assert_eq!(counts["ready_for_collection"], 1);
    assert_eq!(counts["payment_claimed"], 0);
    assert_eq!(counts["processing"], 0);
    assert_eq!(counts["out_for_delivery"], 0);
    assert_eq!(counts["serving_on_site"], 0);
    assert_eq!(counts["completed"], 0);
    assert_eq!(counts["cancelled"], 0);

    assert_eq!(body["today"], 3);
    assert!(body["recent"].as_array().unwrap().len() <= 5);
}

async fn patch_status(
    pool: &PgPool,
    cookie: &str,
    id: i64,
    status: &str,
) -> (StatusCode, serde_json::Value) {
    patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/status"),
        json!({ "status": status }),
        Some(cookie),
    )
    .await
}

/// Phase 22 Decision 4: each step stamps its time for the tracking page.
#[sqlx::test]
async fn status_steps_stamp_their_timestamps(pool: PgPool) {
    let cookie = login(&pool).await;

    for (delivery_type, handover) in [
        ("pickup", "ready_for_collection"),
        ("delivery", "out_for_delivery"),
        ("on_site", "serving_on_site"),
    ] {
        let id =
            insert_order_of_type(&pool, "Ada", "awaiting_payment", "10.00", delivery_type).await;
        for next in ["processing", handover, "completed"] {
            assert_eq!(
                patch_status(&pool, &cookie, id, next).await.0,
                StatusCode::OK
            );
        }

        let (_, detail) = get_with_cookie(
            pool.clone(),
            &format!("/api/admin/orders/{id}"),
            Some(&cookie),
        )
        .await;
        for field in ["payment_confirmed_at", "ready_at", "completed_at"] {
            assert!(detail[field].is_string(), "{delivery_type}: {field}");
        }
        assert!(detail["cancelled_at"].is_null());
        assert_eq!(detail["completed_by"], "admin");
    }

    let cancelled = insert_order(&pool, "Bo", "processing", "10.00").await;
    assert_eq!(
        patch_status(&pool, &cookie, cancelled, "cancelled").await.0,
        StatusCode::OK
    );
    let (_, detail) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{cancelled}"),
        Some(&cookie),
    )
    .await;
    assert!(detail["cancelled_at"].is_string());
}

/// Phase 22 Decision 8: only an order the customer completed can be
/// reopened, back to its ready status.
#[sqlx::test]
async fn customer_completed_orders_can_be_reopened(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_order_of_type(&pool, "Ada", "completed", "10.00", "delivery").await;
    sqlx::query!(
        "UPDATE orders SET completed_by = 'customer', completed_at = now(), ready_at = now() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_, detail) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(detail["next_statuses"], json!(["out_for_delivery"]));

    let (status, body) = patch_status(&pool, &cookie, id, "out_for_delivery").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "out_for_delivery");

    let (_, detail) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}"),
        Some(&cookie),
    )
    .await;
    assert!(detail["completed_at"].is_null());
    assert!(detail["completed_by"].is_null());
    assert!(detail["ready_at"].is_string());
}

#[sqlx::test]
async fn admin_completed_orders_cannot_be_reopened(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_order(&pool, "Ada", "completed", "10.00").await;
    sqlx::query!("UPDATE orders SET completed_by = 'admin' WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = patch_status(&pool, &cookie, id, "ready_for_collection").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "invalid_status_transition");
}

#[sqlx::test]
async fn summary_lists_the_orders_waiting_on_payment_confirmation(pool: PgPool) {
    let first = insert_order(&pool, "Ada", "payment_claimed", "10.00").await;
    insert_order(&pool, "Bo", "awaiting_payment", "10.00").await;
    let second = insert_order(&pool, "Cy", "payment_claimed", "10.00").await;

    let cookie = login(&pool).await;
    let (status, body) = get_with_cookie(pool, "/api/admin/orders/summary", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["claimed_ids"], json!([first, second]));
}
