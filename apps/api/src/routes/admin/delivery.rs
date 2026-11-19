//! Phase 37: delivery settings, the owner's address lookups, and setting a
//! delivery fee on an order (requirement.md Decisions 5, 6, 14, 15, 17).
//! The only place the business origin is returned (Decision 7). Saves are
//! live at once, with `updated_at` optimistic concurrency like rentals.

use crate::chrono_to_offset;
use crate::delivery::{self, DeliveryConfig, DeliverySettings, EstimatedBy, LookupDown, Suggestion};
use crate::email::{DeliveryRef, EmailConfig, OutgoingEmail};
use crate::error::{AppError, FieldError};
use crate::order_emails::delivery_fee_confirmed_email;
use crate::payment::PaymentConfig;
use crate::routes::admin::orders::{AdminOrderResponse, get_order};
use crate::routes::delivery::{QuoteRequest, QuoteResponse, SuggestQuery, quote_address, suggest_addresses};
use crate::routes::orders::load_order_by_token;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

const RATE_MAX: Decimal = Decimal::from_parts(100, 0, 0, false, 0);
const MINIMUM_FEE_MAX: Decimal = Decimal::from_parts(500, 0, 0, false, 0);
const MAX_DISTANCE_LIMIT: Decimal = Decimal::from_parts(500, 0, 0, false, 0);
const ORDER_FEE_MAX: Decimal = Decimal::from_parts(500, 0, 0, false, 0);

