//! The admin Events panel (Phase 31 Decisions 7, 12, 13, 18). Times go in
//! and out as Europe/Amsterdam wall-clock strings (`YYYY-MM-DDTHH:MM`, what
//! a `datetime-local` input uses), converted in SQL, so the browser's own
//! time zone never matters.

use crate::email::EmailConfig;
use crate::error::{AppError, FieldError};
use crate::event_options::{OptionGroup, choices_summary, normalize_groups, parse_choices, validate_groups};
use crate::events::{self, BookingEvent, localized, seats_taken, slugify};
use crate::order_emails::Locale;
use crate::payment::{PaymentConfig, payment_reference};
use crate::push::PushConfig;
use crate::routes::events::{LocaleQuery, PublicEvent, load_public_event};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

const LOCAL_FORMAT: &str = "%Y-%m-%dT%H:%M";
const MAX_PHOTOS: usize = 30;
const MAX_PLATES: usize = 12;

fn field_error(field: &str, message: &str) -> FieldError {
    FieldError { field: field.into(), message: message.into() }
}

#[derive(Debug, Serialize)]
pub struct AdminEventSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub published: bool,
    pub archived: bool,
    pub is_past: bool,
    pub capacity: i32,
    pub seats_taken: i64,
    pub waitlist_count: i64,
    #[serde(with = "rust_decimal::serde::str")]
    pub paid_total: Decimal,
}

