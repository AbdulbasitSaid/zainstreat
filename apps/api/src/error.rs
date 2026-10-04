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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BlockingMenuItem {
    pub id: i64,
    pub name: String,
}

pub enum AppError {
    Database(sqlx::Error),
    Validation(Vec<FieldError>),
    ItemsUnavailable(Vec<UnavailableItem>),
    Unauthorized,
    InvalidCredentials,
    Session(tower_sessions::session::Error),
    NotFound,
    CategoryHasActiveItems(Vec<BlockingMenuItem>),
    CategoryArchived,
    Conflict(serde_json::Value),
    Internal,
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        // requirement.md Decision 7's addendum (open risk 5) — Group 1a's
        // trigger raises this specific check_violation when a menu item is
        // inserted/re-pointed under an archived category under a race the
        // application-level pre-check in `validate_menu_item` can't close
        // alone. Matched on message, not just the 23514 code, so some other
        // unrelated check_violation doesn't get swallowed under this shape.
        if let sqlx::Error::Database(db_err) = &err
            && db_err.code().as_deref() == Some("23514")
            && db_err.message().contains("menu item category")
        {
            return Self::CategoryArchived;
        }
        Self::Database(err)
    }
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(err: tower_sessions::session::Error) -> Self {
        Self::Session(err)
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
            AppError::Unauthorized => {
                tracing::warn!("rejected request with no valid admin session");
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": "unauthorized" }))).into_response()
            }
            AppError::InvalidCredentials => {
                tracing::warn!("admin login failed");
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "invalid_credentials" })),
                )
                    .into_response()
            }
            AppError::Session(err) => {
                tracing::error!(error = ?err, "session store error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal_server_error" })),
                )
                    .into_response()
            }
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" }))).into_response()
            }
            AppError::CategoryHasActiveItems(items) => {
                tracing::warn!(?items, "category archive rejected: active menu items remain");
                (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "category_has_active_items", "items": items })),
                )
                    .into_response()
            }
            AppError::CategoryArchived => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "validation_error",
                    "fields": [{ "field": "category_id", "message": "invalid" }],
                })),
            )
                .into_response(),
            AppError::Conflict(current) => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "conflict", "current": current })),
            )
                .into_response(),
            AppError::Internal => {
                tracing::error!("internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal_server_error" })),
                )
                    .into_response()
            }
        }
    }
}
