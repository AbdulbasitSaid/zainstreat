use crate::error::AppError;
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let categories = sqlx::query_as!(
        CategoryResponse,
        r#"SELECT id, name, description FROM categories WHERE deleted_at IS NULL ORDER BY display_order, id"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(categories))
}
