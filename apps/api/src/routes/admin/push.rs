//! Admin push subscriptions (Phase 27 Decisions 9, 10, 16).

use crate::error::{AppError, FieldError};
use crate::push::{self, PushConfig};
use crate::routes::admin::SESSION_USER_ID_KEY;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::USER_AGENT},
    response::{IntoResponse, Response},
};
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tower_sessions::Session;

const ENDPOINT_MAX_CHARS: usize = 1000;
const KEY_MAX_CHARS: usize = 200;
const USER_AGENT_MAX_CHARS: usize = 500;

#[derive(Deserialize)]
pub struct SubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

/// `PushSubscription.toJSON()` as the browser sends it.
#[derive(Deserialize)]
pub struct SubscribeRequest {
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Serialize)]
pub struct SubscriptionResponse {
    pub id: i64,
    pub endpoint: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
}

async fn current_user_id(session: &Session) -> Result<i64, AppError> {
    let user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    user_id.ok_or(AppError::Unauthorized)
}

fn not_configured() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "push_not_configured" }))).into_response()
}

pub async fn get_public_key(State(push): State<PushConfig>) -> Response {
    match push.public_key() {
        Some(key) => Json(json!({ "public_key": key })).into_response(),
        None => not_configured(),
    }
}

pub async fn list_subscriptions(
    State(pool): State<PgPool>,
    session: Session,
) -> Result<Json<Vec<SubscriptionResponse>>, AppError> {
    let user_id = current_user_id(&session).await?;
    let rows = sqlx::query!(
        r#"
        SELECT id, endpoint, user_agent,
               created_at as "created_at: DateTime<Utc>",
               last_success_at as "last_success_at: DateTime<Utc>",
               last_failure_at as "last_failure_at: DateTime<Utc>"
        FROM push_subscriptions
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id,
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| SubscriptionResponse {
                id: row.id,
                // The web marks "This device" by comparing endpoints; it's
                // only ever returned to the admin who owns it.
                endpoint: row.endpoint,
                label: device_label(row.user_agent.as_deref().unwrap_or("")),
                created_at: row.created_at,
                last_success_at: row.last_success_at,
                last_failure_at: row.last_failure_at,
            })
            .collect(),
    ))
}

fn validate(payload: &SubscribeRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();
    let mut error = |field: &str, message: &str| {
        errors.push(FieldError {
            field: field.to_string(),
            message: message.to_string(),
        })
    };
    let endpoint = payload.endpoint.trim();
    if !endpoint.starts_with("https://") || endpoint.chars().count() > ENDPOINT_MAX_CHARS {
        error("endpoint", "must be an https URL of at most 1000 characters");
    }
    let base64url = |value: &str| {
        !value.is_empty()
            && value.len() <= KEY_MAX_CHARS
            && Base64UrlUnpadded::decode_vec(value.trim_end_matches('=')).is_ok()
    };
    if !base64url(&payload.keys.p256dh) {
        error("keys.p256dh", "must be base64url");
    }
    if !base64url(&payload.keys.auth) {
        error("keys.auth", "must be base64url");
    }
    errors
}

/// Upserts on `endpoint`: re-subscribing on the same device never makes a
/// duplicate, and moves the row to whoever is signed in (Decision 10).
pub async fn subscribe(
    State(pool): State<PgPool>,
    session: Session,
    headers: HeaderMap,
    Json(payload): Json<SubscribeRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = current_user_id(&session).await?;
    let errors = validate(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|ua| ua.chars().take(USER_AGENT_MAX_CHARS).collect::<String>());

    let inserted = sqlx::query_scalar!(
        r#"
        INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (endpoint) DO UPDATE
        SET user_id = EXCLUDED.user_id, p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth,
            user_agent = EXCLUDED.user_agent, failure_count = 0
        RETURNING (xmax = 0) as "inserted!"
        "#,
        user_id,
        payload.endpoint.trim(),
        payload.keys.p256dh.trim_end_matches('='),
        payload.keys.auth.trim_end_matches('='),
        user_agent,
    )
    .fetch_one(&pool)
    .await?;

    Ok(if inserted { StatusCode::CREATED } else { StatusCode::OK })
}

pub async fn delete_subscription(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user_id = current_user_id(&session).await?;
    let deleted = sqlx::query!(
        "DELETE FROM push_subscriptions WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .execute(&pool)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn send_test(
    State(pool): State<PgPool>,
    State(push): State<PushConfig>,
    session: Session,
) -> Result<Response, AppError> {
    let user_id = current_user_id(&session).await?;
    if !push.is_enabled() {
        return Ok(not_configured());
    }
    push.spawn_notify_user(&pool, user_id, push::test_notification());
    Ok(StatusCode::ACCEPTED.into_response())
}

/// "iPhone · Safari", "Mac · Chrome" and so on, for the Devices list.
pub fn device_label(user_agent: &str) -> String {
    let device = if user_agent.contains("iPhone") {
        Some("iPhone")
    } else if user_agent.contains("iPad") {
        Some("iPad")
    } else if user_agent.contains("Android") {
        Some("Android")
    } else if user_agent.contains("Macintosh") || user_agent.contains("Mac OS X") {
        Some("Mac")
    } else if user_agent.contains("Windows") {
        Some("Windows")
    } else if user_agent.contains("Linux") || user_agent.contains("CrOS") {
        Some("Linux")
    } else {
        None
    };
    // Order matters: Edge and Chrome UAs also say "Safari", Edge also says
    // "Chrome", and Chrome on iOS says "CriOS".
    let browser = if user_agent.contains("Edg") {
        Some("Edge")
    } else if user_agent.contains("Firefox") || user_agent.contains("FxiOS") {
        Some("Firefox")
    } else if user_agent.contains("Chrome") || user_agent.contains("CriOS") {
        Some("Chrome")
    } else if user_agent.contains("Safari") {
        Some("Safari")
    } else {
        None
    };
    match (device, browser) {
        (Some(device), Some(browser)) => format!("{device} · {browser}"),
        (Some(only), None) | (None, Some(only)) => only.to_string(),
        (None, None) => "Unknown device".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::device_label;

    #[test]
    fn labels_common_user_agents() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";
        let android = "Mozilla/5.0 (Linux; Android 15; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Mobile Safari/537.36";
        let mac_chrome = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
        let windows_edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36 Edg/142.0.0.0";
        assert_eq!(device_label(iphone), "iPhone · Safari");
        assert_eq!(device_label(android), "Android · Chrome");
        assert_eq!(device_label(mac_chrome), "Mac · Chrome");
        assert_eq!(device_label(windows_edge), "Windows · Edge");
        assert_eq!(device_label(""), "Unknown device");
    }
}
