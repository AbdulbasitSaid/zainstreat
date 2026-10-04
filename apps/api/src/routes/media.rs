use crate::error::AppError;
use crate::MediaConfig;
use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};

pub async fn get_media(
    State(media): State<MediaConfig>,
    Path(key): Path<String>,
) -> Result<Response, AppError> {
    let object = media
        .client
        .get_object()
        .bucket(&media.bucket)
        .key(&key)
        .send()
        .await
        .map_err(|err| {
            if err.as_service_error().is_some_and(|e| e.is_no_such_key()) {
                AppError::NotFound
            } else {
                tracing::error!(%err, %key, "media fetch from MinIO failed");
                AppError::Internal
            }
        })?;

    let content_type = object
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = object
        .body
        .collect()
        .await
        .map_err(|err| {
            tracing::error!(%err, %key, "failed to read media body from MinIO");
            AppError::Internal
        })
        .map(|data| data.into_bytes())?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        bytes,
    )
        .into_response())
}
