use crate::delivery::{self, DeliveryConfig, DeliveryError, EstimatedBy};
use crate::email::{DeliveryRef, EmailConfig, OutgoingEmail};
use crate::error::{AppError, FieldError, UnavailableItem};
use crate::order_emails::{Locale, invoice_email};
use crate::order_status::CUSTOMER_COMPLETABLE;
use crate::payment::{PaymentConfig, PaymentInstructions, TikkieInstructions, payment_reference};
use crate::push::{self, PushConfig};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryType {
    Pickup,
    Delivery,
}

/// A menu item (optionally a price option) or a rental: exactly one of
/// the two ids (Phase 37 Decision 3).
#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub menu_item_id: Option<i64>,
    pub rental_id: Option<i64>,
    pub price_option_id: Option<i64>,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: DeliveryType,
    /// Only read when address lookup is down (Phase 37 Decision 17);
    /// otherwise the address comes from `delivery_address_id`.
    pub delivery_address: Option<String>,
    /// The PDOK id picked at checkout (Phase 37 Decision 12).
    pub delivery_address_id: Option<String>,
    /// Apartment, floor or instructions, appended to the looked-up address.
    pub delivery_address_extra: Option<String>,
    /// The fee checkout showed; a different server fee is a 409.
    pub expected_delivery_fee: Option<Decimal>,
    /// Required exactly when the order has rentals (Phase 37 Decision 4).
    pub rental_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    pub items: Vec<CreateOrderItemRequest>,
    /// The site language the customer ordered in, for their emails. Missing
    /// means `en` (Phase 22 Decision 5).
    pub locale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrderItemResponse {
    /// `menu` or `rental` (Phase 37 Decision 3).
    pub kind: String,
    pub item_name: String,
    pub option_label: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    pub quantity: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
}

type Timestamp = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: i64,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: String,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub delivery_fee: Decimal,
    /// Phase 37: how the fee was worked out, kept with the order.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub delivery_distance_km: Option<Decimal>,
    pub delivery_estimated_by: Option<String>,
    /// Placed while address lookup was down: no payment until the owner
    /// sets the fee (Decision 17).
    pub delivery_fee_pending: bool,
    pub rental_date: Option<chrono::NaiveDate>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<OrderItemResponse>,
    /// The unguessable key for `/order/{token}` (Phase 20 Decision 4).
    pub public_token: Uuid,
    pub payment_reference: String,
    /// `None` when no bank details are configured (Decision 5).
    pub payment: Option<PaymentInstructions>,
    /// When each tracking step was reached (Phase 22 Decision 4).
    pub payment_claimed_at: Option<Timestamp>,
    pub payment_confirmed_at: Option<Timestamp>,
    pub ready_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub cancelled_at: Option<Timestamp>,
    /// `customer` or `admin`, once completed.
    pub completed_by: Option<String>,
    /// For an `event` order: the booking it pays for (Phase 31 Decision 11).
    pub event_booking_token: Option<Uuid>,
    /// For an unpaid `event` order whose event has a valid Tikkie link
    /// (Phase 34 Decision 15). Separate from `payment`, so it shows even
    /// without bank details.
    pub tikkie: Option<TikkieInstructions>,
}

const NAME_MIN_LENGTH: usize = 2;
const ADDRESS_MIN_LENGTH: usize = 5;
const ADDRESS_EXTRA_MAX_LENGTH: usize = 120;
const NOTES_MAX_LENGTH: usize = 500;
const RENTAL_MAX_QUANTITY: i32 = 99;
const RENTAL_MAX_DAYS_AHEAD: i64 = 365;

fn field_error(field: impl Into<String>, message: &str) -> FieldError {
    FieldError { field: field.into(), message: message.into() }
}

