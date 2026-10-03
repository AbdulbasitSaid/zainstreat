use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct AppError(sqlx::Error);

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "not_found"),
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                Some("23505") | Some("23503") => (StatusCode::CONFLICT, "conflict"),
                Some("23514") => (StatusCode::BAD_REQUEST, "bad_request"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error"),
            },
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error"),
        };

        if status.is_server_error() {
            tracing::error!(error = ?self.0, %status, "request failed");
        } else {
            tracing::warn!(error = ?self.0, %status, "request failed");
        }

        (status, Json(json!({ "error": code }))).into_response()
    }
}
