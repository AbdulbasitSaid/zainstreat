use crate::error::{AppError, FieldError};
use crate::MediaConfig;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    Json,
};
use serde::Serialize;

const MAX_BYTES: usize = 5 * 1024 * 1024;

fn extension_for(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub url: String,
}

pub async fn upload(
    State(media): State<MediaConfig>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    // Single "file" field is the only thing this form ever sends.
    let Some(field) = multipart.next_field().await.map_err(|_| {
        AppError::Validation(vec![FieldError { field: "file".into(), message: "missing".into() }])
    })?
    else {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "missing".into(),
        }]));
    };

    let content_type = field.content_type().unwrap_or("").to_string();
    let Some(ext) = extension_for(&content_type) else {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "unsupported_type".into(),
        }]));
    };

    let bytes = field.bytes().await.map_err(|_| {
        AppError::Validation(vec![FieldError { field: "file".into(), message: "unreadable".into() }])
    })?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "too_large".into(),
        }]));
    }

    let key = format!("{}.{ext}", uuid::Uuid::new_v4());

    media
        .client
        .put_object()
        .bucket(&media.bucket)
        .key(&key)
        .content_type(content_type)
        .body(bytes.into())
        .send()
        .await
        .map_err(|err| {
            tracing::error!(%err, %key, "upload to MinIO failed");
            AppError::Internal
        })?;

    Ok(Json(UploadResponse {
        url: format!("{}/api/media/{key}", media.public_base_url),
    }))
}

pub fn upload_router() -> axum::routing::MethodRouter<crate::AppState> {
    axum::routing::post(upload).layer(DefaultBodyLimit::max(6 * 1024 * 1024))
}
