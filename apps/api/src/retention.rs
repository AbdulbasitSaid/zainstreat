//! Phase 33 Decisions 14-15: how long personal data is kept, and the
//! anonymisation shared by the nightly sweeper and the admin Privacy tool
//! (`routes/admin/privacy.rs`).
//!
//! Records the business must keep for its books (Dutch fiscal record-keeping,
//! 7 years) are anonymised, never deleted: amounts, items, dates and status
//! stay so totals and cook counts still add up. Everything else is kept for
//! 12 months.

use std::time::Duration;

use sqlx::{PgConnection, PgPool};

/// Orders, paid event bookings and enquiries whose quote was accepted.
pub const BOOKKEEPING_MONTHS: i32 = 7 * 12;
/// Unpaid cancelled orders, unpaid or free event bookings (after the event),
/// enquiries without an accepted quote, contact messages.
pub const SHORT_MONTHS: i32 = 12;
/// `email_deliveries`, the outgoing email log.
pub const EMAIL_LOG_DAYS: i32 = 90;
/// `login_failures`, only needed for the 15-minute lockout window.
pub const LOGIN_FAILURE_HOURS: i32 = 24;

const SWEEP_EVERY: Duration = Duration::from_secs(24 * 3600);

/// Overwrites the personal columns of these orders. A fresh public token
/// means an old tracking link stops showing anything.
pub async fn anonymise_orders(conn: &mut PgConnection, ids: &[i64]) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query!(
        r#"UPDATE orders SET
               customer_name = 'Anonymised',
               customer_email = 'anonymised+' || id || '@invalid',
               customer_phone = '',
               delivery_address = NULL,
               notes = NULL,
               public_token = gen_random_uuid(),
               anonymised_at = now()
           WHERE id = ANY($1) AND anonymised_at IS NULL"#,
        ids
    )
    .execute(conn)
    .await?
    .rows_affected())
}

/// Event bookings: guest rows only hold plate snapshots, so they stay.
pub async fn anonymise_bookings(conn: &mut PgConnection, ids: &[i64]) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query!(
        r#"UPDATE event_bookings SET
               customer_name = 'Anonymised',
               customer_email = 'anonymised+' || id || '@invalid',
               customer_phone = '',
               notes = NULL,
               public_token = gen_random_uuid(),
               anonymised_at = now()
           WHERE id = ANY($1) AND anonymised_at IS NULL"#,
        ids
    )
    .execute(conn)
    .await?
    .rows_affected())
}

/// Catering enquiries and the customer-typed parts of their quotes.
pub async fn anonymise_enquiries(conn: &mut PgConnection, ids: &[i64]) -> Result<u64, sqlx::Error> {
    sqlx::query!(
        r#"UPDATE catering_quotes SET
               delivery_address = NULL,
               decline_reason = NULL,
               public_token = gen_random_uuid()
           WHERE enquiry_id = ANY($1)"#,
        ids
    )
    .execute(&mut *conn)
    .await?;
    Ok(sqlx::query!(
        r#"UPDATE catering_enquiries SET
               name = 'Anonymised',
               email = 'anonymised+' || id || '@invalid',
               phone = '',
               location = '',
               message = NULL,
               anonymised_at = now()
           WHERE id = ANY($1) AND anonymised_at IS NULL"#,
        ids
    )
    .execute(conn)
    .await?
    .rows_affected())
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SweepCounts {
    pub orders: u64,
    pub bookings: u64,
    pub enquiries: u64,
    pub contact_messages: u64,
    pub email_deliveries: u64,
    pub login_failures: u64,
}

/// One pass of the Decision 14 table, in one transaction.
pub async fn sweep(pool: &PgPool) -> Result<SweepCounts, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let order_ids = sqlx::query_scalar!(
        r#"SELECT id FROM orders
           WHERE anonymised_at IS NULL
             AND (created_at < now() - make_interval(months => $1)
                  OR (status = 'cancelled' AND payment_confirmed_at IS NULL
                      AND created_at < now() - make_interval(months => $2)))"#,
        BOOKKEEPING_MONTHS,
        SHORT_MONTHS,
    )
    .fetch_all(&mut *tx)
    .await?;

    // A confirmed booking with an order was paid for; anything else (free
    // RSVP, waitlist, cancelled, released) is kept 12 months after the event.
    let booking_ids = sqlx::query_scalar!(
        r#"SELECT b.id FROM event_bookings b JOIN events e ON e.id = b.event_id
           WHERE b.anonymised_at IS NULL
             AND (b.created_at < now() - make_interval(months => $1)
                  OR ((b.status <> 'confirmed' OR b.order_id IS NULL)
                      AND e.starts_at < now() - make_interval(months => $2)))"#,
        BOOKKEEPING_MONTHS,
        SHORT_MONTHS,
    )
    .fetch_all(&mut *tx)
    .await?;

    let enquiry_ids = sqlx::query_scalar!(
        r#"SELECT ce.id FROM catering_enquiries ce
           LEFT JOIN catering_quotes q ON q.enquiry_id = ce.id
           WHERE ce.anonymised_at IS NULL
             AND (ce.created_at < now() - make_interval(months => $1)
                  OR (q.status IS DISTINCT FROM 'accepted'
                      AND greatest(ce.created_at, q.updated_at) < now() - make_interval(months => $2)))"#,
        BOOKKEEPING_MONTHS,
        SHORT_MONTHS,
    )
    .fetch_all(&mut *tx)
    .await?;

    let counts = SweepCounts {
        orders: anonymise_orders(&mut tx, &order_ids).await?,
        bookings: anonymise_bookings(&mut tx, &booking_ids).await?,
        enquiries: anonymise_enquiries(&mut tx, &enquiry_ids).await?,
        contact_messages: sqlx::query!(
            "DELETE FROM contact_messages WHERE created_at < now() - make_interval(months => $1)",
            SHORT_MONTHS
        )
        .execute(&mut *tx)
        .await?
        .rows_affected(),
        email_deliveries: sqlx::query!(
            "DELETE FROM email_deliveries WHERE created_at < now() - make_interval(days => $1)",
            EMAIL_LOG_DAYS
        )
        .execute(&mut *tx)
        .await?
        .rows_affected(),
        login_failures: sqlx::query!(
            "DELETE FROM login_failures WHERE created_at < now() - make_interval(hours => $1)",
            LOGIN_FAILURE_HOURS
        )
        .execute(&mut *tx)
        .await?
        .rows_affected(),
    };

    tx.commit().await?;
    Ok(counts)
}

/// Runs `sweep` once a day, starting now. Only started when
/// `RETENTION_SWEEP_ENABLED=true` (`main.rs`): the periods wait on the
/// client's accountant before they run on production.
pub fn spawn_retention_sweeper(pool: PgPool) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SWEEP_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match sweep(&pool).await {
                // Counts only, never which rows (Decision 14).
                Ok(c) => tracing::info!(
                    orders = c.orders,
                    bookings = c.bookings,
                    enquiries = c.enquiries,
                    contact_messages = c.contact_messages,
                    email_deliveries = c.email_deliveries,
                    login_failures = c.login_failures,
                    "retention sweep"
                ),
                Err(error) => tracing::error!(%error, "retention sweep failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the periods to requirement.md Decision 14 and the privacy policy
    /// copy: change all three together.
    #[test]
    fn periods_match_the_privacy_policy() {
        assert_eq!(BOOKKEEPING_MONTHS, 84);
        assert_eq!(SHORT_MONTHS, 12);
        assert_eq!(EMAIL_LOG_DAYS, 90);
        assert_eq!(LOGIN_FAILURE_HOURS, 24);
    }
}
