use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

pub async fn health(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query!("SELECT 1 AS one").fetch_one(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "db_unreachable" })),
        ),
    }
}
