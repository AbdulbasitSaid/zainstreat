use crate::email::{latest_delivery, EmailConfig, EmailDeliveryStatus};
use crate::error::AppError;
use crate::routes::admin::quotes::{AdminQuoteResponse, load_admin_quote};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn clamp(params: &ListQuery) -> (i64, i64) {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);
    (limit, offset)
}

#[derive(Debug, Serialize)]
pub struct CateringEnquirySummary {
    pub id: i64,
    pub name: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub created_at: DateTime<Utc>,
    /// The quote's display status, or `None` before one is started
    /// (Phase 23 Decision 8).
    pub quote_status: Option<String>,
}

struct CateringEnquiryDetailRow {
    id: i64,
    name: String,
    phone: String,
    email: String,
    event_type: String,
    event_date: NaiveDate,
    guest_count: i32,
    location: String,
    services_required: Vec<String>,
    message: Option<String>,
    locale: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryDetail {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub location: String,
    pub services_required: Vec<String>,
    pub message: Option<String>,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub quote: Option<AdminQuoteResponse>,
    /// The quote email's outcome (Phase 25 Decision 6a).
    pub customer_email_delivery: Option<EmailDeliveryStatus>,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryListResponse {
    pub enquiries: Vec<CateringEnquirySummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_catering_enquiries(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<CateringEnquiryListResponse>, AppError> {
    let (limit, offset) = clamp(&params);

    let enquiries = sqlx::query_as!(
        CateringEnquirySummary,
        r#"SELECT e.id, e.name, e.event_type, e.event_date as "event_date: NaiveDate", e.guest_count,
                  e.created_at as "created_at: DateTime<Utc>",
                  CASE WHEN q.status = 'sent' AND q.valid_until < (now() AT TIME ZONE 'Europe/Amsterdam')::date
                       THEN 'expired' ELSE q.status END as "quote_status?"
           FROM catering_enquiries e
           LEFT JOIN catering_quotes q ON q.enquiry_id = e.id
           ORDER BY e.created_at DESC LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let total = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM catering_enquiries"#)
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    Ok(Json(CateringEnquiryListResponse {
        enquiries,
        total,
        limit,
        offset,
    }))
}

pub async fn get_catering_enquiry(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    Path(id): Path<i64>,
) -> Result<Json<CateringEnquiryDetail>, AppError> {
    let row = sqlx::query_as!(
        CateringEnquiryDetailRow,
        r#"SELECT id, name, phone, email, event_type, event_date as "event_date: NaiveDate", guest_count, location,
                  services_required, message, locale,
                  created_at as "created_at: DateTime<Utc>"
           FROM catering_enquiries WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let quote = load_admin_quote(&pool, email.site_url(), id).await?;
    let customer_email_delivery = match &quote {
        Some(quote) => latest_delivery(&pool, None, Some(quote.id)).await?,
        None => None,
    };

    Ok(Json(CateringEnquiryDetail {
        id: row.id,
        name: row.name,
        phone: row.phone,
        email: row.email,
        event_type: row.event_type,
        event_date: row.event_date,
        guest_count: row.guest_count,
        location: row.location,
        services_required: row.services_required,
        message: row.message,
        locale: row.locale,
        created_at: row.created_at,
        quote,
        customer_email_delivery,
    }))
}

#[derive(Debug, Serialize)]
pub struct ContactMessageSummary {
    pub id: i64,
    pub name: String,
    pub subject: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageDetail {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub subject: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageListResponse {
    pub messages: Vec<ContactMessageSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_contact_messages(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ContactMessageListResponse>, AppError> {
    let (limit, offset) = clamp(&params);

    let messages = sqlx::query_as!(
        ContactMessageSummary,
        r#"SELECT id, name, subject,
                  created_at as "created_at: DateTime<Utc>"
           FROM contact_messages ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let total = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM contact_messages"#)
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    Ok(Json(ContactMessageListResponse {
        messages,
        total,
        limit,
        offset,
    }))
}

pub async fn get_contact_message(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<ContactMessageDetail>, AppError> {
    let message = sqlx::query_as!(
        ContactMessageDetail,
        r#"SELECT id, name, email, phone, subject, message,
                  created_at as "created_at: DateTime<Utc>"
           FROM contact_messages WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(message))
}
