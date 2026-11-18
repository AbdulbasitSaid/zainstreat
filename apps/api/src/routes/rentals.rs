//! Phase 36: the public rentals list (requirement.md Decision 10).
//! Rentals are reserved on WhatsApp, never ordered, so this is read-only.

use crate::error::AppError;
use axum::{Json, extract::State};
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
}

/// Available, non-archived rentals in the owner's order.
pub async fn list_rentals(State(pool): State<PgPool>) -> Result<Json<Vec<RentalResponse>>, AppError> {
    let rentals = sqlx::query_as!(
        RentalResponse,
        r#"
        SELECT id, name, description, price, image_url
        FROM rentals
        WHERE deleted_at IS NULL AND is_available
        ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rentals))
}
