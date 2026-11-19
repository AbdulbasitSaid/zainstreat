//! Phase 37: the public delivery calculator (requirement.md Decisions 9, 11).
//! Nothing here returns the business origin (Decision 7).

use crate::delivery::{self, DeliveryConfig, EstimatedBy, LookupDown, Suggestion};
use crate::error::{AppError, FieldError};
use axum::{
    Json,
    extract::{Query, State},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const QUERY_MIN_CHARS: usize = 3;
const QUERY_MAX_CHARS: usize = 100;

#[derive(Debug, Serialize)]
pub struct DeliveryInfoResponse {
    pub enabled: bool,
    #[serde(with = "rust_decimal::serde::str")]
    pub rate_per_km: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub minimum_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub max_distance_km: Option<Decimal>,
}

pub async fn get_info(State(pool): State<PgPool>) -> Result<Json<DeliveryInfoResponse>, AppError> {
    let settings = delivery::load_settings(&pool).await?;
    Ok(Json(DeliveryInfoResponse {
        enabled: settings.delivery_enabled,
        rate_per_km: settings.rate_per_km,
        minimum_fee: settings.minimum_fee,
        max_distance_km: settings.max_distance_km,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub q: Option<String>,
}

pub(crate) async fn suggest_addresses(delivery: &DeliveryConfig, q: Option<&str>) -> Result<Vec<Suggestion>, AppError> {
    let q = q.unwrap_or("").trim();
    let chars = q.chars().count();
    if !(QUERY_MIN_CHARS..=QUERY_MAX_CHARS).contains(&chars) {
        return Err(AppError::Validation(vec![FieldError { field: "q".into(), message: "invalid".into() }]));
    }
    delivery.suggest(q).await.map_err(|LookupDown| AppError::AddressLookupDown)
}

pub async fn suggest(
    State(delivery): State<DeliveryConfig>,
    Query(query): Query<SuggestQuery>,
) -> Result<Json<Vec<Suggestion>>, AppError> {
    Ok(Json(suggest_addresses(&delivery, query.q.as_deref()).await?))
}

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub address_id: String,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub address_id: String,
    pub address: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub distance_km: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fee: Decimal,
    pub estimated_by: EstimatedBy,
}

pub(crate) fn validate_address_id(address_id: &str) -> Result<(), AppError> {
    if delivery::is_valid_address_id(address_id) {
        Ok(())
    } else {
        Err(AppError::Validation(vec![FieldError { field: "address_id".into(), message: "invalid".into() }]))
    }
}

pub(crate) async fn quote_address(
    pool: &PgPool,
    delivery: &DeliveryConfig,
    address_id: &str,
    require_enabled: bool,
) -> Result<QuoteResponse, AppError> {
    validate_address_id(address_id)?;
    let quote = delivery.quote(pool, address_id, require_enabled).await?;
    Ok(QuoteResponse {
        address_id: quote.address_id,
        address: quote.address,
        distance_km: quote.distance_km,
        fee: quote.fee,
        estimated_by: quote.estimated_by,
    })
}

pub async fn quote(
    State(pool): State<PgPool>,
    State(delivery): State<DeliveryConfig>,
    Json(payload): Json<QuoteRequest>,
) -> Result<Json<QuoteResponse>, AppError> {
    Ok(Json(quote_address(&pool, &delivery, &payload.address_id, true).await?))
}
