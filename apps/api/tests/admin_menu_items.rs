mod common;

use axum::http::StatusCode;
use common::{
    get_with_cookie, insert_admin, insert_category, patch_with_cookie, post_multipart_with_cookie,
    post_with_cookie, session_cookie,
};
use serde_json::{json, Value};
use sqlx::PgPool;

const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

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

async fn upload_image(pool: &PgPool, cookie: &str) -> String {
    let (status, body) = post_multipart_with_cookie(
        pool.clone(),
        "/api/admin/media",
        "file",
        "photo.png",
        "image/png",
        TINY_PNG.to_vec(),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "upload failed: {body:?}");
    body["url"].as_str().unwrap().to_string()
}

fn flat_price_payload(category_id: i64, name: &str, price: &str) -> Value {
    json!({
        "category_id": category_id,
        "name": name,
        "description": null,
        "price": price,
        "price_options": [],
        "image_url": null,
        "is_featured": false,
        "updated_at": "2026-01-01T00:00:00Z",
    })
}

async fn create_item(pool: &PgPool, cookie: &str, payload: Value) -> Value {
    let (status, body, _set) =
        post_with_cookie(pool.clone(), "/api/admin/menu-items", payload, Some(cookie)).await;
    assert_eq!(status, StatusCode::OK, "create failed: {body:?}");
    body
}

