mod common;

use axum::http::{StatusCode, header};
use common::{
    get_raw_with_cookie, get_with_cookie, insert_admin, post_with_cookie, session_cookie,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const EMAIL: &str = "Ada@Example.com";

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

async fn insert_order(pool: &PgPool, email: &str, status: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type, subtotal, total, status)
        VALUES ('Ada', $1, '+31612345678', 'pickup', 12.50, 12.50, $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn insert_contact_message(pool: &PgPool, email: &str) {
    sqlx::query(
        "INSERT INTO contact_messages (name, email, phone, subject, message) VALUES ('Ada', $1, '0', 'general', 'hi')",
    )
    .bind(email)
    .execute(pool)
    .await
    .unwrap();
}

async fn request_count(pool: &PgPool) -> (i64, Option<String>) {
    sqlx::query_as("SELECT count(*), max(email_sha256) FROM privacy_requests")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn privacy_routes_need_an_admin(pool: PgPool) {
    let (status, _body) =
        get_with_cookie(pool, "/api/admin/privacy?email=a@example.com", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn search_lists_only_that_email_case_insensitively(pool: PgPool) {
    let cookie = login(&pool).await;
    let mine = insert_order(&pool, "ada@example.com", "completed").await;
    insert_order(&pool, "someone@else.com", "completed").await;
    insert_contact_message(&pool, "ADA@example.com").await;

    let (status, body) = get_with_cookie(
        pool,
        &format!("/api/admin/privacy?email={EMAIL}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["id"], mine);
    assert_eq!(body["contact_messages"].as_array().unwrap().len(), 1);
    assert_eq!(body["blocking"]["orders"], json!([]));
}

#[sqlx::test]
async fn export_downloads_the_rows_and_logs_a_hash(pool: PgPool) {
    let cookie = login(&pool).await;
    insert_order(&pool, "ada@example.com", "completed").await;
    insert_order(&pool, "someone@else.com", "completed").await;

    let (status, headers, bytes) = get_raw_with_cookie(
        pool.clone(),
        &format!("/api/admin/privacy/export?email={EMAIL}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment")
    );
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["customer_email"], "ada@example.com");
    assert!(body["orders"][0].get("public_token").is_none());

    let (count, hash) = request_count(&pool).await;
    assert_eq!(count, 1);
    let hash = hash.unwrap();
    assert_eq!(hash.len(), 64);
    assert!(!hash.contains('@'));
}

#[sqlx::test]
async fn anonymise_refuses_a_mismatched_confirmation(pool: PgPool) {
    let cookie = login(&pool).await;
    insert_order(&pool, "ada@example.com", "completed").await;

    let (status, body, _cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/privacy/anonymise",
        json!({ "email": EMAIL, "confirm_email": "ada@example.org" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(request_count(&pool).await.0, 0);
}

#[sqlx::test]
async fn anonymise_is_blocked_by_an_open_order(pool: PgPool) {
    let cookie = login(&pool).await;
    let open = insert_order(&pool, "ada@example.com", "processing").await;

    let (status, body, _cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/privacy/anonymise",
        json!({ "email": EMAIL, "confirm_email": "ada@example.com" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["current"]["orders"], json!([open]));
    assert_eq!(request_count(&pool).await.0, 0);
}

#[sqlx::test]
async fn anonymise_erases_the_address(pool: PgPool) {
    let cookie = login(&pool).await;
    let order = insert_order(&pool, "ada@example.com", "completed").await;
    let other = insert_order(&pool, "someone@else.com", "completed").await;
    insert_contact_message(&pool, "ada@example.com").await;

    let (status, body, _cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/privacy/anonymise",
        json!({ "email": EMAIL, "confirm_email": "ada@example.com " }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["orders"], 1);
    assert_eq!(body["contact_messages"], 1);

    let names: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id, customer_name, total::text FROM orders ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        names,
        vec![
            (order, "Anonymised".to_string(), "12.50".to_string()),
            (other, "Ada".to_string(), "12.50".to_string()),
        ]
    );

    // Nothing left to find, and one logged request.
    let (_status, body) = get_with_cookie(
        pool.clone(),
        &format!("/api/admin/privacy?email={EMAIL}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(body["orders"], json!([]));
    assert_eq!(body["contact_messages"], json!([]));
    assert_eq!(request_count(&pool).await.0, 1);
}