pub async fn list_events(State(pool): State<PgPool>) -> Result<Json<Vec<AdminEventSummary>>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT e.id, e.slug, e.content, e.capacity,
               e.starts_at as "starts_at: DateTime<Utc>",
               (e.published_at IS NOT NULL) as "published!",
               (e.archived_at IS NOT NULL) as "archived!",
               (now() >= COALESCE(e.ends_at, e.starts_at)) as "is_past!",
               (SELECT COALESCE(SUM(b.party_size), 0) FROM event_bookings b
                 WHERE b.event_id = e.id AND b.status IN ('awaiting_payment', 'confirmed'))::BIGINT as "seats_taken!",
               (SELECT COUNT(*) FROM event_bookings b
                 WHERE b.event_id = e.id AND b.status = 'waitlisted') as "waitlist_count!",
               (SELECT COALESCE(SUM(o.total), 0) FROM event_bookings b JOIN orders o ON o.id = b.order_id
                 WHERE b.event_id = e.id AND o.status IN ('processing', 'completed')) as "paid_total!"
        FROM events e
        ORDER BY (now() >= COALESCE(e.ends_at, e.starts_at)),
                 CASE WHEN now() < COALESCE(e.ends_at, e.starts_at) THEN e.starts_at END ASC,
                 e.starts_at DESC
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| AdminEventSummary {
                id: row.id,
                title: localized(&row.content, Locale::En, "title"),
                slug: row.slug,
                starts_at: row.starts_at,
                published: row.published,
                archived: row.archived,
                is_past: row.is_past,
                capacity: row.capacity,
                seats_taken: row.seats_taken,
                waitlist_count: row.waitlist_count,
                paid_total: row.paid_total,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct PlateInput {
    pub id: Option<i64>,
    /// `{"en": {"name", "description"}, "nl": …, "fr": …}`
    pub content: Value,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub is_available: bool,
    /// Option groups (Phase 32 Decision 7). New groups and choices come
    /// without a key and get one from their English label.
    #[serde(default)]
    pub options: Vec<OptionGroup>,
}

#[derive(Debug, Deserialize)]
pub struct EventInput {
    pub slug: Option<String>,
    /// `{"en": {"title", "summary", "story", "terms", "recap"}, "nl": …, "fr": …}`
    pub content: Value,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub area: String,
    pub address: String,
    pub capacity: i32,
    pub max_party_size: i32,
    pub meal_orders_close_at: Option<String>,
    pub rsvp_closes_at: Option<String>,
    pub payment_hold_hours: i32,
    pub cover_image_url: Option<String>,
    pub plates: Vec<PlateInput>,
}

const CONTENT_KEYS: [&str; 5] = ["title", "summary", "story", "terms", "recap"];
const PLATE_KEYS: [&str; 2] = ["name", "description"];

/// Keeps only the known locales and string keys, so the stored document
/// always has the shape `localized` reads.
fn clean_content(content: &Value, keys: &[&str]) -> Value {
    let mut cleaned = serde_json::Map::new();
    for locale in Locale::ALL {
        let mut fields = serde_json::Map::new();
        for key in keys {
            let text = content
                .get(locale.as_str())
                .and_then(|fields| fields.get(*key))
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            fields.insert((*key).to_string(), Value::String(text.to_string()));
        }
        cleaned.insert(locale.as_str().to_string(), Value::Object(fields));
    }
    Value::Object(cleaned)
}

fn blank_to_none(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
}

fn parse_local(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), LOCAL_FORMAT).ok()
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 80
        && slug.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn validate(input: &EventInput) -> Vec<FieldError> {
    let mut errors = Vec::new();
    let content = clean_content(&input.content, &CONTENT_KEYS);
    if localized(&content, Locale::En, "title").is_empty() {
        errors.push(field_error("content.en.title", "required"));
    }
    let starts_at = parse_local(&input.starts_at);
    if starts_at.is_none() {
        errors.push(field_error("starts_at", "invalid"));
    }
    for (field, value) in [
        ("ends_at", &input.ends_at),
        ("meal_orders_close_at", &input.meal_orders_close_at),
        ("rsvp_closes_at", &input.rsvp_closes_at),
    ] {
        if let Some(value) = blank_to_none(value) {
            match (parse_local(&value), starts_at) {
                (None, _) => errors.push(field_error(field, "invalid")),
                (Some(at), Some(start)) if field == "ends_at" && at <= start => {
                    errors.push(field_error(field, "before_start"))
                }
                (Some(at), Some(start)) if field != "ends_at" && at > start => {
                    errors.push(field_error(field, "after_start"))
                }
                _ => {}
            }
        }
    }
    if input.area.trim().is_empty() {
        errors.push(field_error("area", "required"));
    }
    if input.address.trim().is_empty() {
        errors.push(field_error("address", "required"));
    }
    if !(1..=1000).contains(&input.capacity) {
        errors.push(field_error("capacity", "out_of_range"));
    }
    if !(1..=50).contains(&input.max_party_size) {
        errors.push(field_error("max_party_size", "out_of_range"));
    }
    if !(1..=336).contains(&input.payment_hold_hours) {
        errors.push(field_error("payment_hold_hours", "out_of_range"));
    }
    if let Some(slug) = blank_to_none(&input.slug)
        && !is_valid_slug(&slug)
    {
        errors.push(field_error("slug", "invalid"));
    }
    if input.plates.len() > MAX_PLATES {
        errors.push(field_error("plates", "too_many"));
    }
    for (index, plate) in input.plates.iter().enumerate() {
        let content = clean_content(&plate.content, &PLATE_KEYS);
        if localized(&content, Locale::En, "name").is_empty() {
            errors.push(field_error(&format!("plates[{index}].name"), "required"));
        }
        if plate.price < Decimal::ZERO || plate.price.scale() > 2 {
            errors.push(field_error(&format!("plates[{index}].price"), "invalid"));
        }
        for (field, message) in validate_groups(&normalize_groups(&plate.options)) {
            errors.push(field_error(&format!("plates[{index}].{field}"), message));
        }
    }
    errors
}

#[derive(Debug, Serialize)]
pub struct AdminPlate {
    pub id: i64,
    pub content: Value,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub is_available: bool,
    pub options: Value,
}

