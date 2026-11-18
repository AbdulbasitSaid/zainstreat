mod common;

use axum::http::StatusCode;
use common::{get, get_with_cookie, insert_admin, insert_category, post_with_cookie, put_with_cookie, session_cookie};
use serde_json::{Value, json};
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

fn rental(name: &str, price: &str, display_order: i32) -> Value {
    json!({
        "name": name,
        "description": "Keeps food warm on the buffet.",
        "price": price,
        "image_url": null,
        "is_available": true,
        "display_order": display_order,
    })
}

async fn create(pool: &PgPool, cookie: &str, body: Value) -> Value {
    let (status, body, _) = post_with_cookie(pool.clone(), "/api/admin/rentals", body, Some(cookie)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

#[sqlx::test]
async fn rentals_crud_round_trip(pool: PgPool) {
    let cookie = login(&pool).await;
    let second = create(&pool, &cookie, rental("Chafing Dish – 5 L", "12", 2)).await;
    let first = create(&pool, &cookie, rental("Chafing Dish – 10 L", "15.00", 1)).await;
    assert_eq!(first["price"], "15.00");

    let (status, list) = get(pool.clone(), "/api/rentals").await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Chafing Dish – 10 L", "Chafing Dish – 5 L"]);
    assert!(list[0].get("is_available").is_none(), "public shape stays small");

    // Edit, then hide: unavailable rentals leave the public list only.
    let mut edit = rental("Chafing Dish – 5 L", "13.50", 2);
    edit["is_available"] = json!(false);
    edit["updated_at"] = second["updated_at"].clone();
    let uri = format!("/api/admin/rentals/{}", second["id"]);
    let (status, edited) = put_with_cookie(pool.clone(), &uri, edit.clone(), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["price"], "13.50");

    let (_, list) = get(pool.clone(), "/api/rentals").await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (_, admin_list) = get_with_cookie(pool.clone(), "/api/admin/rentals", Some(&cookie)).await;
    assert_eq!(admin_list.as_array().unwrap().len(), 2);

    // The same (now stale) token again → 409 with the live rental.
    let (status, conflict) = put_with_cookie(pool.clone(), &uri, edit, Some(&cookie)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");

    let (status, archived, _) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/rentals/{}/archive", first["id"]),
        json!({ "updated_at": first["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert_eq!(archived["is_archived"], true);

    let (_, list) = get(pool.clone(), "/api/rentals").await;
    assert_eq!(list.as_array().unwrap().len(), 0);
    let (_, admin_list) = get_with_cookie(pool.clone(), "/api/admin/rentals", Some(&cookie)).await;
    assert_eq!(admin_list.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn rentals_validation_errors(pool: PgPool) {
    let cookie = login(&pool).await;
    let mut body = rental("  ", "-1", -1);
    body["image_url"] = json!("https://example.com/photo.jpg");
    let (status, response, _) = post_with_cookie(pool.clone(), "/api/admin/rentals", body, Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
    let fields: Vec<&str> =
        response["fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap()).collect();
    assert_eq!(fields, ["name", "price", "image_url", "display_order"]);

    let (status, response, _) =
        post_with_cookie(pool.clone(), "/api/admin/rentals", rental("Cooler", "10.005", 1), Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
    assert_eq!(response["fields"][0]["field"], "price");
}

#[sqlx::test]
async fn rentals_admin_requires_login(pool: PgPool) {
    let (status, _) = get_with_cookie(pool.clone(), "/api/admin/rentals", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = post_with_cookie(pool.clone(), "/api/admin/rentals", rental("Cooler", "10", 1), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

const DATA_MIGRATION: &str = include_str!("../migrations/20261118160100_snacks_rentals_data.up.sql");

// Phase 36 Decision 7: the data migration only touches a database that
// already has a menu, and a second run inserts nothing.
#[sqlx::test]
async fn snacks_rentals_data_migration_is_insert_if_missing(pool: PgPool) {
    let count = |sql: &'static str| {
        let pool = pool.clone();
        async move { sqlx::query_scalar::<_, i64>(sql).fetch_one(&pool).await.unwrap() }
    };
    const SNACKS: &str =
        "SELECT count(*) FROM menu_items m JOIN categories c ON c.id = m.category_id WHERE c.name = 'Snacks'";

    // Empty database (as at test start): nothing was inserted.
    assert_eq!(count("SELECT count(*) FROM categories").await, 0);
    assert_eq!(count("SELECT count(*) FROM rentals").await, 0);

    insert_category(&pool, "Individual Plates").await;
    sqlx::raw_sql(DATA_MIGRATION).execute(&pool).await.unwrap();
    assert_eq!(count(SNACKS).await, 13);
    assert_eq!(count("SELECT count(*) FROM rentals").await, 7);
    assert_eq!(
        count(
            "SELECT count(*) FROM menu_item_price_options p JOIN menu_items m ON m.id = p.menu_item_id \
             WHERE m.name = 'Puff Puff (small chops size)' AND m.price IS NULL"
        )
        .await,
        2
    );
    assert_eq!(count("SELECT count(*) FROM menu_items WHERE name = 'Meat Pie' AND min_quantity = 10").await, 1);
    assert_eq!(count("SELECT count(*) FROM menu_items WHERE name = 'Snack Combo 3' AND min_quantity = 5").await, 1);

    sqlx::raw_sql(DATA_MIGRATION).execute(&pool).await.unwrap();
    assert_eq!(count("SELECT count(*) FROM categories WHERE name = 'Snacks'").await, 1);
    assert_eq!(count(SNACKS).await, 13);
    assert_eq!(count("SELECT count(*) FROM rentals").await, 7);
    assert_eq!(count("SELECT count(*) FROM menu_item_price_options").await, 2);
}
