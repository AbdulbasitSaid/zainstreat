//! Phase 36: the public rentals list (requirement.md Decision 10), in the
//! requested locale since Phase 38 (Decision 7).

use crate::error::AppError;
use crate::routes::categories::LocaleQuery;
use crate::translation::{self, LocalizedText};
use axum::{
    Json,
    extract::{Query, State},
};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct RentalResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub image_url: Option<String>,
    /// The name in every locale, for the cart (Phase 38 Decision 8).
    pub names: LocalizedText,
}

/// Available, non-archived rentals in the owner's order.
pub async fn list_rentals(
    State(pool): State<PgPool>,
    Query(params): Query<LocaleQuery>,
) -> Result<Json<Vec<RentalResponse>>, AppError> {
    let locale = translation::query_locale(params.locale.as_deref());
    let rows = sqlx::query!(
        r#"
        SELECT id, name, description, price, image_url, translations
        FROM rentals
        WHERE deleted_at IS NULL AND is_available
        ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| RentalResponse {
                id: row.id,
                name: translation::resolve(&row.translations, locale, "name", &row.name),
                description: translation::resolve_opt(&row.translations, locale, "description", row.description.as_deref()),
                price: row.price,
                image_url: row.image_url,
                names: LocalizedText::of(&row.translations, "name", &row.name),
            })
            .collect(),
    ))
}
