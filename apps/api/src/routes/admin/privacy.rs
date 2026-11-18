//! Phase 33 Decision 16: the admin Privacy tool. Finds everything held for
//! one email address, exports it (GDPR Art. 15/20) or anonymises it
//! (Art. 17), and logs each request by a hash of the email.

use crate::error::{AppError, FieldError};
use crate::retention::{anonymise_bookings, anonymise_enquiries, anonymise_orders};
use crate::routes::admin::SESSION_USER_ID_KEY;
use axum::{
    Json,
    extract::{Query, State},
    http::header,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use tower_sessions::Session;

#[derive(Debug, Deserialize)]
pub struct EmailQuery {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AnonymiseRequest {
    pub email: String,
    pub confirm_email: String,
}

fn normalised(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::Validation(vec![FieldError {
            field: "email".into(),
            message: "invalid".into(),
        }]));
    }
    Ok(email)
}

/// Ids of the not-yet-anonymised rows for this email, per table.
struct Matches {
    orders: Vec<i64>,
    bookings: Vec<i64>,
    enquiries: Vec<i64>,
    contact_messages: Vec<i64>,
}

async fn find(conn: &mut PgConnection, email: &str) -> Result<Matches, sqlx::Error> {
    Ok(Matches {
        orders: sqlx::query_scalar!(
            "SELECT id FROM orders WHERE lower(customer_email) = $1 AND anonymised_at IS NULL ORDER BY id",
            email
        )
        .fetch_all(&mut *conn)
        .await?,
        bookings: sqlx::query_scalar!(
            "SELECT id FROM event_bookings WHERE lower(customer_email) = $1 AND anonymised_at IS NULL ORDER BY id",
            email
        )
        .fetch_all(&mut *conn)
        .await?,
        enquiries: sqlx::query_scalar!(
            "SELECT id FROM catering_enquiries WHERE lower(email) = $1 AND anonymised_at IS NULL ORDER BY id",
            email
        )
        .fetch_all(&mut *conn)
        .await?,
        contact_messages: sqlx::query_scalar!(
            "SELECT id FROM contact_messages WHERE lower(email) = $1 ORDER BY id",
            email
        )
        .fetch_all(&mut *conn)
        .await?,
    })
}

/// Open orders and bookings for upcoming events must be finished or
/// cancelled before an erasure, or the owner couldn't fulfil them.
async fn blocking(conn: &mut PgConnection, m: &Matches) -> Result<Value, sqlx::Error> {
    let orders = sqlx::query_scalar!(
        "SELECT id FROM orders WHERE id = ANY($1) AND status NOT IN ('completed', 'cancelled') ORDER BY id",
        &m.orders
    )
    .fetch_all(&mut *conn)
    .await?;
    let bookings = sqlx::query_scalar!(
        r#"SELECT b.id FROM event_bookings b JOIN events e ON e.id = b.event_id
           WHERE b.id = ANY($1) AND b.status <> 'cancelled' AND e.starts_at > now()
           ORDER BY b.id"#,
        &m.bookings
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(json!({ "orders": orders, "bookings": bookings }))
}

fn is_blocked(blocking: &Value) -> bool {
    ["orders", "bookings"]
        .iter()
        .any(|key| blocking[key].as_array().is_some_and(|ids| !ids.is_empty()))
}

async fn log_request(
    conn: &mut PgConnection,
    session: &Session,
    kind: &str,
    email: &str,
    row_counts: Value,
) -> Result<(), AppError> {
    let admin_user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    sqlx::query!(
        r#"INSERT INTO privacy_requests (kind, email_sha256, row_counts, admin_user_id)
           VALUES ($1, encode(sha256(convert_to($2, 'UTF8')), 'hex'), $3, $4)"#,
        kind,
        email,
        row_counts,
        admin_user_id,
    )
    .execute(conn)
    .await?;
    Ok(())
}

fn counts(m: &Matches) -> Value {
    json!({
        "orders": m.orders.len(),
        "bookings": m.bookings.len(),
        "enquiries": m.enquiries.len(),
        "contact_messages": m.contact_messages.len(),
    })
}

/// GET /api/admin/privacy?email= — one summary row per record.
pub async fn search(
    State(pool): State<PgPool>,
    Query(query): Query<EmailQuery>,
) -> Result<Json<Value>, AppError> {
    let email = normalised(&query.email)?;
    let mut conn = pool.acquire().await?;
    let m = find(&mut conn, &email).await?;
    let blocking = blocking(&mut conn, &m).await?;

    let summary = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
             'orders', coalesce((SELECT jsonb_agg(jsonb_build_object(
                 'id', id, 'created_at', created_at, 'status', status, 'total', total)
                 ORDER BY id) FROM orders WHERE id = ANY($1)), '[]'),
             'bookings', coalesce((SELECT jsonb_agg(jsonb_build_object(
                 'id', b.id, 'event_id', b.event_id, 'event_slug', e.slug, 'created_at', b.created_at,
                 'status', b.status, 'party_size', b.party_size)
                 ORDER BY b.id) FROM event_bookings b JOIN events e ON e.id = b.event_id
                 WHERE b.id = ANY($2)), '[]'),
             'enquiries', coalesce((SELECT jsonb_agg(jsonb_build_object(
                 'id', ce.id, 'created_at', ce.created_at, 'event_type', ce.event_type,
                 'quote_status', q.status)
                 ORDER BY ce.id) FROM catering_enquiries ce
                 LEFT JOIN catering_quotes q ON q.enquiry_id = ce.id WHERE ce.id = ANY($3)), '[]'),
             'contact_messages', coalesce((SELECT jsonb_agg(jsonb_build_object(
                 'id', id, 'created_at', created_at, 'subject', subject)
                 ORDER BY id) FROM contact_messages WHERE id = ANY($4)), '[]')
           ) AS "summary!""#,
        &m.orders,
        &m.bookings,
        &m.enquiries,
        &m.contact_messages,
    )
    .fetch_one(&mut *conn)
    .await?;

    let mut body = summary;
    body["blocking"] = blocking;
    Ok(Json(body))
}

