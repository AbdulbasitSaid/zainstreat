mod common;

use axum::http::StatusCode;
use common::{
    get, get_with_cookie, insert_admin, insert_category, post_with_cookie, put_with_cookie,
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

async fn publish_category(pool: &PgPool, cookie: &str, id: i64, draft_updated_at: &serde_json::Value) {
    let (status, _body, _set) = post_with_cookie(
        pool.clone(),
        "/api/admin/publish",
        json!({ "entries": [{ "kind": "category", "id": id, "draft_updated_at": draft_updated_at }] }),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// Phase 25: a published category's rename is a draft until it's published.
#[sqlx::test]
async fn category_rename_is_a_draft_until_published(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;
    let (_status, before) = get_with_cookie(pool.clone(), &format!("/api/admin/categories/{id}"), Some(&cookie)).await;

    let (status, body) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}/draft"),
        json!({ "name": "New", "description": "d", "draft_updated_at": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Rice");
    assert_eq!(body["has_draft"], true);
    assert_eq!(body["draft"]["name"], "New");

    let (_status, public) = get(pool.clone(), "/api/categories").await;
    assert_eq!(public[0]["name"], "Rice");

    publish_category(&pool, &cookie, id, &body["draft_updated_at"]).await;

    let (_status, after) = get_with_cookie(pool.clone(), &format!("/api/admin/categories/{id}"), Some(&cookie)).await;
    assert_eq!(after["name"], "New");
    assert_eq!(after["description"], "d");
    assert_eq!(after["has_draft"], false);
    assert_ne!(after["updated_at"], before["updated_at"]);
    let (_status, public) = get(pool, "/api/categories").await;
    assert_eq!(public[0]["name"], "New");
}

#[sqlx::test]
async fn category_draft_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, _body) = put_with_cookie(
        pool,
        "/api/admin/categories/999999/draft",
        json!({ "name": "New", "description": null, "draft_updated_at": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn category_draft_rejects_a_stale_draft_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let id = insert_category(&pool, "Rice").await;

    let (status, body) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}/draft"),
        json!({ "name": "First Rename", "description": null, "draft_updated_at": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["draft"]["name"], "First Rename");

    let (status, conflict_body) = put_with_cookie(
        pool,
        &format!("/api/admin/categories/{id}/draft"),
        json!({ "name": "Second Rename", "description": null, "draft_updated_at": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_body["error"], "conflict");
    assert_eq!(conflict_body["current"]["draft"]["name"], "First Rename");
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

    let (status, draft) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/categories/{id}/draft"),
        json!({ "name": "Renamed", "description": null, "draft_updated_at": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    publish_category(&pool, &cookie, id, &draft["draft_updated_at"]).await;

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
