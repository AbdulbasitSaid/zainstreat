mod common;

use axum::http::StatusCode;
use common::{get_raw_with_cookie, insert_admin, post_multipart_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
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

#[sqlx::test]
async fn upload_requires_a_session(pool: PgPool) {
    let (status, _body) = post_multipart_with_cookie(
        pool,
        "/api/admin/media",
        "file",
        "photo.png",
        "image/png",
        TINY_PNG.to_vec(),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn upload_rejects_an_unsupported_content_type(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = post_multipart_with_cookie(
        pool,
        "/api/admin/media",
        "file",
        "notes.txt",
        "text/plain",
        b"hello".to_vec(),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "file");
}

#[sqlx::test]
async fn upload_rejects_an_oversized_file(pool: PgPool) {
    let cookie = login(&pool).await;
    let oversized = vec![0u8; 5 * 1024 * 1024 + 1];
    let (status, body) = post_multipart_with_cookie(
        pool,
        "/api/admin/media",
        "file",
        "big.png",
        "image/png",
        oversized,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["message"], "too_large");
}

#[sqlx::test]
async fn upload_then_get_round_trips_the_same_bytes(pool: PgPool) {
    let cookie = login(&pool).await;
    let (status, body) = post_multipart_with_cookie(
        pool.clone(),
        "/api/admin/media",
        "file",
        "photo.png",
        "image/png",
        TINY_PNG.to_vec(),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "upload failed: {body:?}");
    let url = body["url"].as_str().unwrap();
    assert!(url.ends_with(".png"));
    assert!(url.contains("/api/media/"));

    let path = url.rsplit_once("/api/media/").unwrap().1;
    let (status, headers, bytes) =
        get_raw_with_cookie(pool, &format!("/api/media/{path}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, TINY_PNG);
    assert_eq!(headers.get("content-type").unwrap(), "image/png");
}

#[sqlx::test]
async fn get_media_404s_for_an_unknown_key(pool: PgPool) {
    let (status, _headers, _bytes) =
        get_raw_with_cookie(pool, "/api/media/does-not-exist.png", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
