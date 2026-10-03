use api::build_app;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn get(pool: PgPool, uri: &str) -> (StatusCode, Value) {
    let app = build_app(pool);
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

#[sqlx::test]
async fn categories_excludes_soft_deleted(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO categories (id, name, deleted_at) VALUES (2, 'Archived', now())"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/categories").await;

    assert_eq!(status, StatusCode::OK);
    let categories = body.as_array().unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0]["name"], "Rice Dishes");
}

#[sqlx::test]
async fn menu_items_unfiltered_excludes_soft_deleted(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes'), (2, 'Snacks')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES (1, 'Jollof Rice', 6000.00)"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES (2, 'Puff Puff', 1500.00)"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price, deleted_at) VALUES (2, 'Discontinued', 1000.00, now())"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 2);
}

#[sqlx::test]
async fn menu_items_filters_by_category_id(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes'), (2, 'Snacks')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES (1, 'Jollof Rice', 6000.00)"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES (2, 'Puff Puff', 1500.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items?category_id=2").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Puff Puff");
}

#[sqlx::test]
async fn menu_items_includes_unavailable_items(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price, is_available) VALUES (1, 'Fried Rice', 6000.00, false)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["is_available"], false);
}

#[sqlx::test]
async fn categories_ordered_by_display_order_then_id(pool: PgPool) {
    sqlx::query!(
        "INSERT INTO categories (id, name, display_order) VALUES (1, 'Bulk Orders', 2), (2, 'Individual Plates', 1)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/categories").await;

    assert_eq!(status, StatusCode::OK);
    let categories = body.as_array().unwrap();
    assert_eq!(categories[0]["name"], "Individual Plates");
    assert_eq!(categories[1]["name"], "Bulk Orders");
}

#[sqlx::test]
async fn menu_items_flat_price_has_empty_price_options(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Individual Plates')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES (1, 'Jollof Rice & Plantain', 13.00)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["price"], "13.00");
    assert_eq!(items[0]["price_options"], serde_json::json!([]));
}

#[sqlx::test]
async fn menu_items_with_price_options_have_null_price_and_ordered_options(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Soups & Stews')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Egusi Soup', NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_item_price_options (menu_item_id, label, price, display_order) VALUES (1, '3 L', 95.00, 2), (1, '2 L', 75.00, 1)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0]["price"].is_null());
    let options = items[0]["price_options"].as_array().unwrap();
    assert_eq!(options.len(), 2);
    assert_eq!(options[0]["label"], "2 L");
    assert_eq!(options[0]["price"], "75.00");
    assert_eq!(options[1]["label"], "3 L");
    assert_eq!(options[1]["price"], "95.00");
}

#[sqlx::test]
async fn menu_items_excludes_soft_deleted_price_options(pool: PgPool) {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Soups & Stews')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Egusi Soup', NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO menu_item_price_options (menu_item_id, label, price, deleted_at) VALUES (1, 'Discontinued Size', 50.00, now())"
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = get(pool, "/api/menu-items").await;

    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items[0]["price_options"], serde_json::json!([]));
}
