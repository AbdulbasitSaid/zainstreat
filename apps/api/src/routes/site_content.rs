use crate::error::AppError;
use crate::routes::admin::drafts::load_site_content;
use crate::site_content::SiteContent;
use axum::{extract::State, Json};
use sqlx::PgPool;

/// `GET /api/site-content`: the published document only, never the draft
/// (Phase 25 Decision 17). The web caches it under a tag and revalidates
/// on publish, so no cache headers here.
pub async fn get_site_content(State(pool): State<PgPool>) -> Result<Json<SiteContent>, AppError> {
    let (published, _draft, _) = load_site_content(&pool).await?;
    Ok(Json(published))
}
