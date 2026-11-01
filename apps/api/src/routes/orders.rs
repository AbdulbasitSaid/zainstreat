use crate::email::{DeliveryRef, EmailConfig, OutgoingEmail};
use crate::error::{AppError, FieldError, UnavailableItem};
use crate::order_emails::{invoice_email, Locale};
use crate::order_status::CUSTOMER_COMPLETABLE;
use crate::payment::{payment_reference, PaymentConfig, PaymentInstructions};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
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

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub menu_item_id: i64,
    pub price_option_id: Option<i64>,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: DeliveryType,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateOrderItemRequest>,
    /// The site language the customer ordered in, for their emails. Missing
    /// means `en` (Phase 22 Decision 5).
    pub locale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrderItemResponse {
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
}

const NAME_MIN_LENGTH: usize = 2;
const ADDRESS_MIN_LENGTH: usize = 5;
const NOTES_MAX_LENGTH: usize = 500;

fn validate(payload: &CreateOrderRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.customer_name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError { field: "customer_name".into(), message: "required".into() });
    } else if trimmed_name.chars().count() < NAME_MIN_LENGTH || !trimmed_name.chars().any(char::is_alphabetic) {
        errors.push(FieldError { field: "customer_name".into(), message: "invalid".into() });
    }

    if payload.customer_email.trim().is_empty() {
        errors.push(FieldError { field: "customer_email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.customer_email) {
        errors.push(FieldError { field: "customer_email".into(), message: "invalid".into() });
    }

    let trimmed_phone = payload.customer_phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError { field: "customer_phone".into(), message: "required".into() });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError { field: "customer_phone".into(), message: "invalid".into() });
    }

    if matches!(payload.delivery_type, DeliveryType::Delivery) {
        let trimmed_address = payload.delivery_address.as_deref().unwrap_or("").trim();
        if trimmed_address.is_empty() {
            errors.push(FieldError { field: "delivery_address".into(), message: "required".into() });
        } else if trimmed_address.chars().count() < ADDRESS_MIN_LENGTH {
            errors.push(FieldError { field: "delivery_address".into(), message: "too_short".into() });
        }
    }

    if let Some(notes) = &payload.notes
        && notes.chars().count() > NOTES_MAX_LENGTH
    {
        errors.push(FieldError { field: "notes".into(), message: "too_long".into() });
    }

    if payload.items.is_empty() {
        errors.push(FieldError { field: "items".into(), message: "empty".into() });
    }

    if let Some(locale) = &payload.locale
        && Locale::parse(locale).is_none()
    {
        errors.push(FieldError { field: "locale".into(), message: "invalid".into() });
    }
    for (index, item) in payload.items.iter().enumerate() {
        if item.quantity <= 0 {
            errors.push(FieldError {
                field: format!("items[{index}].quantity"),
                message: "must be positive".into(),
            });
        }
    }

    errors
}

struct MenuItemRow {
    name: String,
    price: Option<Decimal>,
    is_available: bool,
}

struct PriceOptionRow {
    label: String,
    price: Decimal,
}

struct PricedLine {
    menu_item_id: i64,
    price_option_id: Option<i64>,
    item_name: String,
    option_label: Option<String>,
    unit_price: Decimal,
    quantity: i32,
}

