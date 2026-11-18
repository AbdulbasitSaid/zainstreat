//! Phase 36: admin-managed rentals (requirement.md Decisions 8, 9). Saves
//! are live at once (no draft → publish), with `updated_at` as the
//! optimistic-concurrency token like the other admin forms.

use crate::MediaConfig;
use crate::chrono_to_offset;
use crate::error::{AppError, FieldError};
use crate::routes::admin::menu_items::{delete_media_object, is_valid_media_url};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const NAME_MAX_CHARS: usize = 120;
const DESCRIPTION_MAX_CHARS: usize = 500;
const DISPLAY_ORDER_MAX: i32 = 9999;

#[derive(Debug, Serialize)]
pub struct AdminRentalResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub display_order: i32,
    pub is_archived: bool,
    pub updated_at: DateTime<Utc>,
}

struct RentalRow {
    id: i64,
    name: String,
    description: Option<String>,
    price: Decimal,
    image_url: Option<String>,
    is_available: bool,
    display_order: i32,
    deleted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<RentalRow> for AdminRentalResponse {
    fn from(row: RentalRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            price: row.price,
            image_url: row.image_url,
            is_available: row.is_available,
            display_order: row.display_order,
            is_archived: row.deleted_at.is_some(),
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RentalFields {
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub display_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRentalRequest {
    #[serde(flatten)]
    pub fields: RentalFields,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveRentalRequest {
    pub updated_at: DateTime<Utc>,
}

/// Blank descriptions are stored as NULL.
fn clean_description(description: Option<&str>) -> Option<String> {
    description.map(str::trim).filter(|d| !d.is_empty()).map(str::to_string)
}

/// A row's own unchanged image passes, like menu items; a new one must be
/// an upload this API issued.
fn validate_rental(
    fields: &RentalFields,
    media: &MediaConfig,
    existing_image_url: Option<&str>,
) -> Result<(), AppError> {
    let mut errors = Vec::new();
    let mut push = |field: &str, message: &str| {
        errors.push(FieldError { field: field.into(), message: message.into() });
    };

    let name = fields.name.trim();
    if name.is_empty() {
        push("name", "required");
    } else if name.chars().count() > NAME_MAX_CHARS {
        push("name", "too_long");
    }
    if fields.description.as_deref().is_some_and(|d| d.trim().chars().count() > DESCRIPTION_MAX_CHARS) {
        push("description", "too_long");
    }
    // NUMERIC(10, 2): at most two decimals and eight digits before the point.
    if fields.price < Decimal::ZERO
        || fields.price.normalize().scale() > 2
        || fields.price >= Decimal::from(100_000_000)
    {
        push("price", "invalid");
    }
    if let Some(url) = &fields.image_url
        && existing_image_url != Some(url.as_str())
        && !is_valid_media_url(url, &media.public_base_url)
    {
        push("image_url", "invalid");
    }
    if !(0..=DISPLAY_ORDER_MAX).contains(&fields.display_order) {
        push("display_order", "invalid");
    }

    if errors.is_empty() { Ok(()) } else { Err(AppError::Validation(errors)) }
}

/// Every non-archived rental, available or not.
pub async fn list_rentals(State(pool): State<PgPool>) -> Result<Json<Vec<AdminRentalResponse>>, AppError> {
    let rows = sqlx::query_as!(
        RentalRow,
        r#"
        SELECT id, name, description, price, image_url, is_available, display_order,
               deleted_at as "deleted_at: DateTime<Utc>", updated_at as "updated_at: DateTime<Utc>"
        FROM rentals
        WHERE deleted_at IS NULL
        ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(AdminRentalResponse::from).collect()))
}

async fn load_rental(pool: &PgPool, id: i64) -> Result<RentalRow, AppError> {
    sqlx::query_as!(
        RentalRow,
        r#"
        SELECT id, name, description, price, image_url, is_available, display_order,
               deleted_at as "deleted_at: DateTime<Utc>", updated_at as "updated_at: DateTime<Utc>"
        FROM rentals
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn get_rental(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminRentalResponse>, AppError> {
    Ok(Json(load_rental(&pool, id).await?.into()))
}

/// A 0-row conditional UPDATE: gone or archived → 404, moved on → 409
/// with the live rental.
async fn rental_or_conflict(pool: &PgPool, id: i64) -> AppError {
    match load_rental(pool, id).await {
        Ok(row) if row.deleted_at.is_none() => AppError::Conflict(
            serde_json::to_value(AdminRentalResponse::from(row)).expect("serializable"),
        ),
        Ok(_) => AppError::NotFound,
        Err(err) => err,
    }
}

pub async fn create_rental(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(fields): Json<RentalFields>,
) -> Result<(StatusCode, Json<AdminRentalResponse>), AppError> {
    validate_rental(&fields, &media, None)?;

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO rentals (name, description, price, image_url, is_available, display_order)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
        fields.name.trim(),
        clean_description(fields.description.as_deref()),
        fields.price,
        fields.image_url,
        fields.is_available,
        fields.display_order,
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(load_rental(&pool, id).await?.into())))
}

/// A replaced photo is deleted from storage: rentals have no preview that
/// could still show it.
pub async fn update_rental(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateRentalRequest>,
) -> Result<Json<AdminRentalResponse>, AppError> {
    let current = load_rental(&pool, id).await?;
    if current.deleted_at.is_some() {
        return Err(AppError::NotFound);
    }
    let fields = &payload.fields;
    validate_rental(fields, &media, current.image_url.as_deref())?;

    let previous_image = sqlx::query_scalar!(
        r#"
        UPDATE rentals r
        SET name = $3, description = $4, price = $5, image_url = $6, is_available = $7,
            display_order = $8, updated_at = now()
        FROM (SELECT id, image_url FROM rentals WHERE id = $1 FOR UPDATE) before
        WHERE r.id = before.id AND r.updated_at = $2 AND r.deleted_at IS NULL
        RETURNING before.image_url
        "#,
        id,
        chrono_to_offset(payload.updated_at),
        fields.name.trim(),
        clean_description(fields.description.as_deref()),
        fields.price,
        fields.image_url,
        fields.is_available,
        fields.display_order,
    )
    .fetch_optional(&pool)
    .await?;

    let Some(previous_image) = previous_image else {
        return Err(rental_or_conflict(&pool, id).await);
    };
    if let Some(previous) = previous_image
        && fields.image_url.as_deref() != Some(previous.as_str())
    {
        delete_media_object(&media, &previous).await;
    }

    Ok(Json(load_rental(&pool, id).await?.into()))
}

/// Soft delete; the photo stays in storage, like an archived menu item's.
pub async fn archive_rental(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ArchiveRentalRequest>,
) -> Result<Json<AdminRentalResponse>, AppError> {
    let affected = sqlx::query!(
        r#"
        UPDATE rentals SET deleted_at = now(), updated_at = now()
        WHERE id = $1 AND updated_at = $2 AND deleted_at IS NULL
        "#,
        id,
        chrono_to_offset(payload.updated_at),
    )
    .execute(&pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(rental_or_conflict(&pool, id).await);
    }

    Ok(Json(load_rental(&pool, id).await?.into()))
}
