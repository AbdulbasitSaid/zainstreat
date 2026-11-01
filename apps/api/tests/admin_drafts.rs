mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    build_test_app, get, get_with_cookie, insert_admin, insert_category, post_multipart_with_cookie,
    post_with_cookie, put_with_cookie, session_cookie,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

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

async fn media_status(pool: &PgPool, url: &str) -> StatusCode {
    let path = url.rsplit_once("/api/media/").unwrap().1;
    let (status, _headers, _bytes) = common::get_raw_with_cookie(pool.clone(), &format!("/api/media/{path}"), None).await;
    status
}

async fn delete_with_cookie(pool: &PgPool, uri: &str, cookie: Option<&str>) -> StatusCode {
    let mut builder = Request::builder().method("DELETE").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let app = build_test_app(pool.clone()).await;
    app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap().status()
}

/// A published item (hand-inserted rows default to published) with a
/// legacy image path, as on staging's seed menu.
async fn insert_published_item(pool: &PgPool, category_id: i64) -> i64 {
    sqlx::query_scalar!(
        "INSERT INTO menu_items (category_id, name, price, image_url) VALUES ($1, 'Jollof', 13.00, '/images/menu/jollof.jpg') RETURNING id",
        category_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn item_fields(category_id: i64, name: &str, price: &str, image_url: &str) -> Value {
    json!({
        "category_id": category_id,
        "name": name,
        "description": null,
        "price": price,
        "price_options": [],
        "image_url": image_url,
        "is_featured": false,
    })
}

async fn save_item_draft(pool: &PgPool, cookie: &str, id: i64, mut fields: Value, token: Value) -> (StatusCode, Value) {
    fields["draft_updated_at"] = token;
    put_with_cookie(pool.clone(), &format!("/api/admin/menu-items/{id}/draft"), fields, Some(cookie)).await
}

async fn publish(pool: &PgPool, cookie: &str, entries: Value) -> (StatusCode, Value) {
    let (status, body, _set) =
        post_with_cookie(pool.clone(), "/api/admin/publish", json!({ "entries": entries }), Some(cookie)).await;
    (status, body)
}

async fn public_names(pool: &PgPool) -> Vec<String> {
    let (_status, body) = get(pool.clone(), "/api/menu-items").await;
    body.as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap().to_string()).collect()
}

#[sqlx::test]
async fn draft_routes_require_a_session(pool: PgPool) {
    for uri in [
        "/api/admin/site-content",
        "/api/admin/changes",
        "/api/admin/preview/menu-items",
        "/api/admin/preview/categories",
        "/api/admin/preview/site-content",
    ] {
        let (status, _body) = get_with_cookie(pool.clone(), uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    for uri in ["/api/admin/site-content/draft", "/api/admin/menu-items/1/draft", "/api/admin/categories/1/draft"] {
        let (status, _body) = put_with_cookie(pool.clone(), uri, json!({}), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let (status, _body, _set) = post_with_cookie(pool.clone(), "/api/admin/publish", json!({ "entries": [] }), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(delete_with_cookie(&pool, "/api/admin/drafts/site_content", None).await, StatusCode::UNAUTHORIZED);
    assert_eq!(delete_with_cookie(&pool, "/api/admin/drafts/menu_item/1", None).await, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn the_old_live_update_routes_are_gone(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;
    for uri in [format!("/api/admin/menu-items/{id}"), format!("/api/admin/categories/{category_id}")] {
        let (status, _body) = common::patch_with_cookie(pool.clone(), &uri, json!({}), Some(&cookie)).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
    }
}

#[sqlx::test]
async fn a_draft_is_previewed_but_not_public_until_published(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;

    let (status, body) = save_item_draft(
        &pool,
        &cookie,
        id,
        item_fields(category_id, "Jollof Deluxe", "15.00", "/images/menu/jollof.jpg"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["has_draft"], true);
    assert_eq!(body["is_published"], true);
    assert_eq!(body["name"], "Jollof");

    assert_eq!(public_names(&pool).await, ["Jollof"]);

    let (_status, preview) = get_with_cookie(pool.clone(), "/api/admin/preview/menu-items", Some(&cookie)).await;
    assert_eq!(preview[0]["name"], "Jollof Deluxe");
    assert_eq!(preview[0]["price"], "15.00");

    let (_status, changes) = get_with_cookie(pool.clone(), "/api/admin/changes", Some(&cookie)).await;
    assert_eq!(changes.as_array().unwrap().len(), 1);
    assert_eq!(changes[0]["kind"], "menu_item");
    assert_eq!(changes[0]["is_new"], false);
    let fields: Vec<&str> =
        changes[0]["changed_fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap()).collect();
    assert_eq!(fields, ["name", "price"]);
    assert_eq!(changes[0]["changed_fields"][0]["before"], "Jollof");
    let (_status, count) = get_with_cookie(pool.clone(), "/api/admin/changes?count=1", Some(&cookie)).await;
    assert_eq!(count["count"], 1);

    let (status, body) =
        publish(&pool, &cookie, json!([{ "kind": "menu_item", "id": id, "draft_updated_at": body["draft_updated_at"] }])).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["published"][0]["id"], id);

    assert_eq!(public_names(&pool).await, ["Jollof Deluxe"]);
    let (_status, changes) = get_with_cookie(pool, "/api/admin/changes", Some(&cookie)).await;
    assert!(changes.as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn a_stale_draft_updated_at_conflicts(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;
    let fields = item_fields(category_id, "Jollof Deluxe", "15.00", "/images/menu/jollof.jpg");

    let (_status, first) = save_item_draft(&pool, &cookie, id, fields.clone(), Value::Null).await;
    let (status, _body) = save_item_draft(&pool, &cookie, id, fields, Value::Null).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Publish with a token that predates the latest save: the whole request
    // rolls back.
    let (status, _body) = publish(
        &pool,
        &cookie,
        json!([
            { "kind": "site_content", "draft_updated_at": null },
            { "kind": "menu_item", "id": id, "draft_updated_at": "2020-01-01T00:00:00Z" },
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_status, item) = get_with_cookie(pool.clone(), &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;
    assert_eq!(item["has_draft"], true);
    assert_eq!(item["draft_updated_at"], first["draft_updated_at"]);
}

#[sqlx::test]
async fn an_invalid_draft_is_not_saved(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;

    let (status, body) =
        save_item_draft(&pool, &cookie, id, item_fields(category_id, "  ", "15.00", "/images/menu/jollof.jpg"), Value::Null)
            .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "name");

    let (_status, item) = get_with_cookie(pool, &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;
    assert_eq!(item["has_draft"], false);
}

#[sqlx::test]
async fn discard_restores_and_deletes_the_draft_photo(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;
    let draft_photo = upload_image(&pool, &cookie).await;

    let (status, _body) =
        save_item_draft(&pool, &cookie, id, item_fields(category_id, "Jollof", "13.00", &draft_photo), Value::Null).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(
        delete_with_cookie(&pool, &format!("/api/admin/drafts/menu_item/{id}"), Some(&cookie)).await,
        StatusCode::NO_CONTENT
    );

    let (_status, item) = get_with_cookie(pool.clone(), &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;
    assert_eq!(item["has_draft"], false);
    assert_eq!(item["image_url"], "/images/menu/jollof.jpg");
    assert_eq!(media_status(&pool, &draft_photo).await, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn publish_deletes_the_superseded_photo(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let first = upload_image(&pool, &cookie).await;
    let id: i64 = sqlx::query_scalar!(
        "INSERT INTO menu_items (category_id, name, price, image_url) VALUES ($1, 'Jollof', 13.00, $2) RETURNING id",
        category_id,
        first,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let second = upload_image(&pool, &cookie).await;

    let (_status, draft) =
        save_item_draft(&pool, &cookie, id, item_fields(category_id, "Jollof", "13.00", &second), Value::Null).await;
    // Saving the draft never deletes anything: the live photo still shows.
    assert_eq!(media_status(&pool, &first).await, StatusCode::OK);

    let (status, _body) =
        publish(&pool, &cookie, json!([{ "kind": "menu_item", "id": id, "draft_updated_at": draft["draft_updated_at"] }])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(media_status(&pool, &first).await, StatusCode::NOT_FOUND);
    assert_eq!(media_status(&pool, &second).await, StatusCode::OK);
}

#[sqlx::test]
async fn a_new_item_is_unlisted_until_published(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let photo = upload_image(&pool, &cookie).await;

    let (status, created, _set) = post_with_cookie(
        pool.clone(),
        "/api/admin/menu-items",
        item_fields(category_id, "Puff Puff", "4.00", &photo),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created:?}");
    assert_eq!(created["is_published"], false);
    let id = created["id"].as_i64().unwrap();

    // An unpublished item's edit writes its live columns: there's no
    // audience to protect.
    let (_status, saved) =
        save_item_draft(&pool, &cookie, id, item_fields(category_id, "Puff Puff (6)", "4.50", &photo), Value::Null).await;
    assert_eq!(saved["name"], "Puff Puff (6)");
    assert_eq!(saved["has_draft"], false);

    assert!(public_names(&pool).await.is_empty());
    let (_status, preview) = get_with_cookie(pool.clone(), "/api/admin/preview/menu-items", Some(&cookie)).await;
    assert_eq!(preview[0]["name"], "Puff Puff (6)");
    let (_status, changes) = get_with_cookie(pool.clone(), "/api/admin/changes", Some(&cookie)).await;
    assert_eq!(changes[0]["is_new"], true);

    let (status, _body) =
        publish(&pool, &cookie, json!([{ "kind": "menu_item", "id": id, "draft_updated_at": saved["draft_updated_at"] }])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(public_names(&pool).await, ["Puff Puff (6)"]);
}

#[sqlx::test]
async fn discarding_a_new_item_archives_it(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let photo = upload_image(&pool, &cookie).await;
    let (_status, created, _set) = post_with_cookie(
        pool.clone(),
        "/api/admin/menu-items",
        item_fields(category_id, "Puff Puff", "4.00", &photo),
        Some(&cookie),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    assert_eq!(
        delete_with_cookie(&pool, &format!("/api/admin/drafts/menu_item/{id}"), Some(&cookie)).await,
        StatusCode::NO_CONTENT
    );
    let (_status, item) = get_with_cookie(pool, &format!("/api/admin/menu-items/{id}"), Some(&cookie)).await;
    assert_eq!(item["is_archived"], true);
}

#[sqlx::test]
async fn archive_clears_a_pending_draft(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;
    let draft_photo = upload_image(&pool, &cookie).await;
    let (_status, draft) =
        save_item_draft(&pool, &cookie, id, item_fields(category_id, "Jollof", "13.00", &draft_photo), Value::Null).await;

    let (status, archived, _set) = post_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/archive"),
        json!({ "updated_at": draft["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{archived:?}");
    assert_eq!(archived["has_draft"], false);
    assert_eq!(media_status(&pool, &draft_photo).await, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn availability_stays_immediate_with_a_pending_draft(pool: PgPool) {
    let cookie = login(&pool).await;
    let category_id = insert_category(&pool, "Rice").await;
    let id = insert_published_item(&pool, category_id).await;
    let (_status, draft) = save_item_draft(
        &pool,
        &cookie,
        id,
        item_fields(category_id, "Jollof Deluxe", "15.00", "/images/menu/jollof.jpg"),
        Value::Null,
    )
    .await;

    let (status, _body) = common::patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/menu-items/{id}/availability"),
        json!({ "is_available": false, "updated_at": draft["updated_at"] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_status, body) = get(pool, "/api/menu-items").await;
    assert_eq!(body[0]["is_available"], false);
    assert_eq!(body[0]["name"], "Jollof");
}

#[sqlx::test]
async fn publish_revalidates_against_an_archived_category(pool: PgPool) {
    let cookie = login(&pool).await;
    let rice = insert_category(&pool, "Rice").await;
    let snacks = insert_category(&pool, "Snacks").await;
    let id = insert_published_item(&pool, rice).await;
    let (_status, draft) =
        save_item_draft(&pool, &cookie, id, item_fields(snacks, "Jollof", "13.00", "/images/menu/jollof.jpg"), Value::Null)
            .await;
    sqlx::query!("UPDATE categories SET deleted_at = now() WHERE id = $1", snacks).execute(&pool).await.unwrap();

    let (status, body) =
        publish(&pool, &cookie, json!([{ "kind": "menu_item", "id": id, "draft_updated_at": draft["draft_updated_at"] }])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "category_id");
}

#[sqlx::test]
async fn category_drafts_publish_and_preview(pool: PgPool) {
    let cookie = login(&pool).await;
    let (_status, created, _set) = post_with_cookie(
        pool.clone(),
        "/api/admin/categories",
        json!({ "name": "Desserts", "description": null }),
        Some(&cookie),
    )
    .await;
    assert_eq!(created["is_published"], false);
    let id = created["id"].as_i64().unwrap();

    let (_status, public) = get(pool.clone(), "/api/categories").await;
    assert!(public.as_array().unwrap().is_empty());
    let (_status, preview) = get_with_cookie(pool.clone(), "/api/admin/preview/categories", Some(&cookie)).await;
    assert_eq!(preview[0]["name"], "Desserts");

    let (status, _body) = publish(&pool, &cookie, json!([{ "kind": "category", "id": id, "draft_updated_at": null }])).await;
    assert_eq!(status, StatusCode::OK);
    let (_status, public) = get(pool, "/api/categories").await;
    assert_eq!(public[0]["name"], "Desserts");
}

#[sqlx::test]
async fn publish_rejects_an_unknown_kind(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = publish(&pool, &cookie, json!([{ "kind": "order", "id": 1, "draft_updated_at": null }])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "entries[0].kind");
}
