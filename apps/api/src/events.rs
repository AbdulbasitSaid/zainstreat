//! Event bookings (Phase 31): seat counting, turning a booking's meals
//! into an order, the guest/owner notifications, and the sweeper that
//! releases unpaid holds. The HTTP handlers live in `routes/events.rs` and
//! `routes/admin/events.rs`.

use crate::email::{DeliveryRef, EmailConfig, OutgoingEmail};
use crate::event_emails::{BookingEmail, BookingEmailKind, GuestLine, LocalizedEvent, booking_email};
use crate::event_options::{choices_summary, parse_choices, parse_groups};
use crate::order_emails::Locale;
use crate::payment::{PaymentConfig, TikkieInstructions, payment_reference};
use crate::push::{OwnerPush, PushConfig};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use std::time::Duration;
use uuid::Uuid;

/// One field of an event's or plate's per-locale `content`, falling back to
/// English when the locale's value is missing or blank (Decision 15).
pub fn localized(content: &Value, locale: Locale, key: &str) -> String {
    let pick = |code: &str| {
        content
            .get(code)
            .and_then(|fields| fields.get(key))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
    };
    pick(locale.as_str())
        .or_else(|| pick("en"))
        .unwrap_or_default()
        .to_string()
}

/// Seats held by bookings that are awaiting payment or confirmed
/// (Decision 6). Waitlisted and cancelled bookings hold none.
pub async fn seats_taken(conn: &mut PgConnection, event_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(party_size), 0)::BIGINT as "taken!"
        FROM event_bookings
        WHERE event_id = $1 AND status IN ('awaiting_payment', 'confirmed')
        "#,
        event_id
    )
    .fetch_one(conn)
    .await
}

/// Lowercase ASCII words joined by `-`, for slugs made from a title.
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.to_lowercase().chars() {
        let mapped = match c {
            // Combining tone marks (the grave on "ẹ̀") belong to the letter.
            '\u{0300}'..='\u{036F}' => continue,
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' | 'ẹ' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'ọ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ṣ' => 's',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if mapped == '-' {
            if !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
        } else {
            slug.push(mapped);
        }
    }
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() { "event".to_string() } else { slug }
}

/// Moves a booking with meals to `awaiting_payment`: creates its order (one
/// line per plate) and starts the payment hold (Decisions 1, 8). The
/// caller holds the event row lock and has checked the seats.
pub async fn start_payment(conn: &mut PgConnection, booking_id: i64) -> Result<i64, sqlx::Error> {
    let booking = sqlx::query!(
        r#"
        SELECT b.customer_name, b.customer_email, b.customer_phone, b.locale, b.notes,
               b.party_size, e.content, e.payment_hold_hours,
               to_char(e.starts_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "starts_at!"
        FROM event_bookings b
        JOIN events e ON e.id = b.event_id
        WHERE b.id = $1
        "#,
        booking_id
    )
    .fetch_one(&mut *conn)
    .await?;

    // One line per plate and choices (Phase 32 Decision 5).
    let plates = sqlx::query!(
        r#"
        SELECT plate_name as "plate_name!", unit_price as "unit_price!", choices,
               COUNT(*)::INTEGER as "quantity!"
        FROM event_booking_guests
        WHERE booking_id = $1 AND plate_name IS NOT NULL
        GROUP BY plate_name, unit_price, choices
        ORDER BY MIN(position)
        "#,
        booking_id
    )
    .fetch_all(&mut *conn)
    .await?;

    let title = localized(&booking.content, Locale::En, "title");
    let total = plates.iter().fold(Decimal::ZERO, |acc, plate| {
        acc + plate.unit_price * Decimal::from(plate.quantity)
    });
    // The event context rides along in the notes, like a catering quote's
    // (Phase 23 Decision 7).
    let mut notes = format!(
        "Event: {title} on {}, {} guest(s)",
        booking.starts_at, booking.party_size
    );
    if let Some(note) = booking.notes.as_deref().filter(|n| !n.trim().is_empty()) {
        notes.push_str(&format!("\n{note}"));
    }

    let order_id = sqlx::query_scalar!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total, locale)
        VALUES ($1, $2, $3, 'event', NULL, $4, $5, 0, $5, $6)
        RETURNING id
        "#,
        booking.customer_name,
        booking.customer_email,
        booking.customer_phone,
        notes,
        total,
        booking.locale,
    )
    .fetch_one(&mut *conn)
    .await?;

    for plate in &plates {
        let choices = choices_summary(&parse_choices(&plate.choices), &[], Locale::En);
        let item_name = if choices.is_empty() {
            format!("{title} — {}", plate.plate_name)
        } else {
            format!("{title} — {} ({choices})", plate.plate_name)
        };
        sqlx::query!(
            r#"
            INSERT INTO order_items (order_id, item_name, unit_price, quantity, subtotal)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            order_id,
            item_name,
            plate.unit_price,
            plate.quantity,
            plate.unit_price * Decimal::from(plate.quantity),
        )
        .execute(&mut *conn)
        .await?;
    }

    sqlx::query!(
        r#"
        UPDATE event_bookings
        SET status = 'awaiting_payment', order_id = $2,
            hold_expires_at = now() + make_interval(hours => $3),
            cancel_reason = NULL, cancelled_at = NULL, updated_at = now()
        WHERE id = $1
        "#,
        booking_id,
        order_id,
        booking.payment_hold_hours,
    )
    .execute(&mut *conn)
    .await?;

    Ok(order_id)
}

