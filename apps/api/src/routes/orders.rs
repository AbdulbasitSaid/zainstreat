use crate::email::EmailConfig;
use crate::error::{AppError, FieldError, UnavailableItem};
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
        let menu_item = sqlx::query_as!(
            MenuItemRow,
            "SELECT name, price, is_available FROM menu_items WHERE id = $1 AND deleted_at IS NULL",
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

    let order = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, status, public_token,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>",
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
    };

    email.spawn_send(
        response.customer_email.clone(),
        format!("Your invoice for order {} — Zain's Treat n More", response.payment_reference),
        invoice_email_body(&response, &order.placed_at, email.site_url()),
    );

    if let Some(business_address) = email.business_notify_address() {
        email.spawn_send(
            business_address,
            format!("New order {} — €{}", response.payment_reference, response.total),
            new_order_owner_email_body(&response, email.site_url()),
        );
    }

    Ok((StatusCode::CREATED, Json(response)))
}

fn delivery_line(order: &OrderResponse) -> String {
    match order.delivery_address.as_deref() {
        Some(address) if order.delivery_type == "delivery" => format!("Delivery to: {address}"),
        _ => "Pickup".to_string(),
    }
}

/// The customer's invoice email (Phase 20 Decision 8). Plain text and
/// English only, like every other email (Phase 17); no VAT line (KOR).
fn invoice_email_body(order: &OrderResponse, placed_at: &str, site_url: &str) -> String {
    let mut body = format!(
        "Hi {},\n\nThanks for your order! Here is your invoice.\n\nOrder: {}\nDate: {}\n{}\n\n",
        order.customer_name,
        order.payment_reference,
        placed_at,
        delivery_line(order),
    );

    for item in &order.items {
        let name = match &item.option_label {
            Some(label) => format!("{} ({label})", item.item_name),
            None => item.item_name.clone(),
        };
        body.push_str(&format!("{} x {name} — €{}\n", item.quantity, item.subtotal));
    }
    body.push_str(&format!("\nTotal: €{}\n\n", order.total));

    match &order.payment {
        Some(payment) => {
            body.push_str(&format!(
                "Please pay by bank transfer:\nAccount name: {}\nIBAN: {}\n",
                payment.account_name, payment.iban,
            ));
            if let Some(bic) = &payment.bic {
                body.push_str(&format!("BIC: {bic}\n"));
            }
            body.push_str(&format!(
                "Reference: {}\n\nPlease use this exact reference so we can match your payment.\n\n",
                payment.reference,
            ));
        }
        None => body.push_str(
            "Please contact us on WhatsApp for our bank details.\n\n",
        ),
    }

    body.push_str(&format!(
        "We only start preparing your order once your payment has been verified.\n\n\
         After paying, let us know on your order page:\n{site_url}/order/{}\n\n\
         — Zain's Treat n More",
        order.public_token,
    ));
    body
}

fn new_order_owner_email_body(order: &OrderResponse, site_url: &str) -> String {
    format!(
        "New order {} from {} ({}).\n{}\nTotal: €{}\n\nIt is awaiting payment. Check the bank for a transfer with reference {}.\n\n{site_url}/admin/orders/{}",
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
fn parse_token(token: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(token).map_err(|_| AppError::NotFound)
}

async fn load_order_by_token(
    pool: &PgPool,
    payment: &PaymentConfig,
    token: Uuid,
) -> Result<OrderResponse, AppError> {
    let order = sqlx::query!(
        r#"
        SELECT id, customer_name, customer_email, customer_phone, delivery_type,
               delivery_address, notes, subtotal, delivery_fee, total, status, public_token,
               created_at as "created_at: chrono::DateTime<chrono::Utc>"
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
        email.spawn_send(
            business_address,
            format!("Payment claimed for order {} — €{}", order.payment_reference, order.total),
            format!(
                "{} says they've paid order {} (€{}).\n\nCheck the bank for a transfer with reference {}, then confirm the payment in admin:\n{}/admin/orders/{}",
                order.customer_name,
                order.payment_reference,
                order.total,
                order.payment_reference,
                email.site_url(),
                order_id,
            ),
        );
    }

    Ok(Json(order))
}
