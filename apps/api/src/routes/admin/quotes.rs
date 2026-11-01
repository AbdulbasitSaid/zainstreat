use crate::email::EmailConfig;
use crate::error::{AppError, FieldError};
use crate::order_emails::{quote_email, quote_url, Locale, QuoteEmail, QuoteEmailLine};
use crate::payment::payment_reference;
use crate::quote_status::{admin_can_edit, display_status};
use crate::routes::quotes::{load_quote_items, QuoteItemResponse};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// Phase 23 Decision 16: generous, but enough to catch a stray extra zero.
const MAX_LINES: usize = 50;
const DESCRIPTION_MAX_LENGTH: usize = 200;
const MAX_QUANTITY: i32 = 10_000;
const MAX_UNIT_PRICE: Decimal = Decimal::from_parts(10_000_000, 0, 0, false, 2); // €100,000.00
const ADDRESS_MIN_LENGTH: usize = 5;
const NOTES_MAX_LENGTH: usize = 1000;

/// Unlike the cart's `DeliveryType`, a quote can also be served on-site
/// (Phase 23 Decision 12).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FulfilmentType {
    Pickup,
    Delivery,
    OnSite,
}

impl FulfilmentType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pickup => "pickup",
            Self::Delivery => "delivery",
            Self::OnSite => "on_site",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QuoteLineRequest {
    pub description: String,
    pub quantity: i32,
    pub unit_price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct SaveQuoteRequest {
    pub delivery_type: FulfilmentType,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    pub valid_until: NaiveDate,
    pub items: Vec<QuoteLineRequest>,
}

#[derive(Debug, Serialize)]
pub struct AdminQuoteResponse {
    pub id: i64,
    pub status: String,
    /// The stored status, or `expired` (Phase 23 Decision 3).
    pub display_status: &'static str,
    pub version: i32,
    /// The customer's link, in the enquiry's language, for the share panel
    /// (Phase 23 Decision 14).
    pub quote_url: String,
    pub delivery_type: String,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub valid_until: NaiveDate,
    pub sent_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub declined_at: Option<DateTime<Utc>>,
    pub decline_reason: Option<String>,
    pub order_id: Option<i64>,
    pub order_reference: Option<String>,
    pub items: Vec<QuoteItemResponse>,
}

/// The enquiry's quote, or `None` if the owner hasn't started one.
pub(crate) async fn load_admin_quote(
    pool: &PgPool,
    site_url: &str,
    enquiry_id: i64,
) -> Result<Option<AdminQuoteResponse>, AppError> {
    let Some(quote) = sqlx::query!(
        r#"
        SELECT q.id, q.status, q.version, q.public_token, q.delivery_type, q.delivery_address,
               q.notes, q.total, q.decline_reason, q.order_id,
               q.valid_until as "valid_until: NaiveDate",
               q.sent_at as "sent_at: DateTime<Utc>",
               q.accepted_at as "accepted_at: DateTime<Utc>",
               q.declined_at as "declined_at: DateTime<Utc>",
               e.locale,
               (now() AT TIME ZONE 'Europe/Amsterdam')::date as "today!: NaiveDate"
        FROM catering_quotes q
        JOIN catering_enquiries e ON e.id = q.enquiry_id
        WHERE q.enquiry_id = $1
        "#,
        enquiry_id
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let locale = Locale::parse(&quote.locale).unwrap_or(Locale::En);

    Ok(Some(AdminQuoteResponse {
        id: quote.id,
        display_status: display_status(&quote.status, quote.valid_until, quote.today),
        status: quote.status,
        version: quote.version,
        quote_url: quote_url(site_url, locale, quote.public_token),
        delivery_type: quote.delivery_type,
        delivery_address: quote.delivery_address,
        notes: quote.notes,
        total: quote.total,
        valid_until: quote.valid_until,
        sent_at: quote.sent_at,
        accepted_at: quote.accepted_at,
        declined_at: quote.declined_at,
        decline_reason: quote.decline_reason,
        order_id: quote.order_id,
        order_reference: quote.order_id.map(payment_reference),
        items: load_quote_items(pool, quote.id).await?,
    }))
}

async fn amsterdam_today(pool: &PgPool) -> Result<NaiveDate, AppError> {
    Ok(sqlx::query_scalar!(r#"SELECT (now() AT TIME ZONE 'Europe/Amsterdam')::date as "today!: NaiveDate""#)
        .fetch_one(pool)
        .await?)
}

fn validate(payload: &SaveQuoteRequest, today: NaiveDate) -> Vec<FieldError> {
    let mut errors = Vec::new();
    let mut push = |field: String, message: &str| errors.push(FieldError { field, message: message.into() });

    if payload.items.is_empty() {
        push("items".into(), "empty");
    } else if payload.items.len() > MAX_LINES {
        push("items".into(), "too_many");
    }

    for (index, line) in payload.items.iter().enumerate() {
        let description = line.description.trim();
        if description.is_empty() {
            push(format!("items[{index}].description"), "required");
        } else if description.chars().count() > DESCRIPTION_MAX_LENGTH {
            push(format!("items[{index}].description"), "too_long");
        }
        if !(1..=MAX_QUANTITY).contains(&line.quantity) {
            push(format!("items[{index}].quantity"), "out_of_range");
        }
        if line.unit_price < Decimal::ZERO || line.unit_price > MAX_UNIT_PRICE {
            push(format!("items[{index}].unit_price"), "out_of_range");
        } else if line.unit_price.normalize().scale() > 2 {
            push(format!("items[{index}].unit_price"), "too_precise");
        }
    }

    if payload.delivery_type != FulfilmentType::Pickup {
        let address = payload.delivery_address.as_deref().unwrap_or("").trim();
        if address.is_empty() {
            push("delivery_address".into(), "required");
        } else if address.chars().count() < ADDRESS_MIN_LENGTH {
            push("delivery_address".into(), "too_short");
        }
    }

    if payload.notes.as_deref().is_some_and(|n| n.chars().count() > NOTES_MAX_LENGTH) {
        push("notes".into(), "too_long");
    }

    if payload.valid_until < today {
        push("valid_until".into(), "must_be_future");
    }

    if !payload.items.is_empty() && quote_total(&payload.items) <= Decimal::ZERO {
        push("total".into(), "must_be_positive");
    }

    errors
}

fn line_subtotal(line: &QuoteLineRequest) -> Decimal {
    line.unit_price * Decimal::from(line.quantity)
}

fn quote_total(items: &[QuoteLineRequest]) -> Decimal {
    items.iter().map(line_subtotal).sum()
}

/// Creates or replaces the enquiry's quote. Saving always moves it back to
/// `draft` and bumps its version, so the customer never accepts a version
/// they weren't sent (Phase 23 Decisions 3, 13).
pub async fn save_quote(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    Path(enquiry_id): Path<i64>,
    Json(payload): Json<SaveQuoteRequest>,
) -> Result<Json<AdminQuoteResponse>, AppError> {
    let mut tx = pool.begin().await?;

    sqlx::query_scalar!("SELECT id FROM catering_enquiries WHERE id = $1", enquiry_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;

    // Locked so a customer's accept can't land between the check and the save.
    let existing = sqlx::query_scalar!("SELECT status FROM catering_quotes WHERE enquiry_id = $1 FOR UPDATE", enquiry_id)
        .fetch_optional(&mut *tx)
        .await?;
    if existing.is_some_and(|status| !admin_can_edit(&status)) {
        return Err(AppError::InvalidStatusTransition);
    }

    let errors = validate(&payload, amsterdam_today(&pool).await?);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let delivery_address = match payload.delivery_type {
        FulfilmentType::Pickup => None,
        _ => payload.delivery_address.as_deref().map(str::trim),
    };
    let notes = payload.notes.as_deref().map(str::trim).filter(|n| !n.is_empty());

    let quote_id = sqlx::query_scalar!(
        r#"
        INSERT INTO catering_quotes (enquiry_id, delivery_type, delivery_address, notes, total, valid_until)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (enquiry_id) DO UPDATE SET
            delivery_type = EXCLUDED.delivery_type,
            delivery_address = EXCLUDED.delivery_address,
            notes = EXCLUDED.notes,
            total = EXCLUDED.total,
            valid_until = EXCLUDED.valid_until,
            status = 'draft',
            decline_reason = NULL,
            declined_at = NULL,
            version = catering_quotes.version + 1,
            updated_at = now()
        RETURNING id
        "#,
        enquiry_id,
        payload.delivery_type.as_str(),
        delivery_address,
        notes,
        quote_total(&payload.items),
        crate::chrono_date_to_offset(payload.valid_until),
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM catering_quote_items WHERE quote_id = $1", quote_id)
        .execute(&mut *tx)
        .await?;

    for (position, line) in payload.items.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO catering_quote_items (quote_id, position, description, quantity, unit_price, subtotal)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            quote_id,
            position as i32,
            line.description.trim(),
            line.quantity,
            line.unit_price,
            line_subtotal(line),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!(enquiry_id, quote_id, "admin saved catering quote");

    load_admin_quote(&pool, email.site_url(), enquiry_id)
        .await?
        .map(Json)
        .ok_or(AppError::Internal)
}

#[derive(Debug, Serialize)]
pub struct SendQuoteResponse {
    pub quote: AdminQuoteResponse,
    /// False when email is switched off, so admin offers the share panel
    /// instead (Phase 23 Decision 14). "Queued", not "delivered": sending is
    /// fire-and-forget.
    pub email_queued: bool,
}

/// Sends (or re-sends) the quote to the customer. A declined quote can be
/// re-sent as it is; an accepted one is final.
pub async fn send_quote(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    Path(enquiry_id): Path<i64>,
) -> Result<Json<SendQuoteResponse>, AppError> {
    let mut tx = pool.begin().await?;

    let current = sqlx::query!(
        r#"
        SELECT id, status, valid_until as "valid_until: NaiveDate"
        FROM catering_quotes
        WHERE enquiry_id = $1
        FOR UPDATE
        "#,
        enquiry_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    if !admin_can_edit(&current.status) {
        return Err(AppError::InvalidStatusTransition);
    }
    if current.valid_until < amsterdam_today(&pool).await? {
        return Err(AppError::Validation(vec![FieldError {
            field: "valid_until".into(),
            message: "must_be_future".into(),
        }]));
    }

    sqlx::query!(
        r#"
        UPDATE catering_quotes
        SET status = 'sent', sent_at = now(), decline_reason = NULL, declined_at = NULL, updated_at = now()
        WHERE id = $1
        "#,
        current.id
    )
    .execute(&mut *tx)
    .await?;

    let enquiry = sqlx::query!(
        r#"
        SELECT name, email, guest_count, locale, event_date as "event_date: NaiveDate"
        FROM catering_enquiries
        WHERE id = $1
        "#,
        enquiry_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(enquiry_id, quote_id = current.id, "admin sent catering quote");

    let quote = load_admin_quote(&pool, email.site_url(), enquiry_id)
        .await?
        .ok_or(AppError::Internal)?;

    // After the commit, like `create_order`: a failed send never undoes it.
    let locale = Locale::parse(&enquiry.locale).unwrap_or(Locale::En);
    let lines: Vec<QuoteEmailLine> = quote
        .items
        .iter()
        .map(|item| QuoteEmailLine {
            description: &item.description,
            quantity: item.quantity,
            subtotal: item.subtotal,
        })
        .collect();
    let (subject, body) = quote_email(
        locale,
        &QuoteEmail {
            customer_name: &enquiry.name,
            quote_id: quote.id,
            event_date: enquiry.event_date,
            guest_count: enquiry.guest_count,
            delivery_type: &quote.delivery_type,
            delivery_address: quote.delivery_address.as_deref(),
            lines: &lines,
            total: quote.total,
            notes: quote.notes.as_deref(),
            valid_until: quote.valid_until,
            url: &quote.quote_url,
        },
    );
    email.spawn_send(enquiry.email, subject, body);

    Ok(Json(SendQuoteResponse { quote, email_queued: email.is_enabled() }))
}
