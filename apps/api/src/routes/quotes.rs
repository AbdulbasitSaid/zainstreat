use crate::email::{EmailConfig, OutgoingEmail};
use crate::error::{AppError, FieldError};
use crate::order_emails::Locale;
use crate::payment::{payment_reference, PaymentConfig};
use crate::quote_status::{customer_can_accept, display_status, is_expired};
use crate::routes::orders::{load_order_by_token, parse_token, send_invoice};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

const DECLINE_REASON_MAX_LENGTH: usize = 500;

#[derive(Debug, Serialize)]
pub struct QuoteItemResponse {
    pub description: String,
    pub quantity: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
}

pub(crate) async fn load_quote_items(pool: &PgPool, quote_id: i64) -> Result<Vec<QuoteItemResponse>, AppError> {
    Ok(sqlx::query_as!(
        QuoteItemResponse,
        r#"
        SELECT description, quantity, unit_price, subtotal
        FROM catering_quote_items
        WHERE quote_id = $1
        ORDER BY position
        "#,
        quote_id
    )
    .fetch_all(pool)
    .await?)
}

/// What the customer's quote page shows (Phase 23 Decision 4). No email or
/// phone: the token is the only thing protecting the page.
#[derive(Debug, Serialize)]
pub struct PublicQuoteResponse {
    pub id: i64,
    /// Echoed back on accept/decline (Phase 23 Decision 13).
    pub version: i32,
    /// `draft`, `sent`, `expired`, `accepted` or `declined`.
    pub display_status: &'static str,
    pub customer_name: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub delivery_type: String,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<QuoteItemResponse>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub valid_until: NaiveDate,
    /// The order's tracking token, once accepted.
    pub order_token: Option<Uuid>,
}