pub async fn create_order(
    State(pool): State<PgPool>,
    State(email): State<EmailConfig>,
    State(payment): State<PaymentConfig>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    let field_errors = validate(&payload);
    if !field_errors.is_empty() {
        return Err(AppError::Validation(field_errors));
    }

    let mut tx = pool.begin().await?;

    let mut unavailable: Vec<UnavailableItem> = Vec::new();
    let mut priced_lines: Vec<PricedLine> = Vec::new();

    for item in &payload.items {
        // An unpublished item can't be ordered, and a pending draft never
        // changes the price (Phase 25 Decision 17).
        let menu_item = sqlx::query_as!(
            MenuItemRow,
            "SELECT name, price, is_available FROM menu_items \
             WHERE id = $1 AND deleted_at IS NULL AND published_at IS NOT NULL",
            item.menu_item_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(menu_item) = menu_item else {
            unavailable.push(UnavailableItem {
                menu_item_id: item.menu_item_id,
                price_option_id: item.price_option_id,
                name: None,
                reason: "not_found".into(),
            });
            continue;
        };

        if !menu_item.is_available {
            unavailable.push(UnavailableItem {
                menu_item_id: item.menu_item_id,
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
                    item.menu_item_id
                )
                .fetch_optional(&mut *tx)
                .await?;

                match option {
                    Some(option) => (option.price, Some(option.label)),
                    None => {
                        unavailable.push(UnavailableItem {
                            menu_item_id: item.menu_item_id,
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
                    menu_item_id: item.menu_item_id,
                    price_option_id: item.price_option_id,
                    name: Some(menu_item.name),
                    reason: "invalid_selection".into(),
                });
                continue;
            }
        };

        priced_lines.push(PricedLine {
            menu_item_id: item.menu_item_id,
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

    let subtotal = priced_lines
        .iter()
        .fold(Decimal::ZERO, |acc, line| acc + line.unit_price * Decimal::from(line.quantity));
    let delivery_fee = Decimal::ZERO; // Decision 9 — Beyond MVP.
    let total = subtotal + delivery_fee;

    let delivery_type_str = match payload.delivery_type {
        DeliveryType::Pickup => "pickup",
        DeliveryType::Delivery => "delivery",
    };
    let locale = payload.locale.as_deref().and_then(Locale::parse).unwrap_or(Locale::En);

    let order = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total, locale)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, status, public_token,
                  created_at as "created_at: Timestamp",
                  to_char(created_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "placed_at!"
        "#,
        payload.customer_name,
        payload.customer_email,
        payload.customer_phone,
        delivery_type_str,
        payload.delivery_address,
        payload.notes,
        subtotal,
        delivery_fee,
        total,
        locale.as_str(),
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
                 unit_price, quantity, subtotal)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            order.id,
            line.menu_item_id,
            line.price_option_id,
            line.item_name,
            line.option_label,
            line.unit_price,
            line.quantity,
            line_subtotal,
        )
        .execute(&mut *tx)
        .await?;

        item_responses.push(OrderItemResponse {
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
        delivery_address: payload.delivery_address,
        notes: payload.notes,
        subtotal,
        delivery_fee,
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
    };

    send_invoice(&email, &pool, locale, &response, &order.placed_at);

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
pub(crate) fn send_invoice(
    email: &EmailConfig,
    pool: &PgPool,
    locale: Locale,
    order: &OrderResponse,
    placed_at: &str,
) {
    let (subject, body) = invoice_email(locale, order, placed_at, email.site_url(), order.public_token);
    email.spawn_send(OutgoingEmail {
        to: order.customer_email.clone(),
        subject,
        body,
        reply_to: None,
        track: Some(DeliveryRef { pool: pool.clone(), order_id: Some(order.id), quote_id: None, kind: "invoice" }),
    });
}

fn delivery_line(order: &OrderResponse) -> String {
    match order.delivery_address.as_deref() {
        Some(address) if order.delivery_type == "delivery" => format!("Delivery to: {address}"),
        _ => "Pickup".to_string(),
    }
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
               completed_by,
               created_at as "created_at: Timestamp",
               payment_claimed_at as "payment_claimed_at: Timestamp",
               payment_confirmed_at as "payment_confirmed_at: Timestamp",
               ready_at as "ready_at: Timestamp",
               completed_at as "completed_at: Timestamp",
               cancelled_at as "cancelled_at: Timestamp"
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
        SELECT item_name, option_label, unit_price, quantity, subtotal
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
    State(payment): State<PaymentConfig>,
    Path(token): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let token = parse_token(&token)?;

    let claimed = sqlx::query_scalar!(
        r#"
        UPDATE orders
        SET status = 'payment_claimed', payment_claimed_at = now(), updated_at = now()
        WHERE public_token = $1 AND status = 'awaiting_payment'
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
        return Ok(Json(order));
    };

    tracing::info!(order_id, "customer claimed payment");

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