#[derive(Debug, Serialize)]
pub struct AdminPhoto {
    pub id: i64,
    pub image_url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminEvent {
    pub id: i64,
    pub slug: String,
    pub content: Value,
    /// Wall-clock Europe/Amsterdam, `YYYY-MM-DDTHH:MM`.
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub meal_orders_close_at: Option<String>,
    pub rsvp_closes_at: Option<String>,
    pub starts_at_utc: DateTime<Utc>,
    pub area: String,
    pub address: String,
    pub capacity: i32,
    pub max_party_size: i32,
    pub payment_hold_hours: i32,
    pub cover_image_url: Option<String>,
    pub published: bool,
    pub archived: bool,
    pub is_past: bool,
    /// The slug is locked once anyone has booked (Decision 18).
    pub has_bookings: bool,
    pub seats_taken: i64,
    pub plates: Vec<AdminPlate>,
    pub photos: Vec<AdminPhoto>,
}

async fn load_admin_event(pool: &PgPool, id: i64) -> Result<AdminEvent, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT e.id, e.slug, e.content, e.area, e.address, e.capacity, e.max_party_size,
               e.payment_hold_hours, e.cover_image_url,
               to_char(e.starts_at AT TIME ZONE 'Europe/Amsterdam', 'YYYY-MM-DD"T"HH24:MI') as "starts_at!",
               to_char(e.ends_at AT TIME ZONE 'Europe/Amsterdam', 'YYYY-MM-DD"T"HH24:MI') as ends_at,
               to_char(e.meal_orders_close_at AT TIME ZONE 'Europe/Amsterdam', 'YYYY-MM-DD"T"HH24:MI') as meal_orders_close_at,
               to_char(e.rsvp_closes_at AT TIME ZONE 'Europe/Amsterdam', 'YYYY-MM-DD"T"HH24:MI') as rsvp_closes_at,
               e.starts_at as "starts_at_utc: DateTime<Utc>",
               (e.published_at IS NOT NULL) as "published!",
               (e.archived_at IS NOT NULL) as "archived!",
               (now() >= COALESCE(e.ends_at, e.starts_at)) as "is_past!",
               EXISTS (SELECT 1 FROM event_bookings b WHERE b.event_id = e.id) as "has_bookings!"
        FROM events e
        WHERE e.id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let plates = sqlx::query_as!(
        AdminPlate,
        "SELECT id, content, price, is_available, options FROM event_plates WHERE event_id = $1 ORDER BY position, id",
        id
    )
    .fetch_all(pool)
    .await?;
    let photos = sqlx::query_as!(
        AdminPhoto,
        "SELECT id, image_url, caption FROM event_photos WHERE event_id = $1 ORDER BY position, id",
        id
    )
    .fetch_all(pool)
    .await?;
    let mut conn = pool.acquire().await?;
    let seats_taken = seats_taken(&mut conn, id).await?;

    Ok(AdminEvent {
        id: row.id,
        slug: row.slug,
        content: row.content,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        meal_orders_close_at: row.meal_orders_close_at,
        rsvp_closes_at: row.rsvp_closes_at,
        starts_at_utc: row.starts_at_utc,
        area: row.area,
        address: row.address,
        capacity: row.capacity,
        max_party_size: row.max_party_size,
        payment_hold_hours: row.payment_hold_hours,
        cover_image_url: row.cover_image_url,
        published: row.published,
        archived: row.archived,
        is_past: row.is_past,
        has_bookings: row.has_bookings,
        seats_taken,
        plates,
        photos,
    })
}