async fn load_public_quote(pool: &PgPool, token: Uuid) -> Result<PublicQuoteResponse, AppError> {
    let quote = sqlx::query!(
        r#"
        SELECT q.id, q.version, q.status, q.delivery_type, q.delivery_address, q.notes, q.total,
               q.valid_until as "valid_until: NaiveDate",
               e.name, e.event_type, e.event_date as "event_date: NaiveDate", e.guest_count,
               o.public_token as "order_token?",
               (now() AT TIME ZONE 'Europe/Amsterdam')::date as "today!: NaiveDate"
        FROM catering_quotes q
        JOIN catering_enquiries e ON e.id = q.enquiry_id
        LEFT JOIN orders o ON o.id = q.order_id
        WHERE q.public_token = $1
        "#,
        token
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(PublicQuoteResponse {
        id: quote.id,
        version: quote.version,
        display_status: display_status(&quote.status, quote.valid_until, quote.today),
        customer_name: quote.name,
        event_type: quote.event_type,
        event_date: quote.event_date,
        guest_count: quote.guest_count,
        delivery_type: quote.delivery_type,
        delivery_address: quote.delivery_address,
        notes: quote.notes,
        items: load_quote_items(pool, quote.id).await?,
        total: quote.total,
        valid_until: quote.valid_until,
        order_token: quote.order_token,
    })
}

pub async fn get_quote_by_token(
    State(pool): State<PgPool>,
    Path(token): Path<String>,
) -> Result<Json<PublicQuoteResponse>, AppError> {
    let token = parse_token(&token)?;
    Ok(Json(load_public_quote(&pool, token).await?))
}

#[derive(Debug, Deserialize)]
pub struct AcceptQuoteRequest {
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct AcceptQuoteResponse {
    pub order_token: Uuid,
}

/// The customer accepts a sent quote, which creates a normal order in the
/// same transaction (Phase 23 Decisions 1, 4, 7). The quote row is locked,
/// so a double click or two racing requests create one order; a repeat
/// returns that same order.
pub async fn accept_quote(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
    Json(payload): Json<AcceptQuoteRequest>,
) -> Result<Json<AcceptQuoteResponse>, AppError> {
    let token = parse_token(&token)?;

    let mut tx = pool.begin().await?;

    let quote = sqlx::query!(
        r#"
        SELECT q.id, q.status, q.version, q.delivery_type, q.delivery_address, q.total, q.order_id,
               q.valid_until as "valid_until: NaiveDate",
               e.name, e.email, e.phone, e.event_type, e.event_date as "event_date: NaiveDate",
               e.guest_count, e.locale,
               (now() AT TIME ZONE 'Europe/Amsterdam')::date as "today!: NaiveDate"
        FROM catering_quotes q
        JOIN catering_enquiries e ON e.id = q.enquiry_id
        WHERE q.public_token = $1
        FOR UPDATE OF q
        "#,
        token
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    // Already accepted: same order back, whatever version the page had.
    // The order is looked up separately: a request that waited on the lock
    // sees the quote row as updated, but a joined `orders` row as it was
    // before (Postgres re-checks only the locked row).
    if let Some(order_id) = quote.order_id {
        let order_token = sqlx::query_scalar!("SELECT public_token FROM orders WHERE id = $1", order_id)
            .fetch_one(&mut *tx)
            .await?;
        return Ok(Json(AcceptQuoteResponse { order_token }));
    }
    // Checked before the status, so a quote the owner pulled back to draft
    // also reads as "changed" (Phase 23 Decision 13).
    if quote.version != payload.version {
        return Err(AppError::QuoteChanged);
    }
    if !customer_can_accept(&quote.status, quote.valid_until, quote.today) {
        if is_expired(&quote.status, quote.valid_until, quote.today) {
            return Err(AppError::QuoteExpired);
        }
        return Err(AppError::InvalidStatusTransition);
    }

    let locale = Locale::parse(&quote.locale).unwrap_or(Locale::En);
    let event_date = quote.event_date.format("%d-%m-%Y").to_string();
    // Decision 7: the event context rides along in the order's notes.
    let notes = format!(
        "Catering quote #{} for {} on {event_date}, {} guests",
        quote.id, quote.event_type, quote.guest_count,
    );

    let order = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total, locale)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $7, $8)
        RETURNING id, public_token,
                  to_char(created_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "placed_at!"
        "#,
        quote.name,
        quote.email,
        quote.phone,
        quote.delivery_type,
        quote.delivery_address,
        notes,
        quote.total,
        locale.as_str(),
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO order_items (order_id, item_name, unit_price, quantity, subtotal)
        SELECT $1, description, unit_price, quantity, subtotal
        FROM catering_quote_items
        WHERE quote_id = $2
        ORDER BY position
        "#,
        order.id,
        quote.id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE catering_quotes
        SET status = 'accepted', accepted_at = now(), order_id = $2, updated_at = now()
        WHERE id = $1
        "#,
        quote.id,
        order.id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(quote_id = quote.id, order_id = order.id, "customer accepted catering quote");

    // After the commit, like `create_order`: the customer gets the usual
    // invoice, and the owner a heads-up.
    let response = load_order_by_token(&pool, &payment, order.public_token).await?;
    send_invoice(&email, &pool, locale, &response, &order.placed_at);

    if let Some(business_address) = email.business_notify_address() {
        email.spawn_send(OutgoingEmail {
            to: business_address,
            subject: format!(
                "Quote #{} accepted — order {} — €{}",
                quote.id,
                payment_reference(order.id),
                response.total
            ),
            body: format!(
                "{} accepted quote #{} for their {} on {event_date} ({} guests).\n\nOrder {} has been created and is awaiting payment.\n\n{}/orders/{}",
                quote.name,
                quote.id,
                quote.event_type,
                quote.guest_count,
                payment_reference(order.id),
                email.admin_url(),
                order.id,
            ),
            reply_to: Some(quote.email.clone()),
            track: None,
        });
    }

    Ok(Json(AcceptQuoteResponse { order_token: order.public_token }))
}

#[derive(Debug, Deserialize)]
pub struct DeclineQuoteRequest {
    pub version: i32,
    pub reason: Option<String>,
}

/// The customer declines a sent quote, expired or not (Phase 23 Decision
/// 4). Idempotent: the guarded UPDATE decides, so only the request that
/// actually declines it emails the owner.
pub async fn decline_quote(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    Path(token): Path<String>,
    Json(payload): Json<DeclineQuoteRequest>,
) -> Result<Json<PublicQuoteResponse>, AppError> {
    let token = parse_token(&token)?;

    let reason = payload.reason.as_deref().map(str::trim).filter(|r| !r.is_empty());
    if reason.is_some_and(|r| r.chars().count() > DECLINE_REASON_MAX_LENGTH) {
        return Err(AppError::Validation(vec![FieldError {
            field: "reason".into(),
            message: "too_long".into(),
        }]));
    }

    let declined = sqlx::query!(
        r#"
        UPDATE catering_quotes q
        SET status = 'declined', declined_at = now(), decline_reason = $2, updated_at = now()
        FROM catering_enquiries e
        WHERE e.id = q.enquiry_id AND q.public_token = $1 AND q.status = 'sent' AND q.version = $3
        RETURNING q.id, q.enquiry_id, e.name, e.email, e.event_type, e.event_date as "event_date: NaiveDate"
        "#,
        token,
        reason,
        payload.version,
    )
    .fetch_optional(&pool)
    .await?;

    let quote = load_public_quote(&pool, token).await?;

    let Some(row) = declined else {
        return match quote.display_status {
            "declined" => Ok(Json(quote)),
            _ if quote.version != payload.version => Err(AppError::QuoteChanged),
            _ => Err(AppError::InvalidStatusTransition),
        };
    };

    tracing::info!(quote_id = row.id, "customer declined catering quote");

    if let Some(business_address) = email.business_notify_address() {
        email.spawn_send(OutgoingEmail {
            to: business_address,
            subject: format!("Quote #{} declined by {}", row.id, row.name),
            body: format!(
                "{} declined quote #{} for their {} on {}.\n\nReason: {}\n\n{}/enquiries/catering/{}",
                row.name,
                row.id,
                row.event_type,
                row.event_date.format("%d-%m-%Y"),
                reason.unwrap_or("(none given)"),
                email.admin_url(),
                row.enquiry_id,
            ),
            reply_to: Some(row.email.clone()),
            track: None,
        });
    }

    Ok(Json(quote))
}
