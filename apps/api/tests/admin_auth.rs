mod common;

use argon2::password_hash::PasswordHasher;
use argon2::Argon2;
use axum::http::StatusCode;
use common::{get_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

async fn insert_admin(pool: &PgPool, email: &str, password: &str) {
    let hash = Argon2::default().hash_password(password.as_bytes()).unwrap().to_string();
    sqlx::query!(
        "INSERT INTO users (name, email, password_hash, role) VALUES ($1, $2, $3, 'admin')",
        "Test Admin",
        email,
        hash,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn login_succeeds_and_me_reflects_session(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct horse battery staple").await;

    let (status, body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email"], "owner@example.com");
    let cookie = session_cookie(&set_cookie.expect("login should set a session cookie")).to_string();

    let (me_status, me_body) = get_with_cookie(pool, "/api/admin/me", Some(&cookie)).await;
    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me_body["email"], "owner@example.com");
}

#[sqlx::test]
async fn me_without_session_is_unauthorized(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/me", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn login_rejects_wrong_password(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct-password").await;

    let (status, body, set_cookie) = post_with_cookie(
        pool,
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "wrong-password" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
    assert!(set_cookie.is_none());
}

#[sqlx::test]
async fn login_rejects_unknown_email(pool: PgPool) {
    let (status, body, set_cookie) = post_with_cookie(
        pool,
        "/api/admin/login",
        json!({ "email": "nobody@example.com", "password": "anything" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
    assert!(set_cookie.is_none());
}

#[sqlx::test]
async fn logout_invalidates_session(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct horse battery staple").await;

    let (_status, _body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    let cookie = session_cookie(&set_cookie.unwrap()).to_string();

    let (logout_status, _body, _set_cookie) =
        post_with_cookie(pool.clone(), "/api/admin/logout", json!({}), Some(&cookie)).await;
    assert_eq!(logout_status, StatusCode::NO_CONTENT);

    let (me_status, _me_body) = get_with_cookie(pool, "/api/admin/me", Some(&cookie)).await;
    assert_eq!(me_status, StatusCode::UNAUTHORIZED);
}