fn validate(payload: &CreateOrderRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.customer_name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError {
            field: "customer_name".into(),
            message: "required".into(),
        });
    } else if trimmed_name.chars().count() < NAME_MIN_LENGTH
        || !trimmed_name.chars().any(char::is_alphabetic)
    {
        errors.push(FieldError {
            field: "customer_name".into(),
            message: "invalid".into(),
        });
    }

    if payload.customer_email.trim().is_empty() {
        errors.push(FieldError {
            field: "customer_email".into(),
            message: "required".into(),
        });
    } else if !is_valid_email(&payload.customer_email) {
        errors.push(FieldError {
            field: "customer_email".into(),
            message: "invalid".into(),
        });
    }

    let trimmed_phone = payload.customer_phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError {
            field: "customer_phone".into(),
            message: "required".into(),
        });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError {
            field: "customer_phone".into(),
            message: "invalid".into(),
        });
    }

    if matches!(payload.delivery_type, DeliveryType::Delivery) {
        let typed_address = payload.delivery_address.as_deref().unwrap_or("").trim();
        match payload.delivery_address_id.as_deref() {
            Some(id) => {
                if !delivery::is_valid_address_id(id) {
                    errors.push(field_error("delivery_address_id", "invalid"));
                }
                if payload.expected_delivery_fee.is_none() {
                    errors.push(field_error("expected_delivery_fee", "required"));
                }
            }
            // A typed address is only accepted while lookup is down; that's
            // checked against PDOK in `resolve_delivery`.
            None if typed_address.is_empty() => errors.push(field_error("delivery_address_id", "required")),
            None if typed_address.chars().count() < ADDRESS_MIN_LENGTH => {
                errors.push(field_error("delivery_address", "too_short"));
            }
            None => {}
        }
        if payload
            .delivery_address_extra
            .as_deref()
            .is_some_and(|extra| extra.trim().chars().count() > ADDRESS_EXTRA_MAX_LENGTH)
        {
            errors.push(field_error("delivery_address_extra", "too_long"));
        }
    }

    if let Some(notes) = &payload.notes
        && notes.chars().count() > NOTES_MAX_LENGTH
    {
        errors.push(FieldError {
            field: "notes".into(),
            message: "too_long".into(),
        });
    }

    if payload.items.is_empty() {
        errors.push(FieldError {
            field: "items".into(),
            message: "empty".into(),
        });
    }

    if let Some(locale) = &payload.locale
        && Locale::parse(locale).is_none()
    {
        errors.push(FieldError {
            field: "locale".into(),
            message: "invalid".into(),
        });
    }
    for (index, item) in payload.items.iter().enumerate() {
        if item.quantity <= 0 {
            errors.push(FieldError {
                field: format!("items[{index}].quantity"),
                message: "must be positive".into(),
            });
        }
        match (item.menu_item_id, item.rental_id) {
            (Some(_), None) => {}
            (None, Some(_)) => {
                if item.price_option_id.is_some() {
                    errors.push(field_error(format!("items[{index}].price_option_id"), "invalid"));
                }
                if item.quantity > RENTAL_MAX_QUANTITY {
                    errors.push(field_error(format!("items[{index}].quantity"), "too_many"));
                }
            }
            _ => errors.push(field_error(format!("items[{index}]"), "invalid")),
        }
    }

    let has_rentals = payload.items.iter().any(|item| item.rental_id.is_some());
    match (has_rentals, payload.rental_date) {
        (true, None) => errors.push(field_error("rental_date", "required")),
        (false, Some(_)) => errors.push(field_error("rental_date", "not_allowed")),
        _ => {}
    }

    errors
}

/// How a delivery order is charged (Phase 37 Decisions 12, 17).
struct DeliveryCharge {
    address: String,
    fee: Decimal,
    distance_km: Option<Decimal>,
    rate_per_km: Option<Decimal>,
    estimated_by: Option<EstimatedBy>,
    pending: bool,
}

