//! Phase 38: the attendee list once an event's bookings close
//! (requirement.md Decisions 12–18): confirmed guests only, one row per
//! person, as JSON for the check-in sheet, as CSV, and emailed to the owner
//! once when bookings close.

use crate::email::{EmailAttachment, EmailConfig, OutgoingEmail};
use crate::error::AppError;
use crate::event_options::{choices_summary, parse_choices};
use crate::order_emails::Locale;
use crate::payment::payment_reference;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct AttendeeEvent {
    pub id: i64,
    pub slug: String,
    /// English, like the rest of the admin.
    pub title: String,
    pub starts_at: DateTime<Utc>,
    /// "Sat 6 Dec 2026, 18:00", Amsterdam time.
    pub starts_at_label: String,
    pub area: String,
    /// Decision 12: the same rule that stops new bookings.
    pub bookings_closed: bool,
    pub bookings_close_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Attendee {
    pub booker_name: String,
    /// 1-based position within the party.
    pub guest_no: i64,
    pub party_size: i32,
    /// `None` is "no meal".
    pub plate_name: Option<String>,
    /// English: "Fried plantain, Chicken".
    pub choices: String,
    pub phone: String,
    /// The booking's note, on the party's first guest only.
    pub note: Option<String>,
    pub payment_reference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttendeePlateCount {
    pub plate_name: Option<String>,
    pub choices: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct AttendeesResponse {
    pub event: AttendeeEvent,
    pub guests: Vec<Attendee>,
    pub plate_counts: Vec<AttendeePlateCount>,
    pub confirmed_guests: usize,
}

pub async fn load_attendees(pool: &PgPool, event_id: i64) -> Result<AttendeesResponse, AppError> {
    let event = sqlx::query!(
        r#"
        SELECT id, slug, area, content->'en'->>'title' as "title!",
               starts_at as "starts_at: DateTime<Utc>",
               to_char(starts_at AT TIME ZONE 'Europe/Amsterdam', 'Dy FMDD Mon YYYY, HH24:MI') as "starts_at_label!",
               LEAST(meal_orders_close_at, rsvp_closes_at, starts_at) as "bookings_close_at!: DateTime<Utc>",
               now() >= LEAST(meal_orders_close_at, rsvp_closes_at, starts_at) as "bookings_closed!"
        FROM events WHERE id = $1
        "#,
        event_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let rows = sqlx::query!(
        r#"
        SELECT b.customer_name, b.customer_phone, b.party_size, b.notes, b.order_id,
               g.plate_name, g.choices,
               ROW_NUMBER() OVER (PARTITION BY b.id ORDER BY g.position) as "guest_no!"
        FROM event_bookings b
        JOIN event_booking_guests g ON g.booking_id = b.id
        WHERE b.event_id = $1 AND b.status = 'confirmed'
        ORDER BY lower(b.customer_name), b.id, g.position
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    let guests: Vec<Attendee> = rows
        .into_iter()
        .map(|row| Attendee {
            note: if row.guest_no == 1 { row.notes.filter(|note| !note.trim().is_empty()) } else { None },
            booker_name: row.customer_name,
            guest_no: row.guest_no,
            party_size: row.party_size,
            plate_name: row.plate_name,
            choices: choices_summary(&parse_choices(&row.choices), &[], Locale::En),
            phone: row.customer_phone,
            payment_reference: row.order_id.map(payment_reference),
        })
        .collect();

    let mut plate_counts: Vec<AttendeePlateCount> = Vec::new();
    for guest in &guests {
        match plate_counts
            .iter_mut()
            .find(|count| count.plate_name == guest.plate_name && count.choices == guest.choices)
        {
            Some(count) => count.count += 1,
            None => plate_counts.push(AttendeePlateCount {
                plate_name: guest.plate_name.clone(),
                choices: guest.choices.clone(),
                count: 1,
            }),
        }
    }
    // Plates first by name, "No meal" last.
    plate_counts.sort_by(|a, b| {
        (a.plate_name.is_none(), &a.plate_name, &a.choices).cmp(&(b.plate_name.is_none(), &b.plate_name, &b.choices))
    });

    Ok(AttendeesResponse {
        confirmed_guests: guests.len(),
        event: AttendeeEvent {
            id: event.id,
            slug: event.slug,
            title: event.title,
            starts_at: event.starts_at,
            starts_at_label: event.starts_at_label,
            area: event.area,
            bookings_closed: event.bookings_closed,
            bookings_close_at: event.bookings_close_at,
        },
        guests,
        plate_counts,
    })
}

/// Quotes a CSV cell, and defuses a leading =, +, - or @, which would run
/// as a formula in a spreadsheet.
pub fn csv_field(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@']) { format!("'{value}") } else { value.to_string() };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// Decision 14. The byte-order mark makes Excel read accented names as
/// UTF-8; "Checked in" is empty, for ticking on paper.
pub fn attendees_csv(data: &AttendeesResponse) -> String {
    let mut csv = String::from("\u{feff}Checked in,Name,Guest,Plate,Choices,Phone,Payment reference,Note\n");
    for guest in &data.guests {
        let row = [
            String::new(),
            guest.booker_name.clone(),
            format!("{} of {}", guest.guest_no, guest.party_size),
            guest.plate_name.clone().unwrap_or_else(|| "No meal".into()),
            guest.choices.clone(),
            guest.phone.clone(),
            guest.payment_reference.clone().unwrap_or_default(),
            guest.note.clone().unwrap_or_default(),
        ];
        csv.push_str(&row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(","));
        csv.push('\n');
    }
    csv
}

pub fn plate_summary(data: &AttendeesResponse) -> String {
    data.plate_counts
        .iter()
        .map(|count| {
            let name = count.plate_name.as_deref().unwrap_or("No meal");
            match count.choices.as_str() {
                "" => format!("{name} × {}", count.count),
                choices => format!("{name} ({choices}) × {}", count.count),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Claims every published, live event whose bookings have just closed, so
/// the email goes out once even across restarts (Decision 17). The 24-hour
/// guard keeps events that closed long before deploy from all emailing at
/// once.
pub async fn claim_closed_events(pool: &PgPool) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        UPDATE events SET attendees_sent_at = now()
        WHERE attendees_sent_at IS NULL AND archived_at IS NULL AND published_at IS NOT NULL
          AND now() >= LEAST(meal_orders_close_at, rsvp_closes_at, starts_at)
          AND starts_at > now() - interval '24 hours'
        RETURNING id
        "#
    )
    .fetch_all(pool)
    .await
}

pub fn owner_email_body(data: &AttendeesResponse, admin_url: &str) -> String {
    format!(
        "Bookings for {} ({}) have closed.\n\nConfirmed guests: {}\n\nPlates to cook:\n{}\n\n\
         Check-in sheet (print or save as PDF):\n{admin_url}/events/{}/attendees\n\n\
         The attendee list is attached as a spreadsheet (CSV).",
        data.event.title,
        data.event.starts_at_label,
        data.confirmed_guests,
        plate_summary(data),
        data.event.id,
    )
}

/// Emails the owner the attendee list of each newly closed event. Nothing
/// is sent for an event without confirmed guests; it stays claimed. Logs
/// carry event ids only (Decision 18).
pub async fn notify_closed_events(pool: &PgPool, email: &EmailConfig) {
    let claimed = match claim_closed_events(pool).await {
        Ok(claimed) => claimed,
        Err(error) => {
            tracing::error!(%error, "attendee list claim failed");
            return;
        }
    };
    for event_id in claimed {
        let data = match load_attendees(pool, event_id).await {
            Ok(data) => data,
            Err(_) => {
                tracing::error!(event_id, "attendee list load failed");
                continue;
            }
        };
        if data.confirmed_guests == 0 {
            tracing::info!(event_id, "bookings closed with no confirmed guests");
            continue;
        }
        let Some(business_address) = email.business_notify_address() else { continue };
        tracing::info!(event_id, guests = data.confirmed_guests, "sending attendee list");
        email.spawn_send(OutgoingEmail {
            to: business_address,
            subject: format!("Attendee list: {} ({})", data.event.title, data.event.starts_at_label),
            body: owner_email_body(&data, email.admin_url()),
            reply_to: None,
            track: None,
            attachments: vec![EmailAttachment {
                filename: format!("{}-attendees.csv", data.event.slug),
                content_type: "text/csv; charset=utf-8".into(),
                bytes: attendees_csv(&data).into_bytes(),
            }],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_and_defuses_formulas() {
        assert_eq!(csv_field("Amina"), "Amina");
        assert_eq!(csv_field("Abula; Jollof, extra"), "\"Abula; Jollof, extra\"");
        assert_eq!(csv_field("=HYPERLINK(1)"), "'=HYPERLINK(1)");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