/// GET /api/admin/privacy/export?email= — every stored row, as a download.
pub async fn export(
    State(pool): State<PgPool>,
    session: Session,
    Query(query): Query<EmailQuery>,
) -> Result<impl IntoResponse, AppError> {
    let email = normalised(&query.email)?;
    let mut conn = pool.acquire().await?;
    let m = find(&mut conn, &email).await?;

    let data = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
             'orders', coalesce((SELECT jsonb_agg(to_jsonb(o) - 'public_token'
                 || jsonb_build_object('items', coalesce((SELECT jsonb_agg(to_jsonb(i) ORDER BY i.id)
                     FROM order_items i WHERE i.order_id = o.id), '[]'))
                 ORDER BY o.id) FROM orders o WHERE o.id = ANY($1)), '[]'),
             'event_bookings', coalesce((SELECT jsonb_agg(to_jsonb(b) - 'public_token'
                 || jsonb_build_object('guests', coalesce((SELECT jsonb_agg(to_jsonb(g) ORDER BY g.id)
                     FROM event_booking_guests g WHERE g.booking_id = b.id), '[]'))
                 ORDER BY b.id) FROM event_bookings b WHERE b.id = ANY($2)), '[]'),
             'catering_enquiries', coalesce((SELECT jsonb_agg(to_jsonb(ce)
                 || jsonb_build_object('quote', (SELECT to_jsonb(q) - 'public_token'
                     FROM catering_quotes q WHERE q.enquiry_id = ce.id))
                 ORDER BY ce.id) FROM catering_enquiries ce WHERE ce.id = ANY($3)), '[]'),
             'contact_messages', coalesce((SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id)
                 FROM contact_messages c WHERE c.id = ANY($4)), '[]'),
             'exported_at', now()
           ) AS "data!""#,
        &m.orders,
        &m.bookings,
        &m.enquiries,
        &m.contact_messages,
    )
    .fetch_one(&mut *conn)
    .await?;

    log_request(&mut conn, &session, "export", &email, counts(&m)).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"privacy-export.json\"",
            ),
        ],
        Json(data),
    ))
}

/// POST /api/admin/privacy/anonymise — erasure. Bookkeeping amounts stay
/// (retention.rs); contact messages and the email log for the address go.
pub async fn anonymise(
    State(pool): State<PgPool>,
    session: Session,
    Json(payload): Json<AnonymiseRequest>,
) -> Result<Json<Value>, AppError> {
    let email = normalised(&payload.email)?;
    if payload.confirm_email.trim().to_lowercase() != email {
        return Err(AppError::Validation(vec![FieldError {
            field: "confirm_email".into(),
            message: "mismatch".into(),
        }]));
    }

    let mut tx = pool.begin().await?;
    let m = find(&mut tx, &email).await?;
    let blocking = blocking(&mut tx, &m).await?;
    if is_blocked(&blocking) {
        return Err(AppError::Conflict(blocking));
    }

    let result = json!({
        "orders": anonymise_orders(&mut tx, &m.orders).await?,
        "bookings": anonymise_bookings(&mut tx, &m.bookings).await?,
        "enquiries": anonymise_enquiries(&mut tx, &m.enquiries).await?,
        "contact_messages": sqlx::query!(
            "DELETE FROM contact_messages WHERE id = ANY($1)",
            &m.contact_messages
        )
        .execute(&mut *tx)
        .await?
        .rows_affected(),
        "email_deliveries": sqlx::query!(
            "DELETE FROM email_deliveries WHERE lower(recipient) = $1",
            email
        )
        .execute(&mut *tx)
        .await?
        .rows_affected(),
    });
    log_request(&mut tx, &session, "anonymise", &email, result.clone()).await?;
    tx.commit().await?;

    Ok(Json(result))
}