pub async fn get_event(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<Json<AdminEvent>, AppError> {
    Ok(Json(load_admin_event(&pool, id).await?))
}

/// Replaces the plate list in input order. A removed plate that a guest
/// already chose is only made unavailable, so the booking keeps its link.
async fn save_plates(
    tx: &mut sqlx::PgConnection,
    event_id: i64,
    plates: &[PlateInput],
) -> Result<(), AppError> {
    let mut kept = Vec::with_capacity(plates.len());
    for (position, plate) in plates.iter().enumerate() {
        let content = clean_content(&plate.content, &PLATE_KEYS);
        let options = serde_json::to_value(normalize_groups(&plate.options)).expect("options serialize");
        let id = match plate.id {
            Some(id) => sqlx::query_scalar!(
                r#"
                UPDATE event_plates SET content = $3, price = $4, is_available = $5, position = $6, options = $7
                WHERE id = $1 AND event_id = $2
                RETURNING id
                "#,
                id,
                event_id,
                content,
                plate.price,
                plate.is_available,
                position as i32,
                options,
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::Validation(vec![field_error(&format!("plates[{position}].id"), "not_found")]))?,
            None => sqlx::query_scalar!(
                r#"
                INSERT INTO event_plates (event_id, position, content, price, is_available, options)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
                "#,
                event_id,
                position as i32,
                content,
                plate.price,
                plate.is_available,
                options,
            )
            .fetch_one(&mut *tx)
            .await?,
        };
        kept.push(id);
    }

    sqlx::query!(
        r#"
        UPDATE event_plates SET is_available = false
        WHERE event_id = $1 AND id <> ALL($2)
          AND EXISTS (SELECT 1 FROM event_booking_guests g WHERE g.plate_id = event_plates.id)
        "#,
        event_id,
        &kept,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"
        DELETE FROM event_plates
        WHERE event_id = $1 AND id <> ALL($2)
          AND NOT EXISTS (SELECT 1 FROM event_booking_guests g WHERE g.plate_id = event_plates.id)
        "#,
        event_id,
        &kept,
    )
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// A free slug: `base`, then `base-2`, `base-3`, …
async fn free_slug(tx: &mut sqlx::PgConnection, base: &str, except_id: Option<i64>) -> Result<String, sqlx::Error> {
    let mut candidate = base.to_string();
    let mut n = 2;
    loop {
        let taken = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM events WHERE slug = $1 AND id IS DISTINCT FROM $2) as "taken!""#,
            candidate,
            except_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        if !taken {
            return Ok(candidate);
        }
        candidate = format!("{base}-{n}");
        n += 1;
    }
}

/// New events start as unpublished drafts (Decision 12).
pub async fn create_event(
    State(pool): State<PgPool>,
    Json(input): Json<EventInput>,
) -> Result<(StatusCode, Json<AdminEvent>), AppError> {
    let errors = validate(&input);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let content = clean_content(&input.content, &CONTENT_KEYS);
    let base = blank_to_none(&input.slug).unwrap_or_else(|| slugify(&localized(&content, Locale::En, "title")));

    let mut tx = pool.begin().await?;
    let slug = free_slug(&mut tx, &base, None).await?;
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO events
            (slug, content, starts_at, ends_at, area, address, capacity, max_party_size,
             meal_orders_close_at, rsvp_closes_at, payment_hold_hours, cover_image_url)
        VALUES ($1, $2,
                $3::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
                $4::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
                $5, $6, $7, $8,
                $9::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
                $10::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
                $11, $12)
        RETURNING id
        "#,
        slug,
        content,
        input.starts_at.trim(),
        blank_to_none(&input.ends_at),
        input.area.trim(),
        input.address.trim(),
        input.capacity,
        input.max_party_size,
        blank_to_none(&input.meal_orders_close_at),
        blank_to_none(&input.rsvp_closes_at),
        input.payment_hold_hours,
        blank_to_none(&input.cover_image_url),
    )
    .fetch_one(&mut *tx)
    .await?;
    save_plates(&mut tx, id, &input.plates).await?;
    tx.commit().await?;
    tracing::info!(event_id = id, %slug, "admin created event");

    Ok((StatusCode::CREATED, Json(load_admin_event(&pool, id).await?)))
}

