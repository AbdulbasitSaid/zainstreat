use crate::error::AppError;
use axum::{
    extract::{Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MenuItemResponse {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub is_featured: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListMenuItemsQuery {
    pub category_id: Option<i64>,
}

/// Returns all non-deleted items, including unavailable ones — see
/// requirement.md Decision 5. Does NOT filter on is_available.
pub async fn list_menu_items(
    State(pool): State<PgPool>,
    Query(params): Query<ListMenuItemsQuery>,
) -> Result<Json<Vec<MenuItemResponse>>, AppError> {
    let items = sqlx::query_as!(
        MenuItemResponse,
        r#"
        SELECT id, category_id, name, description, price, image_url,
               is_available, is_featured
        FROM menu_items
        WHERE deleted_at IS NULL
          AND ($1::BIGINT IS NULL OR category_id = $1)
        ORDER BY id
        "#,
        params.category_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(items))
}
