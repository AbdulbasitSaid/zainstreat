use crate::error::{AppError, FieldError};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

/// Mirrors the CHECK constraint on `orders.status` in
/// `migrations/20261002162443_initial_schema.up.sql`. Adding a status means
/// editing both — see requirement.md open risk 7.
pub const ORDER_STATUSES: [&str; 6] = [
    "new",
    "confirmed",
    "preparing",
    "ready",
    "completed",
    "cancelled",
];

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;
const RECENT_ORDER_COUNT: i64 = 5;

fn is_valid_status(value: &str) -> bool {
    ORDER_STATUSES.contains(&value)
}

#[derive(Debug, Serialize)]
pub struct OrderSummaryResponse {
    pub id: i64,
    pub customer_name: String,
    pub delivery_type: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderSummaryResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_orders(
    State(pool): State<PgPool>,
    Query(params): Query<ListOrdersQuery>,
) -> Result<Json<OrderListResponse>, AppError> {
    let mut errors = Vec::new();

    if let Some(status) = &params.status
        && !is_valid_status(status)
    {
        errors.push(FieldError { field: "status".into(), message: "invalid".into() });
    }

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        errors.push(FieldError { field: "limit".into(), message: "out_of_range".into() });
    }

    let offset = params.offset.unwrap_or(0);
    if offset < 0 {
        errors.push(FieldError { field: "offset".into(), message: "out_of_range".into() });
    }

    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM orders WHERE ($1::TEXT IS NULL OR status = $1)"#,
        params.status.as_deref()
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let orders = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>"
        FROM orders
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY created_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
        params.status.as_deref(),
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(OrderListResponse { orders, total, limit, offset }))
}

#[derive(Debug, Serialize)]
pub struct AdminOrderItemResponse {
    pub id: i64,
    pub item_name: String,
    pub option_label: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    pub quantity: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AdminOrderResponse {
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<AdminOrderItemResponse>,
}

pub async fn get_order(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminOrderResponse>, AppError> {
    // `fetch_one` yields sqlx::Error::RowNotFound for an unknown id, which
    // error.rs already maps to 404 {"error":"not_found"} — no new variant.
    let order = sqlx::query!(
        r#"
        SELECT id, customer_name, customer_email, customer_phone, delivery_type,
               delivery_address, notes, subtotal, delivery_fee, total, status,
               created_at as "created_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>"
        FROM orders
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&pool)
    .await?;

    let items = sqlx::query_as!(
        AdminOrderItemResponse,
        r#"
        SELECT id, item_name, option_label, unit_price, quantity, subtotal
        FROM order_items
        WHERE order_id = $1
        ORDER BY id
        "#,
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(AdminOrderResponse {
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
        updated_at: order.updated_at,
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub status: String,
}

pub async fn update_order_status(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateOrderStatusRequest>,
) -> Result<Json<OrderSummaryResponse>, AppError> {
    if !is_valid_status(&payload.status) {
        return Err(AppError::Validation(vec![FieldError {
            field: "status".into(),
            message: "invalid".into(),
        }]));
    }

    // `updated_at = now()` is explicit on purpose: the column's DEFAULT only
    // fires on INSERT and there is no BEFORE UPDATE trigger anywhere in
    // migrations/ (requirement.md Decision 9).
    let order = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        UPDATE orders
        SET status = $2, updated_at = now()
        WHERE id = $1
        RETURNING id, customer_name, delivery_type, total, status,
                  created_at as "created_at: DateTime<Utc>"
        "#,
        id,
        payload.status
    )
    .fetch_one(&pool)
    .await?;

    tracing::info!(order_id = id, status = %order.status, "admin updated order status");

    Ok(Json(order))
}

#[derive(Debug, Serialize)]
pub struct OrdersSummaryResponse {
    pub counts: HashMap<String, i64>,
    pub today: i64,
    pub recent: Vec<OrderSummaryResponse>,
}

pub async fn orders_summary(
    State(pool): State<PgPool>,
) -> Result<Json<OrdersSummaryResponse>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT status, COUNT(*) as "count!" FROM orders GROUP BY status"#
    )
    .fetch_all(&pool)
    .await?;

    // Seed all six at zero so the dashboard renders every tile even before a
    // single order of that status exists.
    let mut counts: HashMap<String, i64> =
        ORDER_STATUSES.iter().map(|s| ((*s).to_string(), 0)).collect();
    for row in rows {
        counts.insert(row.status, row.count);
    }

    // "Today" means today in Europe/Amsterdam, not UTC — the business's wall
    // clock, matching what the frontend renders (requirement.md Decision 12).
    let today = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM orders
        WHERE created_at >= date_trunc('day', now() AT TIME ZONE 'Europe/Amsterdam')
                            AT TIME ZONE 'Europe/Amsterdam'
        "#
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let recent = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>"
        FROM orders
        ORDER BY created_at DESC, id DESC
        LIMIT $1
        "#,
        RECENT_ORDER_COUNT
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(OrdersSummaryResponse { counts, today, recent }))
}