async fn resolve_delivery(
    pool: &PgPool,
    delivery: &DeliveryConfig,
    payload: &CreateOrderRequest,
) -> Result<DeliveryCharge, AppError> {
    let unavailable = || AppError::Validation(vec![field_error("delivery_type", "unavailable")]);
    let settings = delivery::load_settings(pool).await?;
    if !settings.delivery_enabled {
        return Err(unavailable());
    }

    let Some(address_id) = payload.delivery_address_id.as_deref() else {
        // A typed address: only while PDOK is really down (Decision 17).
        let typed = payload.delivery_address.as_deref().unwrap_or("").trim();
        return match delivery.geocode(typed).await {
            Ok(_) => Err(AppError::Validation(vec![field_error("delivery_address_id", "required")])),
            Err(delivery::LookupDown) => Ok(DeliveryCharge {
                address: with_extra(typed, payload.delivery_address_extra.as_deref()),
                fee: Decimal::ZERO,
                distance_km: None,
                rate_per_km: None,
                estimated_by: None,
                pending: true,
            }),
        };
    };

    let quote = delivery.quote(pool, address_id, true).await.map_err(|err| match err {
        DeliveryError::OutOfRange => AppError::Validation(vec![field_error("delivery_address_id", "out_of_range")]),
        DeliveryError::NotFound => AppError::Validation(vec![field_error("delivery_address_id", "not_found")]),
        DeliveryError::Unavailable => unavailable(),
        other => other.into(),
    })?;
    if payload.expected_delivery_fee != Some(quote.fee) {
        return Err(AppError::DeliveryFeeChanged { fee: quote.fee, distance_km: quote.distance_km });
    }
    Ok(DeliveryCharge {
        address: with_extra(&quote.address, payload.delivery_address_extra.as_deref()),
        fee: quote.fee,
        distance_km: quote.storable_distance_km(),
        rate_per_km: Some(quote.rate_per_km),
        estimated_by: Some(quote.estimated_by),
        pending: false,
    })
}

fn with_extra(address: &str, extra: Option<&str>) -> String {
    match extra.map(str::trim).filter(|extra| !extra.is_empty()) {
        Some(extra) => format!("{address}, {extra}"),
        None => address.to_string(),
    }
}

struct MenuItemRow {
    name: String,
    price: Option<Decimal>,
    is_available: bool,
    min_quantity: i32,
}

struct PriceOptionRow {
    label: String,
    price: Decimal,
}

struct RentalRow {
    name: String,
    price: Decimal,
    is_available: bool,
}

struct PricedLine {
    kind: &'static str,
    menu_item_id: Option<i64>,
    rental_id: Option<i64>,
    price_option_id: Option<i64>,
    item_name: String,
    option_label: Option<String>,
    unit_price: Decimal,
    quantity: i32,
}

