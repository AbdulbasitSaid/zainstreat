use crate::error::AppError;
use crate::routes::admin::drafts::{load_site_content, load_site_content_for_update};
use crate::site_content::SiteContent;
use crate::MediaConfig;
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct AdminSiteContentResponse {
    pub published: SiteContent,
    pub draft: Option<SiteContent>,
    pub draft_updated_at: Option<DateTime<Utc>>,
}

/// `GET /api/admin/site-content`.
pub async fn get_site_content(
    State(pool): State<PgPool>,
) -> Result<Json<AdminSiteContentResponse>, AppError> {
    let (published, draft, draft_updated_at) = load_site_content(&pool).await?;
    Ok(Json(AdminSiteContentResponse { published, draft, draft_updated_at }))
}

#[derive(Debug, Deserialize)]
pub struct SaveSiteContentDraftRequest {
    pub content: SiteContent,
    /// `null` for the first draft since the last publish.
    pub draft_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SaveSiteContentDraftResponse {
    pub draft: SiteContent,
    pub draft_updated_at: DateTime<Utc>,
}

/// `PUT /api/admin/site-content/draft` (Phase 25 Decisions 11–13, 18).
/// Validated like a menu draft; a stale `draft_updated_at` gives `409`.
pub async fn save_site_content_draft(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(payload): Json<SaveSiteContentDraftRequest>,
) -> Result<Json<SaveSiteContentDraftResponse>, AppError> {
    let content = payload.content.normalize();

    let mut tx = pool.begin().await?;
    let (published, draft, draft_updated_at) = load_site_content_for_update(&mut tx).await?;
    if draft_updated_at != payload.draft_updated_at {
        return Err(AppError::Conflict(json!({ "draft_updated_at": draft_updated_at })));
    }

    let mut previous = vec![&published];
    previous.extend(draft.as_ref());
    let errors = content.validate(&previous, &media.public_base_url);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let saved_at = sqlx::query_scalar!(
        r#"
        UPDATE site_content SET draft = $1, draft_updated_at = now(), updated_at = now()
        RETURNING draft_updated_at as "draft_updated_at!: DateTime<Utc>"
        "#,
        serde_json::to_value(&content).expect("serializable"),
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(SaveSiteContentDraftResponse { draft: content, draft_updated_at: saved_at }))
}
