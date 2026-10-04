mod common;

use axum::http::StatusCode;
use common::{
    get_with_cookie, insert_admin, insert_category, patch_with_cookie, post_with_cookie,
    session_cookie,
};
use serde_json::json;
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

#[sqlx::test]
async fn categories_list_requires_a_session(pool: PgPool) {
    let (status, _body) = get_with_cookie(pool, "/api/admin/categories", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn category_create_requires_a_session(pool: PgPool) {
    let (status, _body, _cookie) = common::post_with_cookie(
        pool,
        "/api/admin/categories",
        json!({ "name": "Rice", "description": null, "updated_at": "2026-01-01T00:00:00Z" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn category_create_rejects_an_empty_name(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body, _set) = common::post_with_cookie(
        pool,
        "/api/admin/categories",
        json!({ "name": "  ", "description": null, "updated_at": "2026-01-01T00:00:00Z" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation_error");
    assert_eq!(body["fields"][0]["field"], "name");
}

#[sqlx::test]
async fn category_create_appends_after_the_current_max_display_order(pool: PgPool) {
    let cookie = login(&pool).await;
    sqlx::query!("INSERT INTO categories (name, display_order) VALUES ('Rice', 0)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("INSERT INTO categories (name, display_order) VALUES ('Soup', 1)")
        .execute(&pool)
        .await
        .unwrap();

    let (status, body, _set) = common::post_with_cookie(
        pool.clone(),
        "/api/admin/categories",
        json!({ "name": "Drinks", "description": null, "updated_at": "2026-01-01T00:00:00Z" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = body["id"].as_i64().unwrap();

    let display_order: i32 =
        sqlx::query_scalar!("SELECT display_order FROM categories WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(display_order, 2);
}

#[sqlx::test]
async fn category_rename_updates_name_and_description(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, body) = patch_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}"),
        json!({ "name": "New", "description": "d", "updated_at": updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "New");
    assert_eq!(body["description"], "d");
    assert_ne!(body["updated_at"], updated_at);
}

#[sqlx::test]
async fn category_rename_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, _body) = patch_with_cookie(
        pool,
        "/api/admin/categories/999999",
        json!({ "name": "New", "description": null, "updated_at": "2026-01-01T00:00:00Z" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn category_rename_rejects_a_stale_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let original_updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}"),
        json!({ "name": "First Rename", "description": null, "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "First Rename");

    let (status, conflict_body) = patch_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}"),
        json!({ "name": "Second Rename", "description": null, "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_body["error"], "conflict");
    assert_eq!(conflict_body["current"]["name"], "First Rename");
}

#[sqlx::test]
async fn category_archive_succeeds_when_no_active_items(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, body, _set) = post_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}/archive"),
        json!({ "updated_at": updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_archived"], true);
}

#[sqlx::test]
async fn category_archive_rejects_when_active_items_remain(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES ($1, 'Jollof', 10.00)",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, body, _set) = post_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}/archive"),
        json!({ "updated_at": updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "category_has_active_items");
    assert_eq!(body["items"][0]["name"], "Jollof");
}

#[sqlx::test]
async fn category_archive_succeeds_when_only_archived_items_remain(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price, deleted_at) VALUES ($1, 'Jollof', 10.00, now())",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, _body, _set) = post_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}/archive"),
        json!({ "updated_at": updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn category_archive_rejects_a_stale_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    let (_status, list_body) = get_with_cookie(pool.clone(), "/api/admin/categories", Some(&cookie)).await;
    let original_updated_at = list_body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()["updated_at"]
        .clone();

    let (status, _body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}"),
        json!({ "name": "Renamed", "description": null, "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}/archive"),
        json!({ "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "conflict");

    let is_archived: bool =
        sqlx::query_scalar!("SELECT deleted_at IS NOT NULL as \"archived!\" FROM categories WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!is_archived);
}
