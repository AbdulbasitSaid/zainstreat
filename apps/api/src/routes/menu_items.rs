use crate::error::AppError;
use axum::{
    extract::{Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct PriceOptionResponse {
    pub id: i64,
    pub label: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
}

struct PriceOptionRow {
    id: i64,
    menu_item_id: i64,
    label: String,
    price: Decimal,
}

#[derive(Debug, Serialize)]
pub struct MenuItemResponse {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub is_featured: bool,
    pub price_options: Vec<PriceOptionResponse>,
}

struct MenuItemRow {
    id: i64,
    category_id: i64,
    name: String,
    description: Option<String>,
    price: Option<Decimal>,
    image_url: Option<String>,
    is_available: bool,
    is_featured: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListMenuItemsQuery {
    pub category_id: Option<i64>,
}

/// Returns all non-deleted items, including unavailable ones — see
/// requirement.md Decision 5. Does NOT filter on is_available.
///
/// Price options are fetched in one extra query across the entire table
/// (not scoped to `category_id`) and grouped in Rust — the table has
/// ~20 rows total, so a second filtered query or a SQL-side `json_agg`
/// isn't worth the complexity here.
pub async fn list_menu_items(
    State(pool): State<PgPool>,
    Query(params): Query<ListMenuItemsQuery>,
) -> Result<Json<Vec<MenuItemResponse>>, AppError> {
    let items = sqlx::query_as!(
        MenuItemRow,
        r#"
        SELECT id, category_id, name, description, price, image_url,
               is_available, is_featured
        FROM menu_items
        WHERE deleted_at IS NULL
          AND ($1::BIGINT IS NULL OR category_id = $1)
        ORDER BY display_order, id
        "#,
        params.category_id
    )
    .fetch_all(&pool)
    .await?;

    let option_rows = sqlx::query_as!(
        PriceOptionRow,
        r#"
        SELECT id, menu_item_id, label, price
        FROM menu_item_price_options
        WHERE deleted_at IS NULL
        ORDER BY menu_item_id, display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Relies on the ORDER BY above: each group's push() calls already
    // arrive in display order, so no secondary sort is needed here.
    let mut options_by_item: HashMap<i64, Vec<PriceOptionResponse>> = HashMap::new();
    for row in option_rows {
        options_by_item
            .entry(row.menu_item_id)
            .or_default()
            .push(PriceOptionResponse {
                id: row.id,
                label: row.label,
                price: row.price,
            });
    }

    let response = items
        .into_iter()
        .map(|item| MenuItemResponse {
            price_options: options_by_item.remove(&item.id).unwrap_or_default(),
            id: item.id,
            category_id: item.category_id,
            name: item.name,
            description: item.description,
            price: item.price,
            image_url: item.image_url,
            is_available: item.is_available,
            is_featured: item.is_featured,
        })
        .collect();

    Ok(Json(response))
}
