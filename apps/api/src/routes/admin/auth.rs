use crate::error::AppError;
use crate::rate_limit::client_ip_key;
use crate::routes::admin::SESSION_USER_ID_KEY;
use argon2::Argon2;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::phc::PasswordHash;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_sessions::Session;

// A real argon2id PHC-format hash generated once against an arbitrary fixed
// password (NOT a real account's password) — verified against on an unknown
// email so "no such user" and "wrong password" take a similar amount of time
// (requirement.md Decision 6).
/// Phase 33 Decision 3: this many failures for one email inside the window
/// locks that email out until the oldest failure ages out of the window.
const LOCKOUT_FAILURES: i64 = 10;
const LOCKOUT_WINDOW_MINUTES: i32 = 15;

const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$pP8y4uKEEF1EKXwXfGJoLg$bI+rGRSb+HQmvycvyHuHX3MYty55Ws2CxQeMRp/ZK4s";

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: String,
}

struct UserRow {
    id: i64,
    name: String,
    email: String,
    password_hash: String,
    role: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    session: Session,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AdminUserResponse>, AppError> {
    if payload.email.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::InvalidCredentials);
    }

    let recent_failures = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM login_failures
           WHERE lower(email) = lower($1)
             AND created_at > now() - make_interval(mins => $2)"#,
        payload.email,
        LOCKOUT_WINDOW_MINUTES,
    )
    .fetch_one(&pool)
    .await?;
    let locked_out = recent_failures >= LOCKOUT_FAILURES;

    let user = sqlx::query_as!(
        UserRow,
        "SELECT id, name, email, password_hash, role FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&pool)
    .await?;

    let hash_to_check = user
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_PASSWORD_HASH);

    // A locked-out email still pays for a full verify, so the response time
    // and body don't reveal the lockout.
    let parsed_hash = PasswordHash::new(hash_to_check).map_err(|_| AppError::InvalidCredentials)?;
    let verified = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok()
        && !locked_out;

    let Some(user) = user.filter(|_| verified) else {
        sqlx::query!(
            "INSERT INTO login_failures (email) VALUES ($1)",
            payload.email
        )
        .execute(&pool)
        .await?;
        // Never the email or password (Phase 33 Decision 10).
        tracing::warn!(ip = %client_ip_key(&headers), locked_out, "admin login failed");
        return Err(AppError::InvalidCredentials);
    };

    sqlx::query!(
        "DELETE FROM login_failures WHERE lower(email) = lower($1)",
        payload.email
    )
    .execute(&pool)
    .await?;

    session.cycle_id().await?; // regenerate session id on login — fixation hardening
    session.insert(SESSION_USER_ID_KEY, user.id).await?;

    Ok(Json(AdminUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
    }))
}

pub async fn me(
    State(pool): State<PgPool>,
    session: Session,
) -> Result<Json<AdminUserResponse>, AppError> {
    let user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    let user_id = user_id.ok_or(AppError::Unauthorized)?;

    let user = sqlx::query_as!(
        UserRow,
        "SELECT id, name, email, password_hash, role FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(Json(AdminUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
    }))
}

pub async fn logout(session: Session) -> Result<StatusCode, AppError> {
    session.delete().await?;
    Ok(StatusCode::NO_CONTENT)
}
