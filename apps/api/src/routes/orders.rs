use crate::error::{AppError, FieldError, UnavailableItem};
use axum::{extract::State, http::StatusCode, Json};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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
}

const NAME_MIN_LENGTH: usize = 2;
const ADDRESS_MIN_LENGTH: usize = 5;
const NOTES_MAX_LENGTH: usize = 500;

fn is_valid_email(value: &str) -> bool {
    // Deliberately loose — just "looks like an email". Decision 10 scopes
    // real validation hardening to roadmap.md Phase 17.
    match value.split_once('@') {
        Some((local, domain)) => !local.is_empty() && domain.contains('.'),
        None => false,
    }
}

const ASCENDING_DIGITS: &str = "01234567890123456789";
const DESCENDING_DIGITS: &str = "09876543210987654321";

fn is_valid_phone(value: &str) -> bool {
    // Must look like a real phone number: an optional leading "+" with a
    // country code, or a leading trunk "0", then only digits/spaces/
    // hyphens/parentheses — "+" may only appear as the very first character.
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return false };
    let rest = chars.as_str();

    let starts_ok = match first {
        '+' => rest.chars().next().is_some_and(|c| c.is_ascii_digit() && c != '0'),
        '0' => true,
        _ => false,
    };
    if !starts_ok {
        return false;
    }
    if !rest.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '(' | ')' | ' ')) {
        return false;
    }

    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 7 || digits.len() > 15 {
        return false;
    }

    // Reject obviously fake numbers: all the same digit, or a straight
    // ascending/descending run (e.g. "1234567890", "0123456789").
    if digits.bytes().all(|b| b == digits.as_bytes()[0]) {
        return false;
    }
    if ASCENDING_DIGITS.contains(&digits) || DESCENDING_DIGITS.contains(&digits) {
        return false;
    }

    true
}

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
        RETURNING id, status, created_at as "created_at: chrono::DateTime<chrono::Utc>"
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

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse {
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
        }),
    ))
}
