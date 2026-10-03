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
        eprintln!("database error: {:?}", self.0); // no tracing crate yet
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "internal_server_error" })),
        )
            .into_response()
    }
}
