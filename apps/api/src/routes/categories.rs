use crate::error::AppError;
use crate::translation;
use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LocaleQuery {
    /// Phase 38 Decision 7: `en`, `nl` or `fr`; anything else is English.
    pub locale: Option<String>,
}

pub async fn list_categories(
    State(pool): State<PgPool>,
    Query(params): Query<LocaleQuery>,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let locale = translation::query_locale(params.locale.as_deref());
    let rows = sqlx::query!(
        r#"
        SELECT id, name, description, translations FROM categories
        WHERE deleted_at IS NULL AND published_at IS NOT NULL
        ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| CategoryResponse {
                id: row.id,
                name: translation::resolve(&row.translations, locale, "name", &row.name),
                description: translation::resolve_opt(&row.translations, locale, "description", row.description.as_deref()),
            })
            .collect(),
    ))
}
