use crate::email::EmailConfig;
use crate::error::{AppError, FieldError};
use crate::order_emails::{status_email, Locale};
use crate::order_status::{admin_next, is_reopen, is_valid_status, ORDER_STATUSES};
use crate::payment::payment_reference;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;
const RECENT_ORDER_COUNT: i64 = 5;

struct OrderSummaryRow {
    id: i64,
    customer_name: String,
    delivery_type: String,
    total: Decimal,
    status: String,
    created_at: DateTime<Utc>,
    catering_enquiry_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct OrderSummaryResponse {
    pub id: i64,
    pub payment_reference: String,
    pub customer_name: String,
    pub delivery_type: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
    /// Set when the order came from an accepted catering quote (Phase 23
    /// Decision 7).
    pub catering_enquiry_id: Option<i64>,
}

impl From<OrderSummaryRow> for OrderSummaryResponse {
    fn from(row: OrderSummaryRow) -> Self {
        Self {
            id: row.id,
            payment_reference: payment_reference(row.id),
            customer_name: row.customer_name,
            delivery_type: row.delivery_type,
            total: row.total,
            status: row.status,
            created_at: row.created_at,
            catering_enquiry_id: row.catering_enquiry_id,
        }
    }
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
        OrderSummaryRow,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>",
               (SELECT q.enquiry_id FROM catering_quotes q WHERE q.order_id = orders.id) as "catering_enquiry_id?"
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
    .await?
    .into_iter()
    .map(OrderSummaryResponse::from)
    .collect();

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
    pub payment_reference: String,
    pub payment_claimed_at: Option<DateTime<Utc>>,
    pub payment_confirmed_at: Option<DateTime<Utc>>,
    pub ready_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    /// `customer` or `admin`, once completed (Phase 22 Decision 8).
    pub completed_by: Option<String>,
    pub catering_enquiry_id: Option<i64>,
    /// The admin UI renders these as buttons, so `admin_next` has no
    /// TypeScript mirror to drift from (Phase 20 open risk 4).
    pub next_statuses: &'static [&'static str],
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
               payment_claimed_at as "payment_claimed_at: DateTime<Utc>",
               payment_confirmed_at as "payment_confirmed_at: DateTime<Utc>",
               ready_at as "ready_at: DateTime<Utc>",
               completed_at as "completed_at: DateTime<Utc>",
               cancelled_at as "cancelled_at: DateTime<Utc>",
               completed_by,
               created_at as "created_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>",
               (SELECT q.enquiry_id FROM catering_quotes q WHERE q.order_id = orders.id) as "catering_enquiry_id?"
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

