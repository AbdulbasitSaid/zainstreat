//! Public events and bookings (Phase 31 Decisions 4-11, 14, 17). The
//! private address never leaves here unless the booking is confirmed.

use crate::email::EmailConfig;
use crate::error::{AppError, FieldError};
use crate::event_options::{label_in, parse_groups, resolve_choices};
use crate::events::{self, BookingEvent, localized, seats_taken};
use crate::order_emails::Locale;
use crate::payment::PaymentConfig;
use crate::push::PushConfig;
use crate::routes::orders::{OrderResponse, load_order_by_token, parse_token};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LocaleQuery {
    pub locale: Option<String>,
}

fn query_locale(query: &LocaleQuery) -> Locale {
    query.locale.as_deref().and_then(Locale::parse).unwrap_or(Locale::En)
}

#[derive(Debug, Serialize)]
pub struct EventCard {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub area: String,
    pub cover_image_url: Option<String>,
    pub capacity: i32,
    pub seats_left: i64,
    pub rsvp_open: bool,
    pub has_gallery: bool,
}

#[derive(Debug, Serialize)]
pub struct EventListResponse {
    pub upcoming: Vec<EventCard>,
    pub past: Vec<EventCard>,
}

/// Published, unarchived events (Decision 12). "Past" is worked out from
/// the clock, never stored.
pub async fn list_events(
    State(pool): State<PgPool>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<EventListResponse>, AppError> {
    let locale = query_locale(&query);
    let rows = sqlx::query!(
        r#"
        SELECT e.slug, e.content, e.area, e.cover_image_url, e.capacity,
               e.starts_at as "starts_at: DateTime<Utc>",
               e.ends_at as "ends_at: DateTime<Utc>",
               (now() >= COALESCE(e.ends_at, e.starts_at)) as "is_past!",
               (now() < COALESCE(e.rsvp_closes_at, e.starts_at)) as "rsvp_open!",
               (SELECT COALESCE(SUM(b.party_size), 0) FROM event_bookings b
                 WHERE b.event_id = e.id AND b.status IN ('awaiting_payment', 'confirmed'))::BIGINT as "taken!",
               EXISTS (SELECT 1 FROM event_photos p WHERE p.event_id = e.id) as "has_gallery!"
        FROM events e
        WHERE e.published_at IS NOT NULL AND e.archived_at IS NULL
        ORDER BY e.starts_at
        "#
    )
    .fetch_all(&pool)
    .await?;

    let mut upcoming = Vec::new();
    let mut past = Vec::new();
    for row in rows {
        let card = EventCard {
            title: localized(&row.content, locale, "title"),
            summary: localized(&row.content, locale, "summary"),
            slug: row.slug,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            area: row.area,
            cover_image_url: row.cover_image_url,
            capacity: row.capacity,
            seats_left: (i64::from(row.capacity) - row.taken).max(0),
            rsvp_open: row.rsvp_open,
            has_gallery: row.has_gallery,
        };
        if row.is_past { past.push(card) } else { upcoming.push(card) }
    }
    past.reverse();
    Ok(Json(EventListResponse { upcoming, past }))
}

#[derive(Debug, Serialize)]
pub struct PublicPlate {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub options: Vec<PublicOptionGroup>,
}

/// A plate's option group in the page's language (Phase 32 Decision 8).
#[derive(Debug, Serialize)]
pub struct PublicOptionGroup {
    pub key: String,
    pub label: String,
    pub choices: Vec<PublicOptionChoice>,
}

#[derive(Debug, Serialize)]
pub struct PublicOptionChoice {
    pub key: String,
    pub label: String,
}

fn public_options(options: &serde_json::Value, locale: Locale) -> Vec<PublicOptionGroup> {
    parse_groups(options)
        .into_iter()
        .map(|group| PublicOptionGroup {
            label: label_in(&group.label, locale),
            choices: group
                .choices
                .into_iter()
                .map(|choice| PublicOptionChoice { label: label_in(&choice.label, locale), key: choice.key })
                .collect(),
            key: group.key,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct PublicPhoto {
    pub image_url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicEvent {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub story: String,
    pub terms: String,
    pub recap: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub area: String,
    pub cover_image_url: Option<String>,
    pub capacity: i32,
    pub max_party_size: i32,
    pub seats_left: i64,
    pub meal_orders_close_at: Option<DateTime<Utc>>,
    pub meals_open: bool,
    pub rsvp_open: bool,
    pub is_past: bool,
    pub published: bool,
    pub plates: Vec<PublicPlate>,
    pub photos: Vec<PublicPhoto>,
}

/// One event as the public page shows it. `include_unpublished` is only
/// for the admin's preview (Decision 18).
pub(crate) async fn load_public_event(
    pool: &PgPool,
    slug: &str,
    locale: Locale,
    include_unpublished: bool,
) -> Result<PublicEvent, AppError> {
    let event = sqlx::query!(
        r#"
        SELECT e.id, e.slug, e.content, e.area, e.cover_image_url, e.capacity, e.max_party_size,
               e.starts_at as "starts_at: DateTime<Utc>",
               e.ends_at as "ends_at: DateTime<Utc>",
               e.meal_orders_close_at as "meal_orders_close_at: DateTime<Utc>",
               (now() >= COALESCE(e.ends_at, e.starts_at)) as "is_past!",
               (now() < COALESCE(e.rsvp_closes_at, e.starts_at)) as "rsvp_open!",
               (now() < COALESCE(e.meal_orders_close_at, e.rsvp_closes_at, e.starts_at)) as "meals_open!",
               (e.published_at IS NOT NULL) as "published!",
               (SELECT COALESCE(SUM(b.party_size), 0) FROM event_bookings b
                 WHERE b.event_id = e.id AND b.status IN ('awaiting_payment', 'confirmed'))::BIGINT as "taken!"
        FROM events e
        WHERE e.slug = $1 AND e.archived_at IS NULL
          AND ($2 OR e.published_at IS NOT NULL)
        "#,
        slug,
        include_unpublished,
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let plates = sqlx::query!(
        "SELECT id, content, price, options FROM event_plates WHERE event_id = $1 AND is_available ORDER BY position, id",
        event.id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|plate| PublicPlate {
        id: plate.id,
        name: localized(&plate.content, locale, "name"),
        description: localized(&plate.content, locale, "description"),
        price: plate.price,
        options: public_options(&plate.options, locale),
    })
    .collect();

    let photos = sqlx::query_as!(
        PublicPhoto,
        "SELECT image_url, caption FROM event_photos WHERE event_id = $1 ORDER BY position, id",
        event.id
    )
    .fetch_all(pool)
    .await?;

    Ok(PublicEvent {
        title: localized(&event.content, locale, "title"),
        summary: localized(&event.content, locale, "summary"),
        story: localized(&event.content, locale, "story"),
        terms: localized(&event.content, locale, "terms"),
        recap: localized(&event.content, locale, "recap"),
        slug: event.slug,
        starts_at: event.starts_at,
        ends_at: event.ends_at,
        area: event.area,
        cover_image_url: event.cover_image_url,
        capacity: event.capacity,
        max_party_size: event.max_party_size,
        seats_left: (i64::from(event.capacity) - event.taken).max(0),
        meal_orders_close_at: event.meal_orders_close_at,
        meals_open: event.meals_open,
        rsvp_open: event.rsvp_open,
        is_past: event.is_past,
        published: event.published,
        plates,
        photos,
    })
}

pub async fn get_event(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<PublicEvent>, AppError> {
    Ok(Json(load_public_event(&pool, &slug, query_locale(&query), false).await?))
}

#[derive(Debug, Deserialize)]
pub struct CreateBookingRequest {
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    /// One entry per person (Decision 17).
    pub guests: Vec<GuestInput>,
    pub notes: Option<String>,
    pub locale: Option<String>,
    #[serde(default)]
    pub accept_terms: bool,
    /// Set by the form's "Join the waitlist" retry after a 409 (Decision 7).
    #[serde(default)]
    pub join_waitlist: bool,
    /// Honeypot (Phase 15): real visitors never fill it.
    #[serde(default)]
    pub website: String,
}

/// A guest's plate and its option picks (Phase 32). A bare plate id or
/// `null` is the Phase 31 shape, still accepted from a cached page.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GuestInput {
    WithChoices {
        plate_id: Option<i64>,
        #[serde(default)]
        choices: BTreeMap<String, String>,
    },
    PlateOnly(Option<i64>),
}

impl GuestInput {
    fn plate_id(&self) -> Option<i64> {
        match self {
            GuestInput::WithChoices { plate_id, .. } => *plate_id,
            GuestInput::PlateOnly(plate_id) => *plate_id,
        }
    }

    fn choices(&self) -> BTreeMap<String, String> {
        match self {
            GuestInput::WithChoices { choices, .. } => choices.clone(),
            GuestInput::PlateOnly(_) => BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateBookingResponse {
    pub token: Uuid,
    pub status: String,
}

const NAME_MIN_LENGTH: usize = 2;
const NOTES_MAX_LENGTH: usize = 500;

fn field_error(field: &str, message: &str) -> FieldError {
    FieldError { field: field.into(), message: message.into() }
}

fn validate_contact(payload: &CreateBookingRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();
    let name = payload.customer_name.trim();
    if name.is_empty() {
        errors.push(field_error("customer_name", "required"));
    } else if name.chars().count() < NAME_MIN_LENGTH || !name.chars().any(char::is_alphabetic) {
        errors.push(field_error("customer_name", "invalid"));
    }
    if payload.customer_email.trim().is_empty() {
        errors.push(field_error("customer_email", "required"));
    } else if !is_valid_email(&payload.customer_email) {
        errors.push(field_error("customer_email", "invalid"));
    }
    let phone = payload.customer_phone.trim();
    if phone.is_empty() {
        errors.push(field_error("customer_phone", "required"));
    } else if !is_valid_phone(phone) {
        errors.push(field_error("customer_phone", "invalid"));
    }
    if payload.notes.as_deref().is_some_and(|notes| notes.chars().count() > NOTES_MAX_LENGTH) {
        errors.push(field_error("notes", "too_long"));
    }
    if payload.locale.as_deref().is_some_and(|locale| Locale::parse(locale).is_none()) {
        errors.push(field_error("locale", "invalid"));
    }
    if payload.guests.is_empty() {
        errors.push(field_error("guests", "empty"));
    }
    errors
}

/// One form for the RSVP and the meals (Decisions 4-9, 17). The event row
/// is locked while the seats are counted, so two guests can't both take
/// the last seats.
pub async fn create_booking(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    Path(slug): Path<String>,
    Json(payload): Json<CreateBookingRequest>,
) -> Result<(StatusCode, Json<CreateBookingResponse>), AppError> {
    // Same fake success as the enquiry forms, so a bot learns nothing.
    if !payload.website.trim().is_empty() {
        tracing::warn!("event booking honeypot triggered");
        return Ok((
            StatusCode::CREATED,
            Json(CreateBookingResponse { token: Uuid::new_v4(), status: "awaiting_payment".into() }),
        ));
    }

    let mut errors = validate_contact(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let mut tx = pool.begin().await?;

    let event = sqlx::query!(
        r#"
        SELECT id, capacity, max_party_size,
               (now() < COALESCE(rsvp_closes_at, starts_at)) as "rsvp_open!",
               (now() < COALESCE(meal_orders_close_at, rsvp_closes_at, starts_at)) as "meals_open!"
        FROM events
        WHERE slug = $1 AND published_at IS NOT NULL AND archived_at IS NULL
        FOR UPDATE
        "#,
        slug
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    if !event.rsvp_open {
        return Err(AppError::BookingClosed);
    }

    let locale = payload.locale.as_deref().and_then(Locale::parse).unwrap_or(Locale::En);
    let plates = sqlx::query!(
        "SELECT id, content, price, options FROM event_plates WHERE event_id = $1 AND is_available",
        event.id
    )
    .fetch_all(&mut *tx)
    .await?;

    if payload.guests.len() > event.max_party_size as usize {
        errors.push(field_error("guests", "too_many"));
    }
    // (plate_id, name, price, choices) per guest, in form order.
    let mut guests = Vec::with_capacity(payload.guests.len());
    for (index, guest) in payload.guests.iter().enumerate() {
        let picked = guest.choices();
        match guest.plate_id() {
            None if picked.is_empty() => guests.push((None, None, None, Vec::new())),
            None => errors.push(field_error(&format!("guests[{index}].choices"), "unknown_option")),
            Some(plate_id) => match plates.iter().find(|plate| plate.id == plate_id) {
                Some(plate) => match resolve_choices(&parse_groups(&plate.options), &picked) {
                    Ok(choices) => guests.push((
                        Some(plate.id),
                        // English, the owner's language, so admin counts and
                        // order lines group one plate under one name; guests
                        // see it translated from the plate itself.
                        Some(localized(&plate.content, Locale::En, "name")),
                        Some(plate.price),
                        choices,
                    )),
                    Err(message) => errors.push(field_error(&format!("guests[{index}].choices"), message)),
                },
                None => errors.push(field_error(&format!("guests[{index}]"), "unavailable")),
            },
        }
    }
    let wants_meals = guests.iter().any(|(plate_id, ..)| plate_id.is_some());
    if wants_meals && !event.meals_open {
        errors.push(field_error("guests", "meals_closed"));
    }
    if wants_meals && !payload.accept_terms {
        errors.push(field_error("accept_terms", "required"));
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let party_size = guests.len() as i32;
    let seats_left = i64::from(event.capacity) - seats_taken(&mut tx, event.id).await?;
    let fits = i64::from(party_size) <= seats_left;
    if !fits && !payload.join_waitlist {
        return Err(AppError::NotEnoughSeats(seats_left.max(0)));
    }

    let booking = sqlx::query!(
        r#"
        INSERT INTO event_bookings
            (event_id, status, customer_name, customer_email, customer_phone, party_size, notes, locale)
        VALUES ($1, 'waitlisted', $2, $3, $4, $5, $6, $7)
        RETURNING id, public_token
        "#,
        event.id,
        payload.customer_name.trim(),
        payload.customer_email.trim(),
        payload.customer_phone.trim(),
        party_size,
        payload.notes.as_deref().map(str::trim).filter(|notes| !notes.is_empty()),
        locale.as_str(),
    )
    .fetch_one(&mut *tx)
    .await?;

    for (position, (plate_id, plate_name, unit_price, choices)) in guests.into_iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO event_booking_guests (booking_id, position, plate_id, plate_name, unit_price, choices)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            booking.id,
            position as i32,
            plate_id,
            plate_name,
            unit_price,
            serde_json::to_value(&choices).expect("choices serialize"),
        )
        .execute(&mut *tx)
        .await?;
    }

    // A waitlisted booking holds no seats and has no order until the owner
    // offers it seats (Decision 7).
    let status = if !fits {
        "waitlisted"
    } else if wants_meals {
        events::start_payment(&mut tx, booking.id).await?;
        "awaiting_payment"
    } else {
        events::confirm(&mut tx, booking.id).await?;
        "confirmed"
    };

    tx.commit().await?;
    tracing::info!(booking_id = booking.id, event_id = event.id, status, "event booking created");

    events::notify(&pool, &email, &push, &payment, booking.id, BookingEvent::Created).await;

    Ok((
        StatusCode::CREATED,
        Json(CreateBookingResponse { token: booking.public_token, status: status.into() }),
    ))
}

#[derive(Debug, Serialize)]
pub struct BookingEventSummary {
    pub slug: String,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub area: String,
    pub cover_image_url: Option<String>,
    pub terms: String,
}

#[derive(Debug, Serialize)]
pub struct BookingGuest {
    pub plate_name: Option<String>,
    /// "Fried plantain, Chicken"; empty when the plate has no options.
    pub choices: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub unit_price: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct PublicBooking {
    pub token: Uuid,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub customer_name: String,
    pub party_size: i32,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub hold_expires_at: Option<DateTime<Utc>>,
    pub guest_can_cancel: bool,
    pub event: BookingEventSummary,
    pub guests: Vec<BookingGuest>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    /// Only once the booking is confirmed (Decision 10).
    pub address: Option<String>,
    /// The meals' order, with the invoice and payment instructions.
    pub order: Option<OrderResponse>,
}

async fn booking_id_for_token(pool: &PgPool, token: &str) -> Result<i64, AppError> {
    let token = parse_token(token)?;
    sqlx::query_scalar!("SELECT id FROM event_bookings WHERE public_token = $1", token)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn load_public_booking(
    pool: &PgPool,
    payment: &PaymentConfig,
    booking_id: i64,
    locale: Option<Locale>,
) -> Result<PublicBooking, AppError> {
    let booking = events::load_booking(pool, booking_id).await?.ok_or(AppError::NotFound)?;
    let locale = locale.unwrap_or(booking.locale);
    let order = match booking.order_token {
        Some(token) => Some(load_order_by_token(pool, payment, token).await?),
        None => None,
    };
    let confirmed = booking.status == "confirmed";
    Ok(PublicBooking {
        token: booking.token,
        status: booking.status,
        cancel_reason: booking.cancel_reason,
        customer_name: booking.customer_name,
        party_size: booking.party_size,
        notes: booking.notes,
        created_at: booking.created_at,
        hold_expires_at: booking.hold_expires_at,
        guest_can_cancel: booking.guest_can_cancel,
        event: BookingEventSummary {
            title: localized(&booking.event_content, locale, "title"),
            terms: localized(&booking.event_content, locale, "terms"),
            slug: booking.event_slug,
            starts_at: booking.starts_at,
            area: booking.area,
            cover_image_url: booking.cover_image_url,
        },
        guests: booking
            .guests
            .iter()
            .map(|guest| BookingGuest {
                plate_name: guest.name_in(locale),
                choices: guest.choices_in(locale),
                unit_price: guest.unit_price,
            })
            .collect(),
        total: booking.total,
        address: confirmed.then_some(booking.address),
        order,
    })
}

pub async fn get_booking(
    State(pool): State<PgPool>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<PublicBooking>, AppError> {
    let booking_id = booking_id_for_token(&pool, &token).await?;
    let locale = query.locale.as_deref().and_then(Locale::parse);
    Ok(Json(load_public_booking(&pool, &payment, booking_id, locale).await?))
}

/// The guest's own cancellation, until the meal cutoff (Decision 14).
/// Idempotent: cancelling a cancelled booking returns it unchanged.
pub async fn cancel_booking(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<PublicBooking>, AppError> {
    let booking_id = booking_id_for_token(&pool, &token).await?;
    let locale = query.locale.as_deref().and_then(Locale::parse);

    let mut tx = pool.begin().await?;
    let current = sqlx::query!(
        r#"
        SELECT b.status, (now() < COALESCE(e.meal_orders_close_at, e.starts_at)) as "can_cancel!"
        FROM event_bookings b JOIN events e ON e.id = b.event_id
        WHERE b.id = $1
        FOR UPDATE OF b
        "#,
        booking_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if current.status == "cancelled" {
        drop(tx);
        return Ok(Json(load_public_booking(&pool, &payment, booking_id, locale).await?));
    }
    if !current.can_cancel {
        return Err(AppError::InvalidStatusTransition);
    }
    events::cancel(&mut tx, booking_id, "guest").await?;
    tx.commit().await?;
    tracing::info!(booking_id, "guest cancelled event booking");

    events::notify(&pool, &email, &push, &payment, booking_id, BookingEvent::CancelledByGuest).await;
    Ok(Json(load_public_booking(&pool, &payment, booking_id, locale).await?))
}
