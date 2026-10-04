use api::build_app;
use argon2::password_hash::PasswordHasher;
use argon2::Argon2;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(pool: PgPool, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let app = build_app(pool, false)
        .await
        .expect("failed to build app in test");
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, body)
}

#[allow(dead_code)]
pub async fn get(pool: PgPool, uri: &str) -> (StatusCode, Value) {
    let (status, _headers, body) =
        send(pool, Request::builder().uri(uri).body(Body::empty()).unwrap()).await;
    (status, body)
}

#[allow(dead_code)]
pub async fn post(pool: PgPool, uri: &str, body: Value) -> (StatusCode, Value) {
    let (status, _headers, resp_body) = send(
        pool,
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;
    (status, resp_body)
}

/// For auth flows: returns the response body plus any `Set-Cookie` value so
/// a follow-up request can carry the session forward.
#[allow(dead_code)]
pub async fn post_with_cookie(
    pool: PgPool,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, headers, resp_body) =
        send(pool, builder.body(Body::from(body.to_string())).unwrap()).await;
    let set_cookie = headers
        .get("set-cookie")
        .map(|v| v.to_str().unwrap().to_string());
    (status, resp_body, set_cookie)
}

#[allow(dead_code)]
pub async fn get_with_cookie(pool: PgPool, uri: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, body) = send(pool, builder.body(Body::empty()).unwrap()).await;
    (status, body)
}

#[allow(dead_code)]
pub async fn patch_with_cookie(
    pool: PgPool,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, resp_body) =
        send(pool, builder.body(Body::from(body.to_string())).unwrap()).await;
    (status, resp_body)
}

/// A `Set-Cookie` value looks like `id=...; HttpOnly; SameSite=Lax; ...` —
/// only the `id=...` part before the first `;` is sent back as `Cookie`.
#[allow(dead_code)]
pub fn session_cookie(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap()
}

#[allow(dead_code)]
pub async fn insert_admin(pool: &PgPool, email: &str, password: &str) {
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