#[derive(Debug, Serialize)]
pub struct AdminDeliverySettingsResponse {
    pub origin_address: Option<String>,
    /// Whether the origin has coordinates; delivery can't be switched on
    /// without them (Decision 6).
    pub origin_located: bool,
    #[serde(with = "rust_decimal::serde::str")]
    pub rate_per_km: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub minimum_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub max_distance_km: Option<Decimal>,
    pub delivery_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<DeliverySettings> for AdminDeliverySettingsResponse {
    fn from(settings: DeliverySettings) -> Self {
        Self {
            origin_located: settings.origin().is_some(),
            origin_address: settings.origin_address,
            rate_per_km: settings.rate_per_km,
            minimum_fee: settings.minimum_fee,
            max_distance_km: settings.max_distance_km,
            delivery_enabled: settings.delivery_enabled,
            updated_at: settings.updated_at,
        }
    }
}

/// Locates a seeded origin on the way (the migration copies text only).
pub async fn get_settings(
    State(pool): State<PgPool>,
    State(delivery): State<DeliveryConfig>,
) -> Result<Json<AdminDeliverySettingsResponse>, AppError> {
    let mut settings = delivery::load_settings(&pool).await?;
    delivery.ensure_origin(&pool, &mut settings).await?;
    Ok(Json(settings.into()))
}

#[derive(Debug, Deserialize)]
pub struct SaveSettingsRequest {
    pub updated_at: DateTime<Utc>,
    /// A PDOK id to move the origin to; absent keeps the current one.
    pub origin_address_id: Option<String>,
    pub rate_per_km: Decimal,
    pub minimum_fee: Decimal,
    pub max_distance_km: Option<Decimal>,
    pub delivery_enabled: bool,
}

fn out_of_range(value: Decimal, min: Decimal, max: Decimal) -> bool {
    value < min || value > max || value.scale() > 2
}

pub async fn save_settings(
    State(pool): State<PgPool>,
    State(delivery): State<DeliveryConfig>,
    Json(payload): Json<SaveSettingsRequest>,
) -> Result<Json<AdminDeliverySettingsResponse>, AppError> {
    let mut errors = Vec::new();
    let mut push = |field: &str, message: &str| errors.push(FieldError { field: field.into(), message: message.into() });
    if out_of_range(payload.rate_per_km, Decimal::ZERO, RATE_MAX) {
        push("rate_per_km", "invalid");
    }
    if out_of_range(payload.minimum_fee, Decimal::ZERO, MINIMUM_FEE_MAX) {
        push("minimum_fee", "invalid");
    }
    if payload.max_distance_km.is_some_and(|max| out_of_range(max, Decimal::ONE, MAX_DISTANCE_LIMIT)) {
        push("max_distance_km", "invalid");
    }

    let mut settings = delivery::load_settings(&pool).await?;
    let new_origin = match payload.origin_address_id.as_deref() {
        Some(id) if !delivery::is_valid_address_id(id) => {
            push("origin_address_id", "invalid");
            None
        }
        Some(id) => match delivery.lookup(id).await {
            Ok(Some(found)) => Some(found),
            Ok(None) => {
                push("origin_address_id", "not_found");
                None
            }
            Err(LookupDown) => return Err(AppError::AddressLookupDown),
        },
        None => {
            delivery.ensure_origin(&pool, &mut settings).await?;
            None
        }
    };
    let located = new_origin.is_some() || settings.origin().is_some();
    if payload.delivery_enabled && !located {
        push("delivery_enabled", "origin_required");
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let (origin_address, origin_lat, origin_lon) = match new_origin {
        Some(found) => (Some(found.label), Some(found.lat), Some(found.lon)),
        None => (settings.origin_address.clone(), settings.origin_lat, settings.origin_lon),
    };
    let saved = sqlx::query!(
        r#"
        UPDATE delivery_settings
        SET origin_address = $2, origin_lat = $3, origin_lon = $4, rate_per_km = $5,
            minimum_fee = $6, max_distance_km = $7, delivery_enabled = $8, updated_at = now()
        WHERE updated_at = $1
        "#,
        chrono_to_offset(payload.updated_at),
        origin_address,
        origin_lat,
        origin_lon,
        payload.rate_per_km,
        payload.minimum_fee,
        payload.max_distance_km,
        payload.delivery_enabled,
    )
    .execute(&pool)
    .await?;

    let current: AdminDeliverySettingsResponse = delivery::load_settings(&pool).await?.into();
    if saved.rows_affected() == 0 {
        return Err(AppError::Conflict(json!(current)));
    }
    tracing::info!(enabled = payload.delivery_enabled, "admin saved delivery settings");
    Ok(Json(current))
}

pub async fn suggest(
    State(delivery): State<DeliveryConfig>,
    Query(query): Query<SuggestQuery>,
) -> Result<Json<Vec<Suggestion>>, AppError> {
    Ok(Json(suggest_addresses(&delivery, query.q.as_deref()).await?))
}

/// "Test an address" and the quote editor's "Calculate delivery": works
/// while delivery is still switched off (Decision 15).
pub async fn quote(
    State(pool): State<PgPool>,
    State(delivery): State<DeliveryConfig>,
    Json(payload): Json<QuoteRequest>,
) -> Result<Json<QuoteResponse>, AppError> {
    Ok(Json(quote_address(&pool, &delivery, &payload.address_id, false).await?))
}

#[derive(Debug, Deserialize)]
pub struct SetOrderFeeRequest {
    pub updated_at: DateTime<Utc>,
    /// The owner's fee; absent means "use the calculated one".
    pub fee: Option<Decimal>,
    /// A PDOK id to calculate from.
    pub address_id: Option<String>,
}

/// Sets the delivery fee on an unpaid delivery order (Decision 17 and open
/// risk 6), recomputes the total and emails the customer the updated
/// invoice. Not once the customer has said they've paid.
pub async fn set_order_fee(
    State(pool): State<PgPool>,
    State(delivery): State<DeliveryConfig>,
    State(email): State<EmailConfig>,
    State(payment): State<PaymentConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<SetOrderFeeRequest>,
) -> Result<Json<AdminOrderResponse>, AppError> {
    let calculated = match payload.address_id.as_deref() {
        Some(address_id) => Some(quote_address(&pool, &delivery, address_id, false).await?),
        None => None,
    };
    let (fee, distance_km, rate_per_km, estimated_by) = match (payload.fee, &calculated) {
        (None, None) => {
            return Err(AppError::Validation(vec![FieldError { field: "fee".into(), message: "required".into() }]));
        }
        (Some(fee), _) if out_of_range(fee, Decimal::ZERO, ORDER_FEE_MAX) => {
            return Err(AppError::Validation(vec![FieldError { field: "fee".into(), message: "invalid".into() }]));
        }
        (fee, Some(quote)) => {
            let settings = delivery::load_settings(&pool).await?;
            let fee = fee.unwrap_or(quote.fee);
            let by = if fee == quote.fee { quote.estimated_by } else { EstimatedBy::Manual };
            (fee, Some(quote.distance_km), Some(settings.rate_per_km), by)
        }
        (Some(fee), None) => (fee, None, None, EstimatedBy::Manual),
    };

    let updated = sqlx::query!(
        r#"
        UPDATE orders
        SET delivery_fee = $3, total = subtotal + $3, delivery_distance_km = $4,
            delivery_rate_per_km = $5, delivery_estimated_by = $6,
            delivery_fee_pending = false, updated_at = now()
        WHERE id = $1 AND updated_at = $2 AND delivery_type = 'delivery'
          AND status = 'awaiting_payment' AND payment_claimed_at IS NULL
        RETURNING public_token,
                  to_char(created_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "placed_at!"
        "#,
        id,
        chrono_to_offset(payload.updated_at),
        fee,
        distance_km,
        rate_per_km,
        estimated_by.as_str(),
    )
    .fetch_optional(&pool)
    .await?;

    let Some(updated) = updated else {
        let current = get_order(State(pool.clone()), Path(id)).await?.0;
        let editable = current.delivery_type == "delivery"
            && current.status == "awaiting_payment"
            && current.payment_claimed_at.is_none();
        return Err(if editable { AppError::Conflict(json!(current)) } else { AppError::InvalidStatusTransition });
    };
    tracing::info!(order_id = id, by = estimated_by.as_str(), "admin set delivery fee");

    let order = load_order_by_token(&pool, &payment, updated.public_token).await?;
    let (subject, body) = delivery_fee_confirmed_email(&order, &updated.placed_at, email.site_url(), order.public_token);
    email.spawn_send(OutgoingEmail {
        to: order.customer_email.clone(),
        subject,
        body,
        reply_to: None,
        track: Some(DeliveryRef { pool: pool.clone(), order_id: Some(order.id), quote_id: None, kind: "invoice" }),
    });

    get_order(State(pool), Path(id)).await
}
