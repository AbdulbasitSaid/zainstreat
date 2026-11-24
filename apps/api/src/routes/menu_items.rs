use crate::error::AppError;
use crate::order_emails::Locale;
use crate::translation::{self, LocalizedText};
use axum::{
    Json,
    extract::{Query, State},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct PriceOptionResponse {
    pub id: i64,
    /// In the requested locale (Phase 38 Decision 7).
    pub label: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    /// Every locale, so the cart can switch language (Decision 8).
    pub labels: LocalizedText,
}

impl PriceOptionResponse {
    pub fn localized(id: i64, label: &str, price: Decimal, translations: &Value, locale: Locale) -> Self {
        Self {
            id,
            label: translation::resolve(translations, locale, "label", label),
            price,
            labels: LocalizedText::of(translations, "label", label),
        }
    }
}

struct PriceOptionRow {
    id: i64,
    menu_item_id: i64,
    label: String,
    price: Decimal,
    translations: Value,
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
    /// Phase 36 Decision 2: the cart and `POST /api/orders` enforce it.
    pub min_quantity: i32,
    pub price_options: Vec<PriceOptionResponse>,
    /// The name in every locale (Phase 38 Decision 8).
    pub names: LocalizedText,
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
    min_quantity: i32,
    translations: Value,
}

#[derive(Debug, Deserialize)]
pub struct ListMenuItemsQuery {
    pub category_id: Option<i64>,
    /// Phase 38 Decision 7: `en`, `nl` or `fr`; anything else is English.
    pub locale: Option<String>,
}

/// Returns all non-deleted items, including unavailable ones — see
/// requirement.md Decision 5. Does NOT filter on is_available. Only
/// published items in published categories are listed, and drafts are
/// never read (Phase 25 Decision 17).
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
        SELECT mi.id, mi.category_id, mi.name, mi.description, mi.price, mi.image_url,
               mi.is_available, mi.is_featured, mi.min_quantity, mi.translations
        FROM menu_items mi
        JOIN categories c ON c.id = mi.category_id
        WHERE mi.deleted_at IS NULL
          AND mi.published_at IS NOT NULL
          AND c.published_at IS NOT NULL
          AND ($1::BIGINT IS NULL OR mi.category_id = $1)
        ORDER BY mi.display_order, mi.id
        "#,
        params.category_id
    )
    .fetch_all(&pool)
    .await?;

    let option_rows = sqlx::query_as!(
        PriceOptionRow,
        r#"
        SELECT id, menu_item_id, label, price, translations
        FROM menu_item_price_options
        WHERE deleted_at IS NULL
        ORDER BY menu_item_id, display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Relies on the ORDER BY above: each group's push() calls already
    // arrive in display order, so no secondary sort is needed here.
    let locale = translation::query_locale(params.locale.as_deref());
    let mut options_by_item: HashMap<i64, Vec<PriceOptionResponse>> = HashMap::new();
    for row in option_rows {
        options_by_item
            .entry(row.menu_item_id)
            .or_default()
            .push(PriceOptionResponse::localized(row.id, &row.label, row.price, &row.translations, locale));
    }

    let response = items
        .into_iter()
        .map(|item| MenuItemResponse {
            price_options: options_by_item.remove(&item.id).unwrap_or_default(),
            id: item.id,
            category_id: item.category_id,
            name: translation::resolve(&item.translations, locale, "name", &item.name),
            description: translation::resolve_opt(&item.translations, locale, "description", item.description.as_deref()),
            names: LocalizedText::of(&item.translations, "name", &item.name),
            price: item.price,
            image_url: item.image_url,
            is_available: item.is_available,
            is_featured: item.is_featured,
            min_quantity: item.min_quantity,
        })
        .collect();

    Ok(Json(response))
}
