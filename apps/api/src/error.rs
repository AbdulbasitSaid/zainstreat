use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UnavailableItem {
    pub menu_item_id: i64,
    pub price_option_id: Option<i64>,
    pub name: Option<String>,
    pub reason: String,
}

pub enum AppError {
    Database(sqlx::Error),
    Validation(Vec<FieldError>),
    ItemsUnavailable(Vec<UnavailableItem>),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(err) => {
                let (status, code) = match &err {
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
                    tracing::error!(error = ?err, %status, "request failed");
                } else {
                    tracing::warn!(error = ?err, %status, "request failed");
                }

                (status, Json(json!({ "error": code }))).into_response()
            }
            AppError::Validation(fields) => {
                tracing::warn!(?fields, "validation failed");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "validation_error", "fields": fields })),
                )
                    .into_response()
            }
            AppError::ItemsUnavailable(items) => {
                tracing::warn!(?items, "order rejected: items unavailable");
                (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "items_unavailable", "items": items })),
                )
                    .into_response()
            }
        }
    }
}