/// Confirms a booking with no meals, or one whose payment came in.
pub async fn confirm(conn: &mut PgConnection, booking_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE event_bookings
        SET status = 'confirmed', confirmed_at = now(), hold_expires_at = NULL,
            cancel_reason = NULL, cancelled_at = NULL, updated_at = now()
        WHERE id = $1
        "#,
        booking_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Whether any guest on the booking has a meal.
pub async fn has_meals(conn: &mut PgConnection, booking_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event_booking_guests WHERE booking_id = $1 AND plate_name IS NOT NULL) as "exists!""#,
        booking_id
    )
    .fetch_one(conn)
    .await
}

/// Cancels a booking and its unfinished order, freeing the seats.
pub async fn cancel(conn: &mut PgConnection, booking_id: i64, reason: &str) -> Result<(), sqlx::Error> {
    let order_id = sqlx::query_scalar!(
        r#"
        UPDATE event_bookings
        SET status = 'cancelled', cancel_reason = $2, cancelled_at = now(),
            hold_expires_at = NULL, updated_at = now()
        WHERE id = $1
        RETURNING order_id
        "#,
        booking_id,
        reason,
    )
    .fetch_one(&mut *conn)
    .await?;

    if let Some(order_id) = order_id {
        sqlx::query!(
            r#"
            UPDATE orders SET status = 'cancelled', cancelled_at = now(), updated_at = now()
            WHERE id = $1 AND status NOT IN ('completed', 'cancelled')
            "#,
            order_id
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// A booking with everything its emails and pages need.
pub struct BookingDetails {
    pub id: i64,
    pub event_id: i64,
    pub token: Uuid,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub party_size: i32,
    pub notes: Option<String>,
    pub locale: Locale,
    pub order_id: Option<i64>,
    pub order_token: Option<Uuid>,
    pub order_status: Option<String>,
    pub hold_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub hold_expires_local: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub event_slug: String,
    pub event_content: Value,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub starts_local: String,
    pub area: String,
    pub address: String,
    pub cover_image_url: Option<String>,
    /// Guests can cancel until the meal cutoff, or until the event starts
    /// when there's no cutoff (Decision 14).
    pub guest_can_cancel: bool,
    pub guests: Vec<GuestLine>,
    pub total: Decimal,
}

pub async fn load_booking(pool: &PgPool, booking_id: i64) -> Result<Option<BookingDetails>, sqlx::Error> {
    let Some(row) = sqlx::query!(
        r#"
        SELECT b.id, b.event_id, b.public_token, b.status, b.cancel_reason, b.customer_name,
               b.customer_email, b.customer_phone, b.party_size, b.notes, b.locale, b.order_id,
               o.public_token as "order_token?", o.status as "order_status?",
               b.hold_expires_at as "hold_expires_at: chrono::DateTime<chrono::Utc>",
               to_char(b.hold_expires_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as hold_expires_local,
               b.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               e.slug, e.content, e.area, e.address, e.cover_image_url,
               e.starts_at as "starts_at: chrono::DateTime<chrono::Utc>",
               to_char(e.starts_at AT TIME ZONE 'Europe/Amsterdam', 'DD-MM-YYYY HH24:MI') as "starts_local!",
               (now() < COALESCE(e.meal_orders_close_at, e.starts_at)) as "guest_can_cancel!"
        FROM event_bookings b
        JOIN events e ON e.id = b.event_id
        LEFT JOIN orders o ON o.id = b.order_id
        WHERE b.id = $1
        "#,
        booking_id
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let guests: Vec<GuestLine> = sqlx::query!(
        r#"
        SELECT g.plate_name, g.unit_price, g.choices, p.content as "plate_content?",
               p.options as "plate_options?"
        FROM event_booking_guests g
        LEFT JOIN event_plates p ON p.id = g.plate_id
        WHERE g.booking_id = $1
        ORDER BY g.position
        "#,
        booking_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|guest| GuestLine {
        plate_name: guest.plate_name,
        unit_price: guest.unit_price,
        plate_content: guest.plate_content,
        choices: parse_choices(&guest.choices),
        plate_options: guest.plate_options.as_ref().map(parse_groups).unwrap_or_default(),
    })
    .collect();
    let total = guests
        .iter()
        .filter_map(|guest| guest.unit_price)
        .fold(Decimal::ZERO, |acc, price| acc + price);

    Ok(Some(BookingDetails {
        id: row.id,
        event_id: row.event_id,
        token: row.public_token,
        status: row.status,
        cancel_reason: row.cancel_reason,
        customer_name: row.customer_name,
        customer_email: row.customer_email,
        customer_phone: row.customer_phone,
        party_size: row.party_size,
        notes: row.notes,
        locale: Locale::parse(&row.locale).unwrap_or(Locale::En),
        order_id: row.order_id,
        order_token: row.order_token,
        order_status: row.order_status,
        hold_expires_at: row.hold_expires_at,
        hold_expires_local: row.hold_expires_local,
        created_at: row.created_at,
        event_slug: row.slug,
        event_content: row.content,
        starts_at: row.starts_at,
        starts_local: row.starts_local,
        area: row.area,
        address: row.address,
        cover_image_url: row.cover_image_url,
        guest_can_cancel: row.guest_can_cancel,
        guests,
        total,
    }))
}

/// The event's Tikkie request for an event order that's still unpaid,
/// while the link is valid: through the end of `tikkie_valid_until` in
/// Amsterdam (Phase 34 Decisions 14-15). `None` for every other order.
pub async fn tikkie_for_order(pool: &PgPool, order_id: i64) -> Result<Option<TikkieInstructions>, sqlx::Error> {
    sqlx::query_as!(
        TikkieInstructions,
        r#"
        SELECT e.tikkie_url as "url!", e.tikkie_valid_until as "valid_until!: chrono::NaiveDate", o.total as amount
        FROM orders o
        JOIN event_bookings b ON b.order_id = o.id
        JOIN events e ON e.id = b.event_id
        WHERE o.id = $1 AND o.status = 'awaiting_payment'
          AND e.tikkie_url IS NOT NULL
          AND now() < ((e.tikkie_valid_until + 1)::timestamp AT TIME ZONE 'Europe/Amsterdam')
        "#,
        order_id
    )
    .fetch_optional(pool)
    .await
}

/// What happened to a booking, for `notify`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookingEvent {
    /// A new booking from the public form.
    Created,
    /// The owner offered seats to a waitlisted or cancelled booking.
    Offered,
    /// The owner confirmed the payment.
    PaymentConfirmed,
    /// The hold ran out (the sweeper).
    Released,
    CancelledByGuest,
    CancelledByAdmin,
}

/// The guest's email for the booking's current status, plus the owner's
/// email and push where the owner didn't cause it (Decision 19). Sent
/// after the commit; failures are only logged, like every other email.
pub async fn notify(
    pool: &PgPool,
    email: &EmailConfig,
    push: &PushConfig,
    payment: &PaymentConfig,
    booking_id: i64,
    what: BookingEvent,
) {
    let booking = match load_booking(pool, booking_id).await {
        Ok(Some(booking)) => booking,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(%error, booking_id, "failed to load booking for notifications");
            return;
        }
    };
    let offered = what == BookingEvent::Offered;
    let kind = match (what, booking.status.as_str()) {
        (BookingEvent::Released, _) => Some(BookingEmailKind::Released),
        (BookingEvent::CancelledByGuest | BookingEvent::CancelledByAdmin, _) => {
            Some(BookingEmailKind::Cancelled)
        }
        (_, "awaiting_payment") => Some(BookingEmailKind::AwaitingPayment { offered }),
        (_, "confirmed") => Some(BookingEmailKind::Confirmed { offered }),
        (_, "waitlisted") => Some(BookingEmailKind::Waitlisted),
        _ => None,
    };

    let title = localized(&booking.event_content, Locale::En, "title");

    if let Some(kind) = kind {
        let instructions = booking.order_id.and_then(|id| payment.instructions(id));
        let tikkie = match booking.order_id {
            Some(order_id) => tikkie_for_order(pool, order_id).await.unwrap_or_else(|error| {
                tracing::error!(%error, booking_id, "failed to load the Tikkie link; sending bank details only");
                None
            }),
            None => None,
        };
        let event = Locale::ALL.map(|locale| LocalizedEvent {
            title: localized(&booking.event_content, locale, "title"),
            terms: localized(&booking.event_content, locale, "terms"),
        });
        let (subject, body) = booking_email(
            &BookingEmail {
                kind,
                customer_name: &booking.customer_name,
                token: booking.token,
                event,
                starts_at: &booking.starts_local,
                area: &booking.area,
                address: &booking.address,
                party_size: booking.party_size,
                guests: &booking.guests,
                total: booking.total,
                hold_expires_at: booking.hold_expires_local.as_deref(),
                payment: instructions.as_ref(),
                tikkie: tikkie.as_ref(),
            },
            email.site_url(),
        );
        // `email_deliveries` rows hang off an order, so only bookings with
        // one are tracked for admin's "may not have been delivered" notice.
        let track = booking.order_id.map(|order_id| DeliveryRef {
            pool: pool.clone(),
            order_id: Some(order_id),
            quote_id: None,
            kind: if matches!(kind, BookingEmailKind::AwaitingPayment { .. }) { "invoice" } else { "status" },
        });
        email.spawn_send(OutgoingEmail {
            to: booking.customer_email.clone(),
            subject,
            body,
            reply_to: None,
            track,
        });
    }

    // The owner hears about what guests and the clock did, not about her
    // own actions.
    let owner = match what {
        BookingEvent::Created if booking.status == "waitlisted" => Some((
            format!("Waitlist — {title}"),
            format!("{} joined the waitlist for {} seat(s)", booking.customer_name, booking.party_size),
        )),
        BookingEvent::Created => Some((
            format!("New booking — {title}"),
            match booking.order_id {
                Some(order_id) => format!(
                    "{} booked {} seat(s) · €{} awaiting payment ({})",
                    booking.customer_name,
                    booking.party_size,
                    booking.total,
                    payment_reference(order_id)
                ),
                None => format!(
                    "{} booked {} seat(s), no meals (confirmed)",
                    booking.customer_name, booking.party_size
                ),
            },
        )),
        BookingEvent::CancelledByGuest => Some((
            format!("Booking cancelled — {title}"),
            format!("{} cancelled their {} seat(s)", booking.customer_name, booking.party_size),
        )),
        BookingEvent::Released => Some((
            format!("Unpaid booking released — {title}"),
            format!(
                "{}'s {} seat(s) were released after the payment window",
                booking.customer_name, booking.party_size
            ),
        )),
        _ => None,
    };

    if let Some((push_title, push_body)) = owner {
        let url = format!("/events/{}?tab=bookings", booking.event_id);
        push.spawn_notify_owner(
            pool,
            OwnerPush {
                title: push_title.clone(),
                body: push_body.clone(),
                url: url.clone(),
                tag: format!("event-booking-{}", booking.id),
            },
        );
        if let Some(business_address) = email.business_notify_address() {
            let mut body = format!(
                "{push_body}.\n\nGuest: {} · {} · {}\n",
                booking.customer_name, booking.customer_phone, booking.customer_email
            );
            if let Some(note) = &booking.notes {
                body.push_str(&format!("Note: {note}\n"));
            }
            body.push_str(&format!("\n{}{url}", email.admin_url()));
            email.spawn_send(OutgoingEmail {
                to: business_address,
                subject: push_title,
                body,
                reply_to: Some(booking.customer_email.clone()),
                track: None,
            });
        }
    }
}

/// Cancels bookings whose payment hold ran out and whose order is still
/// `awaiting_payment` (Decision 8). A guest who pressed "I've paid" moved
/// the order to `payment_claimed`, so they're never released. One
/// statement, so it's safe to run again after a restart.
pub async fn release_expired_holds(pool: &PgPool) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        WITH expired AS (
            UPDATE orders o
            SET status = 'cancelled', cancelled_at = now(), updated_at = now()
            FROM event_bookings b
            WHERE b.order_id = o.id AND b.status = 'awaiting_payment'
              AND b.hold_expires_at < now() AND o.status = 'awaiting_payment'
            RETURNING o.id
        )
        UPDATE event_bookings
        SET status = 'cancelled', cancel_reason = 'expired', cancelled_at = now(),
            hold_expires_at = NULL, updated_at = now()
        WHERE order_id IN (SELECT id FROM expired)
        RETURNING id
        "#
    )
    .fetch_all(pool)
    .await
}

const SWEEP_EVERY: Duration = Duration::from_secs(300);

/// Runs `release_expired_holds` every five minutes for the life of the
/// process, notifying each released guest and the owner.
pub fn spawn_hold_sweeper(pool: PgPool, email: EmailConfig, push: PushConfig, payment: PaymentConfig) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SWEEP_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match release_expired_holds(&pool).await {
                Ok(released) => {
                    for booking_id in released {
                        tracing::info!(booking_id, "released unpaid event booking");
                        notify(&pool, &email, &push, &payment, booking_id, BookingEvent::Released).await;
                    }
                }
                Err(error) => tracing::error!(%error, "event hold sweep failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slugs_are_ascii_words() {
        assert_eq!(slugify("The Abula Experience"), "the-abula-experience");
        assert_eq!(slugify("  Àmàlà & Gbẹ̀gìrì night! "), "amala-gbegiri-night");
        assert_eq!(slugify("!!!"), "event");
    }

    #[test]
    fn localized_falls_back_to_english() {
        let content = json!({ "en": { "title": "Lunch" }, "nl": { "title": "  " }, "fr": { "title": "Déjeuner" } });
        assert_eq!(localized(&content, Locale::Nl, "title"), "Lunch");
        assert_eq!(localized(&content, Locale::Fr, "title"), "Déjeuner");
        assert_eq!(localized(&content, Locale::Fr, "story"), "");
    }
}
