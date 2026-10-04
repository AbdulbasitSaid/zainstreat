use api::{build_app, MediaConfig};
use argon2::password_hash::PasswordHasher;
use argon2::Argon2;
use aws_sdk_s3::config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region};
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// Every object key this test suite uploads is a fresh random UUID (same as
/// production — see admin/media.rs), so a single shared bucket across every
/// test run never collides; no per-test bucket is needed. MinIO itself is
/// not torn down/recreated between tests the way the `#[sqlx::test]`
/// database is, so this bucket is created idempotently (tolerating
/// "already owned by you") on every call.
async fn test_media_config() -> MediaConfig {
    let endpoint = std::env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".into());
    let access_key = std::env::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY must be set to run tests");
    let secret_key = std::env::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY must be set to run tests");
    let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "test-menu-images".into());
    let public_base_url =
        std::env::var("PUBLIC_API_URL").unwrap_or_else(|_| "http://localhost:8080".into());

    let s3_config = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url(endpoint)
        .credentials_provider(Credentials::new(access_key, secret_key, None, None, "minio-static"))
        .force_path_style(true)
        .build();
    let client = aws_sdk_s3::Client::from_conf(s3_config);

    match client.create_bucket().bucket(&bucket).send().await {
        Ok(_) => {}
        Err(err)
            if err
                .as_service_error()
                .is_some_and(|e| e.is_bucket_already_owned_by_you() || e.is_bucket_already_exists()) => {}
        Err(err) => panic!("failed to create test media bucket: {err}"),
    }

    MediaConfig { client, bucket, public_base_url }
}

async fn send_raw(pool: PgPool, request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let app = build_app(pool, false, test_media_config().await)
        .await
        .expect("failed to build app in test");
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, bytes)
}

async fn send(pool: PgPool, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let (status, headers, bytes) = send_raw(pool, request).await;
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, body)
}

/// For routes that don't return JSON (the public media `GET`, which streams
/// raw image bytes) — returns the response body untouched instead of trying
/// (and failing) to parse it as JSON.
#[allow(dead_code)]
pub async fn get_raw_with_cookie(
    pool: PgPool,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    send_raw(pool, builder.body(Body::empty()).unwrap()).await
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

#[allow(dead_code)]
pub async fn insert_category(pool: &PgPool, name: &str) -> i64 {
    sqlx::query_scalar!("INSERT INTO categories (name) VALUES ($1) RETURNING id", name)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[allow(dead_code)]
pub async fn post_multipart_with_cookie(
    pool: PgPool,
    uri: &str,
    field_name: &str,
    filename: &str,
    content_type: &str,
    bytes: Vec<u8>,
    cookie: Option<&str>,
) -> (StatusCode, Value) {
    let boundary = "x-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", format!("multipart/form-data; boundary={boundary}"));
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, resp_body) = send(pool, builder.body(Body::from(body)).unwrap()).await;
    (status, resp_body)
}
