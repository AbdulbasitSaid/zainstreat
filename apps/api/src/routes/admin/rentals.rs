//! Phase 36: admin-managed rentals (requirement.md Decisions 8, 9). Saves
//! are live at once (no draft → publish), with `updated_at` as the
//! optimistic-concurrency token like the other admin forms.

use crate::MediaConfig;
use crate::chrono_to_offset;
use crate::error::{AppError, FieldError};
use crate::routes::admin::menu_items::{delete_media_object, is_valid_media_url};
use crate::translation::{self, Notice, StatusPair, TranslationConfig, Unit};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    /// Phase 38 Decision 1: name and description in nl/fr.
    pub translations: Value,
    pub translation_status: StatusPair,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_notice: Option<Notice>,
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
    translations: Value,
}

fn rental_unit(name: &str, description: Option<&str>, translations: Value) -> Unit {
    Unit::new(translation::english(&[("name", Some(name)), ("description", description)]), translations)
}

impl RentalRow {
    fn into_response(self, keep_version: i32) -> AdminRentalResponse {
        let translation_status =
            rental_unit(&self.name, self.description.as_deref(), self.translations.clone()).status(keep_version);
        AdminRentalResponse {
            translations: self.translations,
            translation_status,
            translation_notice: None,
            id: self.id,
            name: self.name,
            description: self.description,
            price: self.price,
            image_url: self.image_url,
            is_available: self.is_available,
            display_order: self.display_order,
            is_archived: self.deleted_at.is_some(),
            updated_at: self.updated_at,
        }
    }
}

async fn respond(pool: &PgPool, id: i64, notice: Option<Notice>) -> Result<AdminRentalResponse, AppError> {
    let keep_version = translation::load_settings(pool).await?.keep_version;
    let mut response = load_rental(pool, id).await?.into_response(keep_version);
    response.translation_notice = notice;
    Ok(response)
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
    /// Phase 38 Decision 5: the owner's nl/fr, if edited.
    #[serde(default = "translation::empty_translations")]
    pub translations: Value,
}

/// Phase 38 Decisions 2, 3, 5: the owner's nl/fr edits, then DeepL.
async fn translate_rental(
    config: &TranslationConfig,
    pool: &PgPool,
    fields: &mut RentalFields,
) -> Result<Option<Notice>, AppError> {
    let description = clean_description(fields.description.as_deref());
    let mut units = [rental_unit(fields.name.trim(), description.as_deref(), fields.translations.clone())];
    let notice = translation::prepare(config, pool, &mut units).await?;
    let [unit] = units;
    fields.translations = unit.translations;
    Ok(notice)
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
               deleted_at as "deleted_at: DateTime<Utc>", updated_at as "updated_at: DateTime<Utc>",
               translations
        FROM rentals
        WHERE deleted_at IS NULL
        ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    let keep_version = translation::load_settings(&pool).await?.keep_version;
    Ok(Json(rows.into_iter().map(|row| row.into_response(keep_version)).collect()))
}

async fn load_rental(pool: &PgPool, id: i64) -> Result<RentalRow, AppError> {
    sqlx::query_as!(
        RentalRow,
        r#"
        SELECT id, name, description, price, image_url, is_available, display_order,
               deleted_at as "deleted_at: DateTime<Utc>", updated_at as "updated_at: DateTime<Utc>",
               translations
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
    Ok(Json(respond(&pool, id, None).await?))
}

/// A 0-row conditional UPDATE: gone or archived → 404, moved on → 409
/// with the live rental.
async fn rental_or_conflict(pool: &PgPool, id: i64) -> AppError {
    match load_rental(pool, id).await {
        Ok(row) if row.deleted_at.is_none() => match translation::load_settings(pool).await {
            Ok(settings) => AppError::Conflict(
                serde_json::to_value(row.into_response(settings.keep_version)).expect("serializable"),
            ),
            Err(err) => AppError::Database(err),
        },
        Ok(_) => AppError::NotFound,
        Err(err) => err,
    }
}

pub async fn create_rental(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    State(translator): State<TranslationConfig>,
    Json(mut fields): Json<RentalFields>,
) -> Result<(StatusCode, Json<AdminRentalResponse>), AppError> {
    validate_rental(&fields, &media, None)?;
    let notice = translate_rental(&translator, &pool, &mut fields).await?;

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO rentals (name, description, price, image_url, is_available, display_order, translations)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        fields.name.trim(),
        clean_description(fields.description.as_deref()),
        fields.price,
        fields.image_url,
        fields.is_available,
        fields.display_order,
        fields.translations,
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(respond(&pool, id, notice).await?)))
}

/// A replaced photo is deleted from storage: rentals have no preview that
/// could still show it.
pub async fn update_rental(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    State(translator): State<TranslationConfig>,
    Path(id): Path<i64>,
    Json(mut payload): Json<UpdateRentalRequest>,
) -> Result<Json<AdminRentalResponse>, AppError> {
    let current = load_rental(&pool, id).await?;
    if current.deleted_at.is_some() {
        return Err(AppError::NotFound);
    }
    validate_rental(&payload.fields, &media, current.image_url.as_deref())?;
    let notice = translate_rental(&translator, &pool, &mut payload.fields).await?;
    let fields = &payload.fields;

    let previous_image = sqlx::query_scalar!(
        r#"
        UPDATE rentals r
        SET name = $3, description = $4, price = $5, image_url = $6, is_available = $7,
            display_order = $8, translations = $9, updated_at = now()
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
        fields.translations,
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

    Ok(Json(respond(&pool, id, notice).await?))
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

    Ok(Json(respond(&pool, id, None).await?))
}
