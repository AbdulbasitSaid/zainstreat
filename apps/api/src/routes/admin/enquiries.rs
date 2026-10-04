use crate::error::AppError;
use axum::{
    extract::{Path, Query, State},
    Json,
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
    pub created_at: DateTime<Utc>,
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
        r#"SELECT id, name, event_type, event_date as "event_date: NaiveDate", guest_count,
                  created_at as "created_at: DateTime<Utc>"
           FROM catering_enquiries ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let total = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM catering_enquiries"#)
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    Ok(Json(CateringEnquiryListResponse { enquiries, total, limit, offset }))
}

pub async fn get_catering_enquiry(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<CateringEnquiryDetail>, AppError> {
    let enquiry = sqlx::query_as!(
        CateringEnquiryDetail,
        r#"SELECT id, name, phone, email, event_type, event_date as "event_date: NaiveDate", guest_count, location,
                  services_required, message,
                  created_at as "created_at: DateTime<Utc>"
           FROM catering_enquiries WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(enquiry))
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

    Ok(Json(ContactMessageListResponse { messages, total, limit, offset }))
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
