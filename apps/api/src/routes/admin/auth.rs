use crate::error::AppError;
use crate::routes::admin::SESSION_USER_ID_KEY;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::PasswordVerifier;
use argon2::Argon2;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_sessions::Session;

// A real argon2id PHC-format hash generated once against an arbitrary fixed
// password (NOT a real account's password) — verified against on an unknown
// email so "no such user" and "wrong password" take a similar amount of time
// (requirement.md Decision 6).
const DUMMY_PASSWORD_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$pP8y4uKEEF1EKXwXfGJoLg$bI+rGRSb+HQmvycvyHuHX3MYty55Ws2CxQeMRp/ZK4s";

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
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AdminUserResponse>, AppError> {
    if payload.email.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::InvalidCredentials);
    }

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

    let parsed_hash =
        PasswordHash::new(hash_to_check).map_err(|_| AppError::InvalidCredentials)?;
    let verified = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok();

    let Some(user) = user.filter(|_| verified) else {
        return Err(AppError::InvalidCredentials);
    };

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