pub async fn create_order(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    State(delivery): State<DeliveryConfig>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    let field_errors = validate(&payload);
    if !field_errors.is_empty() {
        return Err(AppError::Validation(field_errors));
    }

    // Phase 37 Decision 4: from tomorrow to a year ahead, in Dutch time.
    if let Some(rental_date) = payload.rental_date {
        let today = sqlx::query_scalar!(r#"SELECT (now() AT TIME ZONE 'Europe/Amsterdam')::date as "today!: chrono::NaiveDate""#)
            .fetch_one(&pool)
            .await?;
        if rental_date <= today {
            return Err(AppError::Validation(vec![field_error("rental_date", "too_soon")]));
        }
        if rental_date > today + chrono::Days::new(RENTAL_MAX_DAYS_AHEAD as u64) {
            return Err(AppError::Validation(vec![field_error("rental_date", "too_far")]));
        }
    }

    // Before the transaction: this may call PDOK and Google.
    let charge = match payload.delivery_type {
        DeliveryType::Pickup => None,
        DeliveryType::Delivery => Some(resolve_delivery(&pool, &delivery, &payload).await?),
    };

    let mut tx = pool.begin().await?;

    let mut unavailable: Vec<UnavailableItem> = Vec::new();
    let mut priced_lines: Vec<PricedLine> = Vec::new();
    let mut below_minimum: Vec<FieldError> = Vec::new();
    let mut unavailable_rentals: Vec<FieldError> = Vec::new();

    for (index, item) in payload.items.iter().enumerate() {
        let Some(menu_item_id) = item.menu_item_id else {
            // `validate` guarantees a rental id here.
            let rental_id = item.rental_id.unwrap_or_default();
            let rental = sqlx::query_as!(
                RentalRow,
                "SELECT name, price, is_available FROM rentals WHERE id = $1 AND deleted_at IS NULL",
                rental_id
            )
            .fetch_optional(&mut *tx)
            .await?;
            match rental {
                Some(rental) if rental.is_available => priced_lines.push(PricedLine {
                    kind: "rental",
                    menu_item_id: None,
                    rental_id: Some(rental_id),
                    price_option_id: None,
                    item_name: rental.name,
                    option_label: None,
                    unit_price: rental.price,
                    quantity: item.quantity,
                }),
                _ => unavailable_rentals.push(field_error(format!("items[{index}].rental_id"), "unavailable")),
            }
            continue;
        };

        // An unpublished item can't be ordered, and a pending draft never
        // changes the price (Phase 25 Decision 17).
        let menu_item = sqlx::query_as!(
            MenuItemRow,
            "SELECT name, price, is_available, min_quantity FROM menu_items \
             WHERE id = $1 AND deleted_at IS NULL AND published_at IS NOT NULL",
            menu_item_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(menu_item) = menu_item else {
            unavailable.push(UnavailableItem {
                menu_item_id,
                price_option_id: item.price_option_id,
                name: None,
                reason: "not_found".into(),
            });
            continue;
        };

        if !menu_item.is_available {
            unavailable.push(UnavailableItem {
                menu_item_id,
                price_option_id: item.price_option_id,
                name: Some(menu_item.name),
                reason: "unavailable".into(),
            });
            continue;
        }

        let (unit_price, option_label) = match (item.price_option_id, menu_item.price) {
            (Some(option_id), None) => {
                let option = sqlx::query_as!(
                    PriceOptionRow,
                    "SELECT label, price FROM menu_item_price_options \
                     WHERE id = $1 AND menu_item_id = $2 AND deleted_at IS NULL",
                    option_id,
                    menu_item_id
                )
                .fetch_optional(&mut *tx)
                .await?;

                match option {
                    Some(option) => (option.price, Some(option.label)),
                    None => {
                        unavailable.push(UnavailableItem {
                            menu_item_id,
                            price_option_id: item.price_option_id,
                            name: Some(menu_item.name),
                            reason: "unavailable".into(),
                        });
                        continue;
                    }
                }
            }
            (None, Some(price)) => (price, None),
            // A flat-priced item submitted with an option id, or an
            // option-priced item submitted with none — the menu changed
            // shape (Phase 14 doesn't exist yet, but future-proof it).
            _ => {
                unavailable.push(UnavailableItem {
                    menu_item_id,
                    price_option_id: item.price_option_id,
                    name: Some(menu_item.name),
                    reason: "invalid_selection".into(),
                });
                continue;
            }
        };

        // Per line, so each price option has its own minimum (Phase 36
        // Decision 4).
        if item.quantity < menu_item.min_quantity {
            below_minimum.push(FieldError {
                field: format!("items[{index}].quantity"),
                message: "below_minimum".into(),
            });
        }

        priced_lines.push(PricedLine {
            kind: "menu",
            menu_item_id: Some(menu_item_id),
            rental_id: None,
            price_option_id: item.price_option_id,
            item_name: menu_item.name,
            option_label,
            unit_price,
            quantity: item.quantity,
        });
    }

    if !unavailable.is_empty() {
        return Err(AppError::ItemsUnavailable(unavailable));
    }
    below_minimum.extend(unavailable_rentals);
    if !below_minimum.is_empty() {
        return Err(AppError::Validation(below_minimum));
    }

    let subtotal = priced_lines.iter().fold(Decimal::ZERO, |acc, line| {
        acc + line.unit_price * Decimal::from(line.quantity)
    });
    let delivery_fee = charge.as_ref().map_or(Decimal::ZERO, |charge| charge.fee);
    let total = subtotal + delivery_fee;

    let delivery_type_str = match payload.delivery_type {
        DeliveryType::Pickup => "pickup",
        DeliveryType::Delivery => "delivery",
    };
    let locale = payload
        .locale
        .as_deref()
        .and_then(Locale::parse)
        .unwrap_or(Locale::En);

    let order = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total, locale,
             delivery_distance_km, delivery_rate_per_km, delivery_estimated_by,
             delivery_fee_pending, rental_date)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        RETURNING id, status, public_token,
                  created_at as "created_at: Timestamp",
                  to_char(created_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "placed_at!"
        "#,
        payload.customer_name,
        payload.customer_email,
        payload.customer_phone,
        delivery_type_str,
        charge.as_ref().map(|charge| charge.address.clone()),
        payload.notes,
        subtotal,
        delivery_fee,
        total,
        locale.as_str(),
        charge.as_ref().and_then(|charge| charge.distance_km),
        charge.as_ref().and_then(|charge| charge.rate_per_km),
        charge.as_ref().and_then(|charge| charge.estimated_by).map(EstimatedBy::as_str),
        charge.as_ref().is_some_and(|charge| charge.pending),
        payload.rental_date.map(crate::chrono_date_to_offset),
    )
    .fetch_one(&mut *tx)
    .await?;

    let mut item_responses = Vec::with_capacity(priced_lines.len());
    for line in &priced_lines {
        let line_subtotal = line.unit_price * Decimal::from(line.quantity);
        sqlx::query!(
            r#"
            INSERT INTO order_items
                (order_id, menu_item_id, price_option_id, item_name, option_label,
                 unit_price, quantity, subtotal, rental_id, kind)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            order.id,
            line.menu_item_id,
            line.price_option_id,
            line.item_name,
            line.option_label,
            line.unit_price,
            line.quantity,
            line_subtotal,
            line.rental_id,
            line.kind,
        )
        .execute(&mut *tx)
        .await?;

        item_responses.push(OrderItemResponse {
            kind: line.kind.to_string(),
            item_name: line.item_name.clone(),
            option_label: line.option_label.clone(),
            unit_price: line.unit_price,
            quantity: line.quantity,
            subtotal: line_subtotal,
        });
    }

    tx.commit().await?;

    let response = OrderResponse {
        id: order.id,
        customer_name: payload.customer_name,
        customer_email: payload.customer_email,
        customer_phone: payload.customer_phone,
        delivery_type: delivery_type_str.to_string(),
        delivery_address: charge.as_ref().map(|charge| charge.address.clone()),
        notes: payload.notes,
        subtotal,
        delivery_fee,
        delivery_distance_km: charge.as_ref().and_then(|charge| charge.distance_km),
        delivery_estimated_by: charge
            .as_ref()
            .and_then(|charge| charge.estimated_by)
            .map(|by| by.as_str().to_string()),
        delivery_fee_pending: charge.as_ref().is_some_and(|charge| charge.pending),
        rental_date: payload.rental_date,
        total,
        status: order.status,
        created_at: order.created_at,
        items: item_responses,
        public_token: order.public_token,
        payment_reference: payment_reference(order.id),
        payment: payment.instructions(order.id),
        payment_claimed_at: None,
        payment_confirmed_at: None,
        ready_at: None,
        completed_at: None,
        cancelled_at: None,
        completed_by: None,
        event_booking_token: None,
        tikkie: None,
    };

    send_invoice(&email, &pool, &response, &order.placed_at);
    // Independent of email (Phase 27 Decision 7).
    push.spawn_notify_owner(&pool, push::new_order(&response));

    if let Some(business_address) = email.business_notify_address() {
        email.spawn_send(OutgoingEmail {
            to: business_address,
            subject: format!("New order {} — €{}", response.payment_reference, response.total),
            body: new_order_owner_email_body(&response, email.admin_url()),
            reply_to: Some(response.customer_email.clone()),
            track: None,
        });
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// The customer's invoice email, for a cart order or an accepted catering
/// quote (Phase 23 Decision 4).
pub(crate) fn send_invoice(email: &EmailConfig, pool: &PgPool, order: &OrderResponse, placed_at: &str) {
    let (subject, body) = invoice_email(order, placed_at, email.site_url(), order.public_token);
    email.spawn_send(OutgoingEmail {
        to: order.customer_email.clone(),
        subject,
        body,
        reply_to: None,
        track: Some(DeliveryRef { pool: pool.clone(), order_id: Some(order.id), quote_id: None, kind: "invoice" }),
    });
}

fn delivery_line(order: &OrderResponse) -> String {
    let mut line = match order.delivery_address.as_deref() {
        Some(address) if order.delivery_type == "delivery" => format!("Delivery to: {address}"),
        _ => "Pickup".to_string(),
    };
    // Phase 37 Decisions 4, 13, 17.
    if order.delivery_fee_pending {
        line.push_str("\nDelivery fee needed: the address couldn't be looked up. Set the fee in admin before the customer can pay.");
    } else if order.delivery_type == "delivery" && order.delivery_fee > Decimal::ZERO {
        match order.delivery_distance_km {
            Some(km) => line.push_str(&format!("\nDelivery ({km} km): €{}", order.delivery_fee)),
            None => line.push_str(&format!("\nDelivery: €{}", order.delivery_fee)),
        }
        if order.delivery_estimated_by.as_deref() == Some("straight_line") {
            line.push_str(" (straight-line estimate)");
        }
    }
    if let Some(date) = order.rental_date {
        line.push_str(&format!("\nRental date: {}", date.format("%d-%m-%Y")));
    }
    line
}

fn new_order_owner_email_body(order: &OrderResponse, admin_url: &str) -> String {
    format!(
        "New order {} from {} ({}).\n{}\nTotal: €{}\n\nIt is awaiting payment. Check the bank for a transfer with reference {}.\n\n{admin_url}/orders/{}",
        order.payment_reference,
        order.customer_name,
        order.customer_phone,
        delivery_line(order),
        order.total,
        order.payment_reference,
        order.id,
    )
}

/// Malformed and unknown tokens both come back as 404, so the two can't be
/// told apart (Phase 20 Decision 7).
pub(crate) fn parse_token(token: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(token).map_err(|_| AppError::NotFound)
}

pub(crate) async fn load_order_by_token(
    pool: &PgPool,
    payment: &PaymentConfig,
    token: Uuid,
) -> Result<OrderResponse, AppError> {
    let order = sqlx::query!(
        r#"
        SELECT id, customer_name, customer_email, customer_phone, delivery_type,
               delivery_address, notes, subtotal, delivery_fee, total, status, public_token,
               completed_by, delivery_distance_km, delivery_estimated_by, delivery_fee_pending,
               rental_date as "rental_date: chrono::NaiveDate",
               created_at as "created_at: Timestamp",
               payment_claimed_at as "payment_claimed_at: Timestamp",
               payment_confirmed_at as "payment_confirmed_at: Timestamp",
               ready_at as "ready_at: Timestamp",
               completed_at as "completed_at: Timestamp",
               cancelled_at as "cancelled_at: Timestamp",
               (SELECT b.public_token FROM event_bookings b WHERE b.order_id = orders.id) as "event_booking_token?"
        FROM orders
        WHERE public_token = $1
        "#,
        token
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let items = sqlx::query_as!(
        OrderItemResponse,
        r#"
        SELECT kind, item_name, option_label, unit_price, quantity, subtotal
        FROM order_items
        WHERE order_id = $1
        ORDER BY id
        "#,
        order.id
    )
    .fetch_all(pool)
    .await?;

    Ok(OrderResponse {
        id: order.id,
        customer_name: order.customer_name,
        customer_email: order.customer_email,
        customer_phone: order.customer_phone,
        delivery_type: order.delivery_type,
        delivery_address: order.delivery_address,
        notes: order.notes,
        subtotal: order.subtotal,
        delivery_fee: order.delivery_fee,
        delivery_distance_km: order.delivery_distance_km,
        delivery_estimated_by: order.delivery_estimated_by,
        delivery_fee_pending: order.delivery_fee_pending,
        rental_date: order.rental_date,
        total: order.total,
        status: order.status,
        created_at: order.created_at,
        items,
        public_token: order.public_token,
        payment_reference: payment_reference(order.id),
        payment: payment.instructions(order.id),
        payment_claimed_at: order.payment_claimed_at,
        payment_confirmed_at: order.payment_confirmed_at,
        ready_at: order.ready_at,
        completed_at: order.completed_at,
        cancelled_at: order.cancelled_at,
        completed_by: order.completed_by,
        event_booking_token: order.event_booking_token,
        tikkie: match order.event_booking_token {
            Some(_) => crate::events::tikkie_for_order(pool, order.id).await?,
            None => None,
        },
    })
}

pub async fn get_order_by_token(
    State(pool): State<PgPool>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let token = parse_token(&token)?;
    Ok(Json(load_order_by_token(&pool, &payment, token).await?))
}

/// The customer's "I've paid" (Phase 20 Decision 7). Idempotent: only the
/// request that actually moves the order out of `awaiting_payment` emails
/// the owner, so a double click or two racing requests send one email.
pub async fn claim_payment(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(push): State<PushConfig>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let token = parse_token(&token)?;

    // Phase 37 Decision 17: nothing to pay until the owner sets the fee.
    let claimed = sqlx::query_scalar!(
        r#"
        UPDATE orders
        SET status = 'payment_claimed', payment_claimed_at = now(), updated_at = now()
        WHERE public_token = $1 AND status = 'awaiting_payment' AND NOT delivery_fee_pending
        RETURNING id
        "#,
        token
    )
    .fetch_optional(&pool)
    .await?;

    let order = load_order_by_token(&pool, &payment, token).await?;

    let Some(order_id) = claimed else {
        if order.status == "cancelled" {
            return Err(AppError::InvalidStatusTransition);
        }
        if order.delivery_fee_pending {
            return Err(AppError::DeliveryFeePending);
        }
        return Ok(Json(order));
    };

    tracing::info!(order_id, "customer claimed payment");
    push.spawn_notify_owner(&pool, push::payment_claimed(&order));

    if let Some(business_address) = email.business_notify_address() {
        email.spawn_send(OutgoingEmail {
            to: business_address,
            subject: format!("Payment claimed for order {} — €{}", order.payment_reference, order.total),
            body: format!(
                "{} says they've paid order {} (€{}).\n\nCheck the bank for a transfer with reference {}, then confirm the payment in admin:\n{}/orders/{}",
                order.customer_name,
                order.payment_reference,
                order.total,
                order.payment_reference,
                email.admin_url(),
                order_id,
            ),
            reply_to: Some(order.customer_email.clone()),
            track: None,
        });
    }

    Ok(Json(order))
}

/// The customer's "I've received / collected it" (Phase 22 Decision 8).
/// Idempotent like `claim_payment`: the guarded UPDATE decides, so a double
/// click completes the order once. No email either way.
pub async fn confirm_received(
    State(pool): State<PgPool>,
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let token = parse_token(&token)?;

    let completed = sqlx::query_scalar!(
        r#"
        UPDATE orders
        SET status = 'completed', completed_at = now(), completed_by = 'customer', updated_at = now()
        WHERE public_token = $1 AND status = ANY($2)
        RETURNING id
        "#,
        token,
        &CUSTOMER_COMPLETABLE.map(String::from) as &[String],
    )
    .fetch_optional(&pool)
    .await?;

    let order = load_order_by_token(&pool, &payment, token).await?;

    match completed {
        Some(order_id) => tracing::info!(order_id, "customer confirmed receipt"),
        None if order.status != "completed" => return Err(AppError::InvalidStatusTransition),
        None => {}
    }

    Ok(Json(order))
}
