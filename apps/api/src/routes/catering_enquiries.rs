use crate::error::{AppError, FieldError};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::extract::State;
use axum::{http::StatusCode, Json};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const EVENT_TYPES: [&str; 6] =
    ["wedding", "birthday", "corporate", "family_gathering", "outdoor_event", "other"];
const SERVICES: [&str; 4] = ["catering", "meal_delivery", "event_rental", "other"];
const LOCATION_MIN_LENGTH: usize = 5;
const MESSAGE_MAX_LENGTH: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct CreateCateringEnquiryRequest {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub location: String,
    pub services_required: Vec<String>,
    pub message: Option<String>,
    // Honeypot (tech-stack.md "Spam/Abuse Mitigation"). Real users never
    // see or fill this field; `#[serde(default)]` so older/other callers
    // that omit it entirely still deserialize.
    #[serde(default)]
    pub website: String,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

fn validate(payload: &CreateCateringEnquiryRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    let trimmed_phone = payload.phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError { field: "phone".into(), message: "required".into() });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError { field: "phone".into(), message: "invalid".into() });
    }

    if payload.email.trim().is_empty() {
        errors.push(FieldError { field: "email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.email) {
        errors.push(FieldError { field: "email".into(), message: "invalid".into() });
    }

    if !EVENT_TYPES.contains(&payload.event_type.as_str()) {
        errors.push(FieldError { field: "event_type".into(), message: "invalid".into() });
    }

    // requirement.md Decision 7 — must not be in the past.
    if payload.event_date < Utc::now().date_naive() {
        errors.push(FieldError { field: "event_date".into(), message: "must_be_future".into() });
    }

    if payload.guest_count <= 0 {
        errors.push(FieldError { field: "guest_count".into(), message: "must_be_positive".into() });
    }

    let trimmed_location = payload.location.trim();
    if trimmed_location.is_empty() {
        errors.push(FieldError { field: "location".into(), message: "required".into() });
    } else if trimmed_location.chars().count() < LOCATION_MIN_LENGTH {
        errors.push(FieldError { field: "location".into(), message: "too_short".into() });
    }

    // requirement.md Decision 5 — non-empty, every element a known value.
    if payload.services_required.is_empty() {
        errors.push(FieldError { field: "services_required".into(), message: "required".into() });
    } else if payload.services_required.iter().any(|s| !SERVICES.contains(&s.as_str())) {
        errors.push(FieldError { field: "services_required".into(), message: "invalid".into() });
    }

    if let Some(message) = &payload.message
        && message.chars().count() > MESSAGE_MAX_LENGTH
    {
        errors.push(FieldError { field: "message".into(), message: "too_long".into() });
    }

    errors
}

pub async fn create_catering_enquiry(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateCateringEnquiryRequest>,
) -> Result<(StatusCode, Json<CateringEnquiryResponse>), AppError> {
    // Honeypot tripped: pretend success, touch nothing. Checked before
    // validate()/the DB so a scripted submitter gets no signal at all
    // about which of its other fields were wrong.
    if !payload.website.trim().is_empty() {
        return Ok((
            StatusCode::CREATED,
            Json(CateringEnquiryResponse { id: 0, created_at: Utc::now() }),
        ));
    }

    let errors = validate(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required, message)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, created_at as "created_at: DateTime<Utc>"
        "#,
        payload.name.trim(),
        payload.phone.trim(),
        payload.email.trim(),
        payload.event_type,
        crate::chrono_date_to_offset(payload.event_date),
        payload.guest_count,
        payload.location.trim(),
        &payload.services_required,
        payload.message.as_deref().map(str::trim),
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(CateringEnquiryResponse { id: row.id, created_at: row.created_at })))
}
