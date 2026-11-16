mod common;

use api::push::{PushConfig, encode_private_key, encode_public_key};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    routing::post,
};
use base64ct::{Base64UrlUnpadded, Encoding};
use common::{build_test_app_with_push, insert_admin, session_cookie};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU16, Ordering},
};
use std::time::Duration;
use tower::ServiceExt;
use web_push_native::jwt_simple::algorithms::ES256KeyPair;

const PASSWORD: &str = "correct horse battery staple";
// A P-256 point and 16-byte secret in the shape browsers send.
const P256DH: &str = "BLn9b-VR0ca83knDNZ32dCHGyjJp-1riX9ZTN40MqV8K_LpQmLqxC_DoHvqvFXO_nGdAB4W9dogZb_sM-uV4JbY";
const AUTH: &str = "_ordMnz7uTCmrpBTeUV4Bw";

fn enabled_push() -> PushConfig {
    let key_pair = ES256KeyPair::generate();
    PushConfig::from_keys(
        &encode_public_key(&key_pair),
        &encode_private_key(&key_pair),
        "mailto:contact@zainstreat.com",
    )
    .unwrap()
}

async fn request(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header(
            "user-agent",
            "Mozilla/5.0 (Linux; Android 15; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Mobile Safari/537.36",
        );
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let body = body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty);
    let response = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let set_cookie = response
        .headers()
        .get("set-cookie")
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, value, set_cookie)
}

