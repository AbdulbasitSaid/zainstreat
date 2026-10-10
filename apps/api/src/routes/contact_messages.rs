use crate::email::EmailConfig;
use crate::error::{AppError, FieldError};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const SUBJECTS: [&str; 6] = ["general", "catering", "event_rental", "menu", "order", "other"];
const MESSAGE_MAX_LENGTH: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct CreateContactMessageRequest {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub subject: String,
    pub message: String,
    // Honeypot — see catering_enquiries.rs's identical field for the full rationale.
    #[serde(default)]
    pub website: String,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

fn validate(payload: &CreateContactMessageRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    if payload.email.trim().is_empty() {
        errors.push(FieldError { field: "email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.email) {
        errors.push(FieldError { field: "email".into(), message: "invalid".into() });
    }

    let trimmed_phone = payload.phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError { field: "phone".into(), message: "required".into() });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError { field: "phone".into(), message: "invalid".into() });
    }

    if !SUBJECTS.contains(&payload.subject.as_str()) {
        errors.push(FieldError { field: "subject".into(), message: "invalid".into() });
    }

    let trimmed_message = payload.message.trim();
    if trimmed_message.is_empty() {
        errors.push(FieldError { field: "message".into(), message: "required".into() });
    } else if trimmed_message.chars().count() > MESSAGE_MAX_LENGTH {
        errors.push(FieldError { field: "message".into(), message: "too_long".into() });
    }

    errors
}

pub async fn create_contact_message(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    Json(payload): Json<CreateContactMessageRequest>,
) -> Result<(StatusCode, Json<ContactMessageResponse>), AppError> {
    // Honeypot tripped — see catering_enquiries.rs's identical check for the rationale.
    if !payload.website.trim().is_empty() {
        return Ok((
            StatusCode::CREATED,
            Json(ContactMessageResponse { id: 0, created_at: Utc::now() }),
        ));
    }

    let errors = validate(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO contact_messages (name, email, phone, subject, message)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, created_at as "created_at: DateTime<Utc>"
        "#,
        payload.name.trim(),
        payload.email.trim(),
        payload.phone.trim(),
        payload.subject,
        payload.message.trim(),
    )
    .fetch_one(&pool)
    .await?;

    if let Some(to) = email.business_notify_address() {
        email.spawn_send(
            to,
            format!("New contact message from {}", payload.name.trim()),
            format!(
                "Name: {}\nEmail: {}\nPhone: {}\nSubject: {}\n\n{}",
                payload.name.trim(),
                payload.email.trim(),
                payload.phone.trim(),
                payload.subject,
                payload.message.trim(),
            ),
        );
    }

    Ok((StatusCode::CREATED, Json(ContactMessageResponse { id: row.id, created_at: row.created_at })))
}