pub async fn update_event(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(input): Json<EventInput>,
) -> Result<Json<AdminEvent>, AppError> {
    let mut errors = validate(&input);
    let content = clean_content(&input.content, &CONTENT_KEYS);

    let mut tx = pool.begin().await?;
    let current = sqlx::query!(
        r#"
        SELECT slug, EXISTS (SELECT 1 FROM event_bookings b WHERE b.event_id = events.id) as "has_bookings!"
        FROM events WHERE id = $1 FOR UPDATE
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let requested = blank_to_none(&input.slug).unwrap_or_else(|| current.slug.clone());
    if requested != current.slug && current.has_bookings {
        errors.push(field_error("slug", "locked"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let slug = if requested == current.slug { requested } else { free_slug(&mut tx, &requested, Some(id)).await? };

    sqlx::query!(
        r#"
        UPDATE events SET
            slug = $2, content = $3,
            starts_at = $4::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
            ends_at = $5::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
            area = $6, address = $7, capacity = $8, max_party_size = $9,
            meal_orders_close_at = $10::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
            rsvp_closes_at = $11::text::timestamp AT TIME ZONE 'Europe/Amsterdam',
            payment_hold_hours = $12, cover_image_url = $13, updated_at = now()
        WHERE id = $1
        "#,
        id,
        slug,
        content,
        input.starts_at.trim(),
        blank_to_none(&input.ends_at),
        input.area.trim(),
        input.address.trim(),
        input.capacity,
        input.max_party_size,
        blank_to_none(&input.meal_orders_close_at),
        blank_to_none(&input.rsvp_closes_at),
        input.payment_hold_hours,
        blank_to_none(&input.cover_image_url),
    )
    .execute(&mut *tx)
    .await?;
    save_plates(&mut tx, id, &input.plates).await?;
    tx.commit().await?;
    tracing::info!(event_id = id, "admin updated event");

    Ok(Json(load_admin_event(&pool, id).await?))
}

async fn set_flags(pool: &PgPool, id: i64, published: Option<bool>, archived: Option<bool>) -> Result<Json<AdminEvent>, AppError> {
    let updated = sqlx::query_scalar!(
        r#"
        UPDATE events SET
            published_at = CASE WHEN $2::BOOLEAN IS NULL THEN published_at
                                WHEN $2 THEN COALESCE(published_at, now()) ELSE NULL END,
            archived_at = CASE WHEN $3::BOOLEAN IS NULL THEN archived_at
                               WHEN $3 THEN COALESCE(archived_at, now()) ELSE NULL END,
            updated_at = now()
        WHERE id = $1
        RETURNING id
        "#,
        id,
        published,
        archived,
    )
    .fetch_optional(pool)
    .await?;
    updated.ok_or(AppError::NotFound)?;
    Ok(Json(load_admin_event(pool, id).await?))
}

/// An event can't go live without the address confirmed guests receive
/// (Phase 32 Decision 12); a production seed leaves it blank.
pub async fn publish_event(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<Json<AdminEvent>, AppError> {
    let address = sqlx::query_scalar!("SELECT address FROM events WHERE id = $1", id)
        .fetch_optional(&pool)
        .await?
        .ok_or(AppError::NotFound)?;
    if address.trim().is_empty() {
        return Err(AppError::Validation(vec![field_error("address", "required")]));
    }
    set_flags(&pool, id, Some(true), None).await
}

pub async fn unpublish_event(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<Json<AdminEvent>, AppError> {
    set_flags(&pool, id, Some(false), None).await
}

pub async fn archive_event(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<Json<AdminEvent>, AppError> {
    set_flags(&pool, id, None, Some(true)).await
}

pub async fn unarchive_event(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<Json<AdminEvent>, AppError> {
    set_flags(&pool, id, None, Some(false)).await
}

#[derive(Debug, Deserialize)]
pub struct PhotoInput {
    pub image_url: String,
    pub caption: Option<String>,
}

/// Replaces the past-event gallery in input order (Decision 13).
pub async fn save_photos(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(photos): Json<Vec<PhotoInput>>,
) -> Result<Json<AdminEvent>, AppError> {
    if photos.len() > MAX_PHOTOS {
        return Err(AppError::Validation(vec![field_error("photos", "too_many")]));
    }
    if let Some(index) = photos.iter().position(|photo| photo.image_url.trim().is_empty()) {
        return Err(AppError::Validation(vec![field_error(&format!("photos[{index}].image_url"), "required")]));
    }
    let mut tx = pool.begin().await?;
    sqlx::query_scalar!("SELECT id FROM events WHERE id = $1 FOR UPDATE", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    sqlx::query!("DELETE FROM event_photos WHERE event_id = $1", id)
        .execute(&mut *tx)
        .await?;
    for (position, photo) in photos.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO event_photos (event_id, position, image_url, caption) VALUES ($1, $2, $3, $4)",
            id,
            position as i32,
            photo.image_url.trim(),
            blank_to_none(&photo.caption),
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(load_admin_event(&pool, id).await?))
}

#[derive(Debug, Serialize)]
pub struct AdminBookingGuest {
    pub plate_name: Option<String>,
    /// English, like the plate name: "Fried plantain, Chicken".
    pub choices: String,
}

#[derive(Debug, Serialize)]
pub struct AdminBooking {
    pub id: i64,
    pub token: Uuid,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub party_size: i32,
    pub notes: Option<String>,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub hold_expires_at: Option<DateTime<Utc>>,
    pub order_id: Option<i64>,
    pub order_status: Option<String>,
    pub payment_reference: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub guests: Vec<AdminBookingGuest>,
}

/// Plates to cook: paid or free-confirmed guests, and those still on an
/// unpaid hold, counted separately (Decision 18), per plate and choices
/// (Phase 32 Decision 6).
#[derive(Debug, Serialize)]
pub struct PlateCount {
    /// `None` is "no meal".
    pub plate_name: Option<String>,
    /// English; empty when the plate has no options.
    pub choices: String,
    pub confirmed: i64,
    pub held: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminBookingsResponse {
    pub bookings: Vec<AdminBooking>,
    pub plate_counts: Vec<PlateCount>,
    pub capacity: i32,
    pub seats_taken: i64,
}

async fn load_bookings(pool: &PgPool, event_id: i64) -> Result<AdminBookingsResponse, AppError> {
    let event = sqlx::query!("SELECT capacity FROM events WHERE id = $1", event_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let rows = sqlx::query!(
        r#"
        SELECT b.id, b.public_token, b.status, b.cancel_reason, b.customer_name, b.customer_email,
               b.customer_phone, b.party_size, b.notes, b.locale, b.order_id,
               b.created_at as "created_at: DateTime<Utc>",
               b.hold_expires_at as "hold_expires_at: DateTime<Utc>",
               o.status as "order_status?",
               (SELECT COALESCE(SUM(g.unit_price), 0) FROM event_booking_guests g WHERE g.booking_id = b.id) as "total!"
        FROM event_bookings b
        LEFT JOIN orders o ON o.id = b.order_id
        WHERE b.event_id = $1
        ORDER BY CASE b.status WHEN 'awaiting_payment' THEN 0 WHEN 'confirmed' THEN 1
                               WHEN 'waitlisted' THEN 2 ELSE 3 END,
                 b.created_at
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    let guests = sqlx::query!(
        r#"
        SELECT g.booking_id, g.plate_name, g.choices
        FROM event_booking_guests g JOIN event_bookings b ON b.id = g.booking_id
        WHERE b.event_id = $1
        ORDER BY g.booking_id, g.position
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    let bookings = rows
        .into_iter()
        .map(|row| AdminBooking {
            guests: guests
                .iter()
                .filter(|guest| guest.booking_id == row.id)
                .map(|guest| AdminBookingGuest {
                    plate_name: guest.plate_name.clone(),
                    choices: choices_summary(&parse_choices(&guest.choices), &[], Locale::En),
                })
                .collect(),
            id: row.id,
            token: row.public_token,
            status: row.status,
            cancel_reason: row.cancel_reason,
            customer_name: row.customer_name,
            customer_email: row.customer_email,
            customer_phone: row.customer_phone,
            party_size: row.party_size,
            notes: row.notes,
            locale: row.locale,
            created_at: row.created_at,
            hold_expires_at: row.hold_expires_at,
            payment_reference: row.order_id.map(payment_reference),
            order_id: row.order_id,
            order_status: row.order_status,
            total: row.total,
        })
        .collect();

    let plate_counts = sqlx::query!(
        r#"
        SELECT g.plate_name, g.choices,
               COUNT(*) FILTER (WHERE b.status = 'confirmed') as "confirmed!",
               COUNT(*) FILTER (WHERE b.status = 'awaiting_payment') as "held!"
        FROM event_booking_guests g JOIN event_bookings b ON b.id = g.booking_id
        WHERE b.event_id = $1 AND b.status IN ('confirmed', 'awaiting_payment')
        GROUP BY g.plate_name, g.choices
        ORDER BY g.plate_name NULLS LAST, g.choices::TEXT
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| PlateCount {
        plate_name: row.plate_name,
        choices: choices_summary(&parse_choices(&row.choices), &[], Locale::En),
        confirmed: row.confirmed,
        held: row.held,
    })
    .collect();

    let mut conn = pool.acquire().await?;
    Ok(AdminBookingsResponse {
        bookings,
        plate_counts,
        capacity: event.capacity,
        seats_taken: seats_taken(&mut conn, event_id).await?,
    })
}

pub async fn list_bookings(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminBookingsResponse>, AppError> {
    Ok(Json(load_bookings(&pool, id).await?))
}

fn csv_field(value: &str) -> String {
    // A leading =, +, - or @ would run as a formula in a spreadsheet.
    let value = if value.starts_with(['=', '+', '-', '@']) { format!("'{value}") } else { value.to_string() };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// The guest list for the day (Decision 18).
pub async fn bookings_csv(State(pool): State<PgPool>, Path(id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let data = load_bookings(&pool, id).await?;
    let slug = sqlx::query_scalar!("SELECT slug FROM events WHERE id = $1", id)
        .fetch_one(&pool)
        .await?;
    let mut csv = String::from("Name,Email,Phone,Party,Plates,Status,Order,Order status,Total,Note\n");
    for booking in &data.bookings {
        let plates = booking
            .guests
            .iter()
            .map(|guest| match (&guest.plate_name, guest.choices.as_str()) {
                (None, _) => "No meal".to_string(),
                (Some(name), "") => name.clone(),
                (Some(name), choices) => format!("{name} ({choices})"),
            })
            .collect::<Vec<_>>()
            .join("; ");
        let row = [
            booking.customer_name.clone(),
            booking.customer_email.clone(),
            booking.customer_phone.clone(),
            booking.party_size.to_string(),
            plates,
            booking.status.clone(),
            booking.payment_reference.clone().unwrap_or_default(),
            booking.order_status.clone().unwrap_or_default(),
            booking.total.to_string(),
            booking.notes.clone().unwrap_or_default(),
        ];
        csv.push_str(&row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(","));
        csv.push('\n');
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{slug}-guests.csv\"")),
        ],
        csv,
    ))
}

/// Gives seats to a waitlisted (or cancelled) booking if they fit now
/// (Decision 7, open risk 4). Ignores the meal cutoff: the owner chose to
/// cook for them.
pub async fn offer_seats(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    Path(booking_id): Path<i64>,
) -> Result<Json<AdminBookingsResponse>, AppError> {
    let mut tx = pool.begin().await?;
    let booking = sqlx::query!(
        "SELECT event_id, status, party_size FROM event_bookings WHERE id = $1 FOR UPDATE",
        booking_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if !matches!(booking.status.as_str(), "waitlisted" | "cancelled") {
        return Err(AppError::InvalidStatusTransition);
    }
    let capacity = sqlx::query_scalar!("SELECT capacity FROM events WHERE id = $1 FOR UPDATE", booking.event_id)
        .fetch_one(&mut *tx)
        .await?;
    let seats_left = i64::from(capacity) - seats_taken(&mut tx, booking.event_id).await?;
    if i64::from(booking.party_size) > seats_left {
        return Err(AppError::NotEnoughSeats(seats_left.max(0)));
    }
    if events::has_meals(&mut tx, booking_id).await? {
        events::start_payment(&mut tx, booking_id).await?;
    } else {
        events::confirm(&mut tx, booking_id).await?;
    }
    tx.commit().await?;
    tracing::info!(booking_id, "admin offered seats");

    events::notify(&pool, &email, &push, &payment, booking_id, BookingEvent::Offered).await;
    Ok(Json(load_bookings(&pool, booking.event_id).await?))
}

pub async fn cancel_booking(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    Path(booking_id): Path<i64>,
) -> Result<Json<AdminBookingsResponse>, AppError> {
    let mut tx = pool.begin().await?;
    let booking = sqlx::query!(
        "SELECT event_id, status FROM event_bookings WHERE id = $1 FOR UPDATE",
        booking_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if booking.status == "cancelled" {
        return Err(AppError::InvalidStatusTransition);
    }
    events::cancel(&mut tx, booking_id, "admin").await?;
    tx.commit().await?;
    tracing::info!(booking_id, "admin cancelled event booking");

    events::notify(&pool, &email, &push, &payment, booking_id, BookingEvent::CancelledByAdmin).await;
    Ok(Json(load_bookings(&pool, booking.event_id).await?))
}

/// The public page's data for an unpublished event, for the preview host
/// (Decision 18).
pub async fn preview_event(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<PublicEvent>, AppError> {
    let locale = query.locale.as_deref().and_then(Locale::parse).unwrap_or(Locale::En);
    Ok(Json(load_public_event(&pool, &slug, locale, true).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_and_defuses_formulas() {
        assert_eq!(csv_field("Amina"), "Amina");
        assert_eq!(csv_field("Abula; Jollof, extra"), "\"Abula; Jollof, extra\"");
        assert_eq!(csv_field("=HYPERLINK(1)"), "'=HYPERLINK(1)");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn slugs_are_validated() {
        assert!(is_valid_slug("the-abula-experience-2026"));
        assert!(!is_valid_slug("The-Abula"));
        assert!(!is_valid_slug("double--dash"));
        assert!(!is_valid_slug("-lead"));
    }
}
