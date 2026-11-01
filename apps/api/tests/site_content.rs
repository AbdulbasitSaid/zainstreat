mod common;

use axum::http::StatusCode;
use common::{get, get_with_cookie, insert_admin, post_multipart_with_cookie, post_with_cookie, put_with_cookie, session_cookie};
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

async fn save_draft(pool: &PgPool, cookie: &str, content: Value, token: Value) -> (StatusCode, Value) {
    put_with_cookie(
        pool.clone(),
        "/api/admin/site-content/draft",
        json!({ "content": content, "draft_updated_at": token }),
        Some(cookie),
    )
    .await
}

#[sqlx::test]
async fn site_content_starts_empty(pool: PgPool) {
    let (status, body) = get(pool, "/api/site-content").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "images": {
        "logo": null, "hero": null, "category_meals": null, "category_snacks": null,
        "category_catering": null, "category_event_rentals": null, "about_hero": null,
        "about_mission": null, "contact": null,
    }, "home_text": {} }));
}

#[sqlx::test]
async fn site_content_draft_is_previewed_then_published(pool: PgPool) {
    let cookie = login(&pool).await;
    let hero = upload_image(&pool, &cookie).await;

    let (status, saved) = save_draft(
        &pool,
        &cookie,
        json!({
            "images": { "hero": hero },
            "home_text": { "fr": { "heroHeadline": "  Le <em>goût</em> du Nigeria  ", "heroEyebrow": "" } },
        }),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved:?}");
    // Trimmed, and the empty override dropped (Decision 12).
    assert_eq!(saved["draft"]["home_text"], json!({ "fr": { "heroHeadline": "Le <em>goût</em> du Nigeria" } }));

    let (_status, public) = get(pool.clone(), "/api/site-content").await;
    assert!(public["images"]["hero"].is_null());
    let (_status, preview) = get_with_cookie(pool.clone(), "/api/admin/preview/site-content", Some(&cookie)).await;
    assert_eq!(preview["images"]["hero"], hero);

    let (_status, changes) = get_with_cookie(pool.clone(), "/api/admin/changes", Some(&cookie)).await;
    assert_eq!(changes[0]["kind"], "site_content");
    let fields: Vec<&str> =
        changes[0]["changed_fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap()).collect();
    assert_eq!(fields, ["images.hero", "home_text.fr.heroHeadline"]);

    let (status, _body, _set) = post_with_cookie(
        pool.clone(),
        "/api/admin/publish",
        json!({ "entries": [{ "kind": "site_content", "draft_updated_at": saved["draft_updated_at"] }] }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_status, public) = get(pool.clone(), "/api/site-content").await;
    assert_eq!(public["images"]["hero"], hero);
    assert_eq!(public["home_text"]["fr"]["heroHeadline"], "Le <em>goût</em> du Nigeria");
    let (_status, admin) = get_with_cookie(pool, "/api/admin/site-content", Some(&cookie)).await;
    assert!(admin["draft"].is_null());
}

#[sqlx::test]
async fn site_content_rejects_bad_values_with_the_field(pool: PgPool) {
    let cookie = login(&pool).await;
    for (content, field) in [
        (json!({ "home_text": { "en": { "notAKey": "x" } } }), "home_text.en.notAKey"),
        (json!({ "home_text": { "en": { "heroHeadline": "<script>alert(1)</script>" } } }), "home_text.en.heroHeadline"),
        (json!({ "home_text": { "de": { "heroEyebrow": "Hallo" } } }), "home_text.de"),
        (json!({ "images": { "hero": "https://evil.example/hero.jpg" } }), "images.hero"),
    ] {
        let (status, body) = save_draft(&pool, &cookie, content, Value::Null).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(body["fields"][0]["field"], field);
    }
}

#[sqlx::test]
async fn site_content_draft_rejects_a_stale_token(pool: PgPool) {
    let cookie = login(&pool).await;
    let content = json!({ "home_text": { "en": { "heroEyebrow": "Hello" } } });
    let (status, _body) = save_draft(&pool, &cookie, content.clone(), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _body) = save_draft(&pool, &cookie, content, Value::Null).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn discarding_site_content_deletes_its_draft_images(pool: PgPool) {
    let cookie = login(&pool).await;
    let logo = upload_image(&pool, &cookie).await;
    let (status, _saved) = save_draft(&pool, &cookie, json!({ "images": { "logo": logo } }), Value::Null).await;
    assert_eq!(status, StatusCode::OK);

    let app = common::build_test_app(pool.clone()).await;
    let response = tower::ServiceExt::oneshot(
        app,
        axum::http::Request::builder()
            .method("DELETE")
            .uri("/api/admin/drafts/site_content")
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let path = logo.rsplit_once("/api/media/").unwrap().1;
    let (status, _headers, _bytes) = common::get_raw_with_cookie(pool.clone(), &format!("/api/media/{path}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_status, admin) = get_with_cookie(pool, "/api/admin/site-content", Some(&cookie)).await;
    assert!(admin["draft"].is_null());
}
