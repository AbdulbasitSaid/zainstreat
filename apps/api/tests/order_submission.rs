mod common;

use axum::http::StatusCode;
use common::post;
use serde_json::{json, Value};
use sqlx::PgPool;

fn valid_payload(items: Value) -> Value {
    json!({
        "customer_name": "Ada Lovelace",
        "customer_email": "ada@example.com",
        "customer_phone": "+31 6 1234 5678",
        "delivery_type": "pickup",
        "delivery_address": null,
        "notes": null,
        "items": items,
    })
}

#[sqlx::test]
async fn happy_path_flat_priced_item_pickup(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 2 }
    ]));

    let (status, body) = post(pool.clone(), "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["total"], "26.00");
    assert_eq!(body["subtotal"], "26.00");
    assert_eq!(body["items"][0]["item_name"], "Jollof Rice");
    assert_eq!(body["items"][0]["unit_price"], "13.00");

    let order_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM orders")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(order_count, 1);

    let item_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM order_items")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item_count, 1);
}

#[sqlx::test]
async fn happy_path_priced_option_item(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Soups & Stews')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Egusi Soup', NULL)"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO menu_item_price_options (id, menu_item_id, label, price) VALUES (1, 1, '2 L', 75.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": 1, "quantity": 1 }
    ]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["items"][0]["option_label"], "2 L");
    assert_eq!(body["items"][0]["unit_price"], "75.00");
    assert_eq!(body["total"], "75.00");
}

#[sqlx::test]
async fn delivery_without_address_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["delivery_type"] = json!("delivery");
    payload["delivery_address"] = json!(null);

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "delivery_address"));
}

#[sqlx::test]
async fn empty_items_is_rejected(pool: PgPool) {
    let payload = valid_payload(json!([]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "items"));
}

#[sqlx::test]
async fn invalid_phone_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_phone"] = json!("call me maybe");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "customer_phone" && f["message"] == "invalid"));
}

#[sqlx::test]
async fn repeated_digit_phone_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_phone"] = json!("0000000000");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "customer_phone" && f["message"] == "invalid"));
}

#[sqlx::test]
async fn sequential_digit_phone_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_phone"] = json!("0123456789");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "customer_phone" && f["message"] == "invalid"));
}

#[sqlx::test]
async fn phone_without_leading_zero_or_plus_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_phone"] = json!("612345678");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "customer_phone" && f["message"] == "invalid"));
}

#[sqlx::test]
async fn nl_local_format_phone_is_accepted(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_phone"] = json!("06 1234 5678");

    let (status, _body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn too_short_name_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["customer_name"] = json!("7");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "customer_name" && f["message"] == "invalid"));
}

#[sqlx::test]
async fn too_long_notes_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["notes"] = json!("x".repeat(501));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "notes" && f["message"] == "too_long"));
}

#[sqlx::test]
async fn too_short_delivery_address_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));
    payload["delivery_type"] = json!("delivery");
    payload["delivery_address"] = json!("abc");

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "delivery_address" && f["message"] == "too_short"));
}

#[sqlx::test]
async fn non_positive_quantity_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 0 }
    ]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    let fields = body["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["field"] == "items[0].quantity"));
}

#[sqlx::test]
async fn unavailable_item_rejects_whole_order_with_rollback(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price, is_available) VALUES (2, 1, 'Fried Rice', 14.00, false)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 },
        { "menu_item_id": 2, "price_option_id": null, "quantity": 1 }
    ]));

    let (status, body) = post(pool.clone(), "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "items_unavailable");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["reason"], "unavailable");
    assert_eq!(items[0]["name"], "Fried Rice");

    let order_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM orders")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(order_count, 0);

    let item_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM order_items")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item_count, 0);
}

#[sqlx::test]
async fn soft_deleted_item_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price, deleted_at) VALUES (1, 1, 'Discontinued', 13.00, now())"
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 1 }
    ]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "items_unavailable");
    assert_eq!(body["items"][0]["reason"], "not_found");
}

#[sqlx::test]
async fn mismatched_price_option_selection_is_rejected(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Flat-priced item submitted with a non-null price_option_id.
    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": 999, "quantity": 1 }
    ]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "items_unavailable");
    assert_eq!(body["items"][0]["reason"], "invalid_selection");
}

#[sqlx::test]
async fn response_reflects_current_db_price_not_client_supplied_data(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    // The request DTO has no unit_price/name field at all — only
    // menu_item_id/price_option_id/quantity — so there is nothing for a
    // malicious client to override; the response price always comes from
    // this current DB row.
    let payload = valid_payload(json!([
        { "menu_item_id": 1, "price_option_id": null, "quantity": 3 }
    ]));

    let (status, body) = post(pool, "/api/orders", payload).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["items"][0]["unit_price"], "13.00");
    assert_eq!(body["total"], "39.00");
}