    let next_statuses = admin_next(&order.status, &order.delivery_type, order.completed_by.as_deref());

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
        payment_reference: payment_reference(order.id),
        payment_claimed_at: order.payment_claimed_at,
        payment_confirmed_at: order.payment_confirmed_at,
        ready_at: order.ready_at,
        completed_at: order.completed_at,
        cancelled_at: order.cancelled_at,
        completed_by: order.completed_by,
        catering_enquiry_id: order.catering_enquiry_id,
        next_statuses,
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
    State(email): State<EmailConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateOrderStatusRequest>,
) -> Result<Json<OrderSummaryResponse>, AppError> {
    if !is_valid_status(&payload.status) {
        return Err(AppError::Validation(vec![FieldError {
            field: "status".into(),
            message: "invalid".into(),
        }]));
    }

    let mut tx = pool.begin().await?;

    // Locked so a concurrent customer claim or second admin tab can't slip
    // a status change in between the check and the update.
    let current = sqlx::query!(
        "SELECT status, delivery_type, completed_by FROM orders WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    // Phase 20 Decision 3: only the next step(s) for this order's
    // pickup/delivery branch are accepted.
    // Phase 22 Decision 8: a customer-completed order may also go back to
    // its ready status.
    if !admin_next(&current.status, &current.delivery_type, current.completed_by.as_deref())
        .contains(&payload.status.as_str())
    {
        return Err(AppError::InvalidStatusTransition);
    }
    let reopened = is_reopen(&current.status, &payload.status);

    // `updated_at = now()` is explicit on purpose: the column's DEFAULT only
    // fires on INSERT and there is no BEFORE UPDATE trigger anywhere in
    // migrations/ (Phase 13 requirement.md Decision 9). Each step stamps
    // its time for the tracking page (Phase 22 Decision 4); moving to
    // `processing` is "Confirm payment". A reopen ($3) clears the
    // completion and keeps `ready_at`.
    let order = sqlx::query!(
        r#"
        UPDATE orders
        SET status = $2,
            updated_at = now(),
            payment_confirmed_at = CASE WHEN $2 = 'processing' THEN now() ELSE payment_confirmed_at END,
            ready_at = CASE
                WHEN $2 IN ('ready_for_collection', 'out_for_delivery', 'serving_on_site') AND NOT $3 THEN now()
                ELSE ready_at END,
            completed_at = CASE WHEN $2 = 'completed' THEN now() WHEN $3 THEN NULL ELSE completed_at END,
            completed_by = CASE WHEN $2 = 'completed' THEN 'admin' WHEN $3 THEN NULL ELSE completed_by END,
            cancelled_at = CASE WHEN $2 = 'cancelled' THEN now() ELSE cancelled_at END
        WHERE id = $1
        RETURNING id, customer_name, customer_email, delivery_type, total, status,
                  public_token, locale,
                  created_at as "created_at: DateTime<Utc>",
                  (SELECT q.enquiry_id FROM catering_quotes q WHERE q.order_id = orders.id) as "catering_enquiry_id?"
        "#,
        id,
        payload.status,
        reopened,
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(order_id = id, status = %order.status, reopened, "admin updated order status");

    // After the commit, like `create_order`: a failed send never undoes the
    // status change. A reopen is silent (Phase 22 Decisions 6, 8).
    if !reopened {
        let locale = Locale::parse(&order.locale).unwrap_or(Locale::En);
        let reference = payment_reference(order.id);
        if let Some((subject, body)) = status_email(
            locale,
            &order.customer_name,
            &reference,
            &order.status,
            email.site_url(),
            order.public_token,
        ) {
            email.spawn_send(order.customer_email, subject, body);
        }
    }

    Ok(Json(OrderSummaryResponse {
        id: order.id,
        payment_reference: payment_reference(order.id),
        customer_name: order.customer_name,
        delivery_type: order.delivery_type,
        total: order.total,
        status: order.status,
        created_at: order.created_at,
        catering_enquiry_id: order.catering_enquiry_id,
    }))
}

#[derive(Debug, Serialize)]
pub struct OrdersSummaryResponse {
    pub counts: HashMap<String, i64>,
    pub today: i64,
    pub recent: Vec<OrderSummaryResponse>,
    /// Orders waiting on the owner to confirm payment, for the admin's
    /// "new since you last looked" badge (Phase 22 Decision 9).
    pub claimed_ids: Vec<i64>,
}

pub async fn orders_summary(
    State(pool): State<PgPool>,
) -> Result<Json<OrdersSummaryResponse>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT status, COUNT(*) as "count!" FROM orders GROUP BY status"#
    )
    .fetch_all(&pool)
    .await?;

    // Seed every status at zero so the dashboard renders every tile even before a
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
        OrderSummaryRow,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>",
               (SELECT q.enquiry_id FROM catering_quotes q WHERE q.order_id = orders.id) as "catering_enquiry_id?"
        FROM orders
        ORDER BY created_at DESC, id DESC
        LIMIT $1
        "#,
        RECENT_ORDER_COUNT
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(OrderSummaryResponse::from)
    .collect();

    let claimed_ids = sqlx::query_scalar!("SELECT id FROM orders WHERE status = 'payment_claimed' ORDER BY id")
        .fetch_all(&pool)
        .await?;

    Ok(Json(OrdersSummaryResponse { counts, today, recent, claimed_ids }))
}