#[sqlx::test]
async fn menu_items_list_requires_a_session(pool: PgPool) {
    let (status, _body) = get_with_cookie(pool, "/api/admin/menu-items", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn menu_item_create_requires_a_session(pool: PgPool) {
    let category_id = insert_category(&pool, "Rice").await;
    let (status, _body, _set) = post_with_cookie(
        pool,
        "/api/admin/menu-items",
        flat_price_payload(category_id, "Jollof", "12.50"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn menu_item_create_with_flat_price_succeeds(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "12.50");
    payload["image_url"] = json!(url);
    let body = create_item(&pool, &cookie, payload).await;
    assert_eq!(body["price"], "12.50");
    assert_eq!(body["price_options"], json!([]));
}

#[sqlx::test]
async fn menu_item_create_with_price_options_succeeds(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let payload = json!({
        "category_id": category_id,
        "name": "Jollof",
        "description": null,
        "price": null,
        "price_options": [
            { "label": "Small", "price": "8.00" },
            { "label": "Large", "price": "14.00" },
        ],
        "image_url": url,
        "is_featured": false,
        "updated_at": "2026-01-01T00:00:00Z",
    });
    let body = create_item(&pool, &cookie, payload).await;
    assert_eq!(body["price"], Value::Null);
    assert_eq!(body["price_options"].as_array().unwrap().len(), 2);
    assert!(body["price_options"][0]["id"].is_i64());
}

#[sqlx::test]
async fn menu_item_create_rejects_both_price_and_price_options(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let payload = json!({
        "category_id": category_id,
        "name": "Jollof",
        "description": null,
        "price": "10.00",
        "price_options": [{ "label": "Small", "price": "8.00" }],
        "image_url": null,
        "is_featured": false,
        "updated_at": "2026-01-01T00:00:00Z",
    });
    let (status, body, _set) =
        post_with_cookie(pool, "/api/admin/menu-items", payload, Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "price_options"));
}

#[sqlx::test]
async fn menu_item_create_rejects_neither_price_nor_price_options(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let payload = json!({
        "category_id": category_id,
        "name": "Jollof",
        "description": null,
        "price": null,
        "price_options": [],
        "image_url": null,
        "is_featured": false,
        "updated_at": "2026-01-01T00:00:00Z",
    });
    let (status, body, _set) =
        post_with_cookie(pool, "/api/admin/menu-items", payload, Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"].as_array().unwrap().iter().any(|f| f["field"] == "price"));
}

#[sqlx::test]
async fn menu_item_create_rejects_a_negative_or_zero_price(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let (status, _body, _set) = post_with_cookie(
        pool,
        "/api/admin/menu-items",
        flat_price_payload(category_id, "Jollof", "0.00"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn menu_item_create_rejects_an_archived_or_unknown_category(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    sqlx::query!("UPDATE categories SET deleted_at = now() WHERE id = $1", category_id)
        .execute(&pool)
        .await
        .unwrap();

    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let (status, body, _set) = post_with_cookie(
        pool,
        "/api/admin/menu-items",
        payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "category_id");
}

#[sqlx::test]
async fn menu_item_create_rejects_a_missing_image_url(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let (status, body, _set) = post_with_cookie(
        pool,
        "/api/admin/menu-items",
        flat_price_payload(category_id, "Jollof", "10.00"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "image_url");
    assert_eq!(body["fields"][0]["message"], "required");
}

#[sqlx::test]
async fn menu_item_create_rejects_an_image_url_not_issued_by_this_api(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!("https://attacker.example/x.jpg");

    let (status, body, _set) =
        post_with_cookie(pool, "/api/admin/menu-items", payload, Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "image_url");
}

#[sqlx::test]
async fn menu_item_create_accepts_an_image_url_from_this_apis_own_upload(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;

    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);

    let body = create_item(&pool, &cookie, payload).await;
    assert_eq!(body["image_url"], url);
}

/// Phase 7's seed data predates this phase's upload pipeline and uses
/// relative asset paths (e.g. `/images/menu/jollof-rice-plantain.jpg`) that
/// don't match this API's issued shape. Editing an unrelated field on one of
/// those rows must still succeed — only a *new* or *changed* image_url has
/// to come from this API's own upload endpoint.
#[sqlx::test]
async fn menu_item_update_grandfathers_a_preexisting_non_api_image_url(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let legacy_url = "/images/menu/jollof-rice-plantain.jpg";
    let id: i64 = sqlx::query_scalar!(
        "INSERT INTO menu_items (category_id, name, price, image_url) VALUES ($1, 'Jollof', 10.00, $2) RETURNING id",
        category_id,
        legacy_url,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let (_status, item) =
        get_with_cookie(pool.clone(), &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;

    let mut update_payload = flat_price_payload(category_id, "Jollof Renamed", "10.00");
    update_payload["image_url"] = json!(legacy_url);
    update_payload["updated_at"] = item["updated_at"].clone();
    let (status, body) = patch_with_cookie(
        pool,
        &format!("/api/admin/menu-items/{id}"),
        update_payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {body:?}");
    assert_eq!(body["name"], "Jollof Renamed");
    assert_eq!(body["image_url"], legacy_url);
}

#[sqlx::test]
async fn menu_item_update_replaces_price_options(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let payload = json!({
        "category_id": category_id,
        "name": "Jollof",
        "description": null,
        "price": null,
        "price_options": [
            { "label": "Small", "price": "8.00" },
            { "label": "Large", "price": "14.00" },
        ],
        "image_url": url,
        "is_featured": false,
        "updated_at": "2026-01-01T00:00:00Z",
    });
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let update_payload = json!({
        "category_id": category_id,
        "name": "Jollof",
        "description": null,
        "price": null,
        "price_options": [{ "label": "Medium", "price": "10.00" }],
        "image_url": url,
        "is_featured": false,
        "updated_at": created["updated_at"],
    });
    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}"),
        update_payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {body:?}");
    assert_eq!(body["price_options"].as_array().unwrap().len(), 1);
    assert_eq!(body["price_options"][0]["label"], "Medium");

    let deleted_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM menu_item_price_options WHERE menu_item_id = $1 AND deleted_at IS NOT NULL",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(deleted_count, 2);

    let active_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM menu_item_price_options WHERE menu_item_id = $1 AND deleted_at IS NULL",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(active_count, 1);
}

#[sqlx::test]
async fn menu_item_update_404s_for_an_unknown_id(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let (status, _body) = patch_with_cookie(
        pool,
        "/api/admin/menu-items/999999",
        payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn menu_item_update_rejects_a_stale_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut create_payload = flat_price_payload(category_id, "Jollof", "10.00");
    create_payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, create_payload).await;
    let id = created["id"].as_i64().unwrap();
    let original_updated_at = created["updated_at"].clone();

    let mut first_update = flat_price_payload(category_id, "Jollof Updated", "11.00");
    first_update["image_url"] = json!(url);
    first_update["updated_at"] = original_updated_at.clone();
    let (status, first_body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}"),
        first_update,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let mut second_update = flat_price_payload(category_id, "Jollof Second", "12.00");
    second_update["image_url"] = json!(url);
    second_update["updated_at"] = original_updated_at;
    let (status, conflict_body) = patch_with_cookie(
        pool,
        &format!("/api/admin/menu-items/{id}"),
        second_update,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_body["error"], "conflict");
    assert_eq!(conflict_body["current"]["name"], first_body["name"]);
}

#[sqlx::test]
async fn availability_toggle_rejects_a_stale_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();
    let original_updated_at = created["updated_at"].clone();

    let (status, first_flip) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/availability"),
        json!({ "is_available": false, "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first_flip["is_available"], false);

    let (status, conflict_body) = patch_with_cookie(
        pool,
        &format!("/api/admin/menu-items/{id}/availability"),
        json!({ "is_available": true, "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_body["current"]["is_available"], false);
}

#[sqlx::test]
async fn menu_item_archive_rejects_a_stale_updated_at(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut create_payload = flat_price_payload(category_id, "Jollof", "10.00");
    create_payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, create_payload).await;
    let id = created["id"].as_i64().unwrap();
    let original_updated_at = created["updated_at"].clone();

    let mut rename = flat_price_payload(category_id, "Jollof Renamed", "10.00");
    rename["image_url"] = json!(url);
    rename["updated_at"] = original_updated_at.clone();
    let (status, _body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}"),
        rename,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, conflict_body, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/archive"),
        json!({ "updated_at": original_updated_at }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_body["error"], "conflict");

    let is_archived: bool = sqlx::query_scalar!(
        "SELECT deleted_at IS NOT NULL as \"archived!\" FROM menu_items WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!is_archived);
}

#[sqlx::test]
async fn menu_item_update_deletes_the_superseded_image_from_minio(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let first_url = upload_image(&pool, &cookie).await;

    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(first_url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let second_url = upload_image(&pool, &cookie).await;
    let mut update_payload = flat_price_payload(category_id, "Jollof", "10.00");
    update_payload["image_url"] = json!(second_url);
    update_payload["updated_at"] = created["updated_at"].clone();
    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}"),
        update_payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {body:?}");
    assert_eq!(body["image_url"], second_url);

    let first_path = first_url.rsplit_once("/api/media/").unwrap().1;
    let second_path = second_url.rsplit_once("/api/media/").unwrap().1;

    let (status, _headers, _bytes) =
        common::get_raw_with_cookie(pool.clone(), &format!("/api/media/{first_path}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _headers, _bytes) =
        common::get_raw_with_cookie(pool, &format!("/api/media/{second_path}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn menu_item_update_with_unchanged_image_url_does_not_delete_it(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;

    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let mut update_payload = flat_price_payload(category_id, "Jollof Renamed", "10.00");
    update_payload["image_url"] = json!(url);
    update_payload["updated_at"] = created["updated_at"].clone();
    let (status, _body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}"),
        update_payload,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let path = url.rsplit_once("/api/media/").unwrap().1;
    let (status, _headers, _bytes) =
        common::get_raw_with_cookie(pool, &format!("/api/media/{path}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn menu_item_archive_does_not_delete_its_image(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;

    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let (status, _body, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/archive"),
        json!({ "updated_at": created["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let path = url.rsplit_once("/api/media/").unwrap().1;
    let (status, _headers, _bytes) =
        common::get_raw_with_cookie(pool, &format!("/api/media/{path}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn availability_toggle_flips_is_available_only(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let (status, body) = patch_with_cookie(
        pool,
        &format!("/api/admin/menu-items/{id}/availability"),
        json!({ "is_available": false, "updated_at": created["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_available"], false);
    assert_eq!(body["name"], created["name"]);
    assert_eq!(body["price"], created["price"]);
}

#[sqlx::test]
async fn menu_item_archive_sets_deleted_at_and_is_archived_true(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let (status, body, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/archive"),
        json!({ "updated_at": created["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_archived"], true);

    let (status, body) =
        get_with_cookie(pool, &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_archived"], true);
}

#[sqlx::test]
async fn archived_menu_item_is_excluded_from_the_public_menu(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let url = upload_image(&pool, &cookie).await;
    let mut payload = flat_price_payload(category_id, "Jollof", "10.00");
    payload["image_url"] = json!(url);
    let created = create_item(&pool, &cookie, payload).await;
    let id = created["id"].as_i64().unwrap();

    let (status, _body, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/archive"),
        json!({ "updated_at": created["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_status, public_body) = get_with_cookie(pool, "/api/menu-items", None).await;
    let ids: Vec<i64> = public_body.as_array().unwrap().iter().map(|i| i["id"].as_i64().unwrap()).collect();
    assert!(!ids.contains(&id));
}

#[sqlx::test]
async fn category_archived_trigger_rejects_a_direct_insert_under_an_archived_category(pool: PgPool) {
    let category_id = insert_category(&pool, "Rice").await;
    sqlx::query!("UPDATE categories SET deleted_at = now() WHERE id = $1", category_id)
        .execute(&pool)
        .await
        .unwrap();

    let result = sqlx::query!(
        "INSERT INTO menu_items (category_id, name, price) VALUES ($1, 'Jollof', 10.00)",
        category_id
    )
    .execute(&pool)
    .await;

    let err = result.expect_err("insert under an archived category should be rejected");
    let sqlx::Error::Database(db_err) = err else {
        panic!("expected a database error, got {err:?}");
    };
    assert_eq!(db_err.code().as_deref(), Some("23514"));
}