async fn login(app: &Router, pool: &PgPool, email: &str) -> String {
    insert_admin(pool, email, PASSWORD).await;
    let (status, _, set_cookie) = request(
        app,
        "POST",
        "/api/admin/login",
        Some(json!({ "email": email, "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    session_cookie(&set_cookie.unwrap()).to_string()
}

fn subscription(endpoint: &str) -> Value {
    json!({ "endpoint": endpoint, "keys": { "p256dh": P256DH, "auth": AUTH } })
}

#[sqlx::test]
async fn every_endpoint_needs_a_session(pool: PgPool) {
    let app = build_test_app_with_push(pool, enabled_push()).await;
    for (method, uri) in [
        ("GET", "/api/admin/push/public-key"),
        ("GET", "/api/admin/push/subscriptions"),
        ("POST", "/api/admin/push/subscriptions"),
        ("DELETE", "/api/admin/push/subscriptions/1"),
        ("POST", "/api/admin/push/test"),
    ] {
        let body = (method == "POST").then(|| subscription("https://push.example/a"));
        let (status, _, _) = request(&app, method, uri, body, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[sqlx::test]
async fn public_key_when_configured_and_not(pool: PgPool) {
    let push = enabled_push();
    let expected = push.public_key().unwrap().to_string();
    let app = build_test_app_with_push(pool.clone(), push).await;
    let cookie = login(&app, &pool, "owner@example.com").await;
    let (status, body, _) = request(&app, "GET", "/api/admin/push/public-key", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["public_key"], expected);

    let app = build_test_app_with_push(pool.clone(), PushConfig::disabled()).await;
    let (status, body, _) = request(&app, "GET", "/api/admin/push/public-key", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "push_not_configured");
    let (status, body, _) = request(&app, "POST", "/api/admin/push/test", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "push_not_configured");
}

#[sqlx::test]
async fn subscribe_list_resubscribe_and_delete(pool: PgPool) {
    let app = build_test_app_with_push(pool.clone(), PushConfig::disabled()).await;
    let cookie = login(&app, &pool, "owner@example.com").await;
    let endpoint = "https://fcm.googleapis.com/fcm/send/abc";

    let (status, _, _) =
        request(&app, "POST", "/api/admin/push/subscriptions", Some(subscription(endpoint)), Some(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, _) =
        request(&app, "POST", "/api/admin/push/subscriptions", Some(subscription(endpoint)), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK, "same endpoint again updates, never duplicates");

    let (status, list, _) = request(&app, "GET", "/api/admin/push/subscriptions", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["endpoint"], endpoint);
    assert_eq!(list[0]["label"], "Android · Chrome");
    assert!(list[0]["last_success_at"].is_null());

    let id = list[0]["id"].as_i64().unwrap();
    let (status, _, _) =
        request(&app, "DELETE", &format!("/api/admin/push/subscriptions/{id}"), None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list, _) = request(&app, "GET", "/api/admin/push/subscriptions", None, Some(&cookie)).await;
    assert_eq!(list.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn another_admins_device_is_not_theirs_to_delete(pool: PgPool) {
    let app = build_test_app_with_push(pool.clone(), PushConfig::disabled()).await;
    let owner = login(&app, &pool, "owner@example.com").await;
    let other = login(&app, &pool, "staff@example.com").await;
    request(&app, "POST", "/api/admin/push/subscriptions", Some(subscription("https://push.example/o")), Some(&owner))
        .await;
    let id: i64 = sqlx::query_scalar!("SELECT id FROM push_subscriptions").fetch_one(&pool).await.unwrap();

    let (status, _, _) =
        request(&app, "DELETE", &format!("/api/admin/push/subscriptions/{id}"), None, Some(&other)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list, _) = request(&app, "GET", "/api/admin/push/subscriptions", None, Some(&other)).await;
    assert_eq!(list.as_array().unwrap().len(), 0, "only your own devices are listed");
}

#[sqlx::test]
async fn invalid_subscriptions_are_rejected(pool: PgPool) {
    let app = build_test_app_with_push(pool.clone(), PushConfig::disabled()).await;
    let cookie = login(&app, &pool, "owner@example.com").await;
    let too_long = format!("https://push.example/{}", "a".repeat(1000));
    for (body, field) in [
        (subscription("http://push.example/a"), "endpoint"),
        (subscription(&too_long), "endpoint"),
        (json!({ "endpoint": "https://push.example/a", "keys": { "p256dh": "not base64!", "auth": AUTH } }), "keys.p256dh"),
        (json!({ "endpoint": "https://push.example/a", "keys": { "p256dh": P256DH, "auth": "" } }), "keys.auth"),
    ] {
        let (status, response, _) =
            request(&app, "POST", "/api/admin/push/subscriptions", Some(body), Some(&cookie)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response["fields"][0]["field"], field);
    }
}

#[derive(Clone)]
struct Recorder {
    requests: Arc<Mutex<Vec<(HeaderMap, Bytes)>>>,
    status: Arc<AtomicU16>,
}

async fn record(State(recorder): State<Recorder>, headers: HeaderMap, body: Bytes) -> StatusCode {
    recorder.requests.lock().unwrap().push((headers, body));
    StatusCode::from_u16(recorder.status.load(Ordering::SeqCst)).unwrap()
}

/// A stand-in push service on 127.0.0.1 that records what it receives.
async fn fake_push_service(status: u16) -> (String, Recorder) {
    let recorder = Recorder {
        requests: Arc::default(),
        status: Arc::new(AtomicU16::new(status)),
    };
    let app = Router::new().route("/push", post(record)).with_state(recorder.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{address}/push"), recorder)
}

async fn wait_for<F: Fn() -> bool>(condition: F) {
    for _ in 0..100 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for the push to arrive");
}

/// Inserted directly: the API only accepts https endpoints, and the fake
/// push service is plain http.
async fn insert_subscription(pool: &PgPool, endpoint: &str, p256dh: &str, auth: &str) -> i64 {
    let user_id: i64 = sqlx::query_scalar!("SELECT id FROM users LIMIT 1").fetch_one(pool).await.unwrap();
    sqlx::query_scalar!(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth) VALUES ($1, $2, $3, $4) RETURNING id",
        user_id,
        endpoint,
        p256dh,
        auth,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn place_order(app: &Router, pool: &PgPool) -> i64 {
    sqlx::query!("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')").execute(pool).await.unwrap();
    sqlx::query!("INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)")
        .execute(pool)
        .await
        .unwrap();
    let (status, order, _) = request(
        app,
        "POST",
        "/api/orders",
        Some(json!({
            "customer_name": "Amina Bello",
            "customer_email": "amina@example.com",
            "customer_phone": "+31 6 1234 5678",
            "delivery_type": "pickup",
            "delivery_address": null,
            "notes": null,
            "items": [{ "menu_item_id": 1, "price_option_id": null, "quantity": 2 }],
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    order["id"].as_i64().unwrap()
}

#[sqlx::test]
async fn a_new_order_pushes_an_encrypted_notification(pool: PgPool) {
    let app = build_test_app_with_push(pool.clone(), enabled_push()).await;
    login(&app, &pool, "owner@example.com").await;
    let (endpoint, recorder) = fake_push_service(201).await;

    // The receiving browser's keys, so the test can decrypt the payload.
    let browser_key = ES256KeyPair::generate();
    let auth_secret = [7u8; 16];
    let subscription_id = insert_subscription(
        &pool,
        &endpoint,
        &encode_public_key(&browser_key),
        &Base64UrlUnpadded::encode_string(&auth_secret),
    )
    .await;

    let order_id = place_order(&app, &pool).await;
    wait_for(|| !recorder.requests.lock().unwrap().is_empty()).await;

    let (headers, body) = recorder.requests.lock().unwrap()[0].clone();
    assert_eq!(headers["content-encoding"], "aes128gcm");
    assert_eq!(headers["ttl"], "86400");
    assert_eq!(headers["urgency"], "high");
    assert!(headers["authorization"].to_str().unwrap().starts_with("vapid t="));

    let secret = web_push_native::p256::SecretKey::from_slice(&browser_key.to_bytes()).unwrap();
    let plain = web_push_native::decrypt(body.to_vec(), &secret, &auth_secret.into()).unwrap();
    let payload: Value = serde_json::from_slice(&plain).unwrap();
    assert_eq!(payload["title"], format!("New order ZTM-{order_id}"));
    assert_eq!(payload["body"], "€26.00 from Amina Bello · Pickup");
    assert_eq!(payload["url"], format!("/orders/{order_id}"));
    assert_eq!(payload["tag"], format!("order-{order_id}"));
    assert_eq!(payload["badge"], 1);

    // The outcome is recorded after the response, so give it a moment.
    for _ in 0..100 {
        let delivered: Option<bool> = sqlx::query_scalar!(
            "SELECT last_success_at IS NOT NULL FROM push_subscriptions WHERE id = $1",
            subscription_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if delivered == Some(true) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("last_success_at was never set");
}

#[sqlx::test]
async fn a_gone_subscription_is_deleted(pool: PgPool) {
    let app = build_test_app_with_push(pool.clone(), enabled_push()).await;
    login(&app, &pool, "owner@example.com").await;
    let (endpoint, recorder) = fake_push_service(410).await;
    insert_subscription(&pool, &endpoint, P256DH, AUTH).await;

    place_order(&app, &pool).await;
    wait_for(|| !recorder.requests.lock().unwrap().is_empty()).await;

    for _ in 0..100 {
        let remaining: Option<i64> =
            sqlx::query_scalar!("SELECT COUNT(*) FROM push_subscriptions").fetch_one(&pool).await.unwrap();
        if remaining == Some(0) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the 410 subscription was never deleted");
}
