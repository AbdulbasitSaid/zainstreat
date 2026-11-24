//! Phase 38: the attendee list once bookings close (requirement.md
//! Decisions 12–17).

mod common;

use api::attendees::claim_closed_events;
use axum::http::StatusCode;
use common::{get_raw_with_cookie, get_with_cookie, insert_admin, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

async fn login(pool: &PgPool) -> String {
    insert_admin(pool, "owner@example.com", "correct horse battery staple").await;
    let (_status, _body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    session_cookie(&set_cookie.expect("login should set a session cookie")).to_string()
}

/// An event whose bookings close `close_in` from now, with one confirmed
/// party of three, one waitlisted and one cancelled booking.
async fn seed(pool: &PgPool, close_in: &str) -> i64 {
    let event_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO events (slug, content, starts_at, area, address, capacity, rsvp_closes_at, published_at)
        VALUES ('abula', '{"en": {"title": "The Abula Experience"}}', now() + interval '10 days',
                'Almere', 'Grote Markt 1', 40, now() + $1::interval, now())
        RETURNING id
        "#,
    )
    .bind(close_in)
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::raw_sql(&format!(
        r#"
        INSERT INTO event_bookings (id, event_id, status, cancel_reason, customer_name, customer_email,
                                    customer_phone, party_size, notes)
        VALUES (1, {event_id}, 'confirmed', NULL, 'Zara Bello', 'zara@example.com', '+31 6 1', 3, 'Nut allergy'),
               (2, {event_id}, 'waitlisted', NULL, 'Amina Okafor', 'amina@example.com', '+31 6 2', 2, NULL),
               (3, {event_id}, 'cancelled', 'guest', 'Bisi Ade', 'bisi@example.com', '+31 6 3', 1, NULL);
        INSERT INTO event_booking_guests (booking_id, position, plate_name, unit_price, choices)
        VALUES (1, 0, 'Abula', 20, '[{{"group": "protein", "choice": "beef", "label": "Beef"}}]'),
               (1, 1, 'Abula', 20, '[{{"group": "protein", "choice": "beef", "label": "Beef"}}]'),
               (1, 2, NULL, NULL, '[]'),
               (2, 0, 'Abula', 20, '[]'), (2, 1, 'Abula', 20, '[]'),
               (3, 0, 'Abula', 20, '[]');
        "#
    ))
    .execute(pool)
    .await
    .unwrap();
    event_id
}

#[sqlx::test]
async fn the_attendee_list_is_confirmed_guests_one_per_row(pool: PgPool) {
    let event_id = seed(&pool, "-1 hour").await;
    let cookie = login(&pool).await;

    let (status, body) =
        get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/attendees"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["event"]["title"], "The Abula Experience");
    assert_eq!(body["event"]["bookings_closed"], true);
    assert_eq!(body["confirmed_guests"], 3);
    let guests = body["guests"].as_array().unwrap();
    assert_eq!(guests.len(), 3);
    assert!(guests.iter().all(|guest| guest["booker_name"] == "Zara Bello"));
    assert_eq!(guests[0]["note"], "Nut allergy");
    assert_eq!(guests[1]["note"], serde_json::Value::Null);
    assert_eq!(guests[2]["plate_name"], serde_json::Value::Null);
    assert_eq!(body["plate_counts"][0]["count"], 2);
    assert_eq!(body["plate_counts"][1]["plate_name"], serde_json::Value::Null);

    let (status, headers, bytes) =
        get_raw_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/attendees.csv"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-disposition"], "attachment; filename=\"abula-attendees.csv\"");
    let csv = String::from_utf8(bytes).unwrap();
    assert!(csv.starts_with("\u{feff}Checked in,Name,Guest,Plate,Choices,Phone,Payment reference,Note\n"), "{csv}");
    assert_eq!(csv.lines().count(), 4, "{csv}");
    assert!(csv.contains(",Zara Bello,1 of 3,Abula,"), "{csv}");
    assert!(csv.contains("Nut allergy"));
    assert!(csv.contains(",Zara Bello,3 of 3,No meal,"), "{csv}");
    assert!(!csv.contains("Amina"));

    let (_, bookings) =
        get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/bookings"), Some(&cookie)).await;
    assert_eq!(bookings["bookings_closed"], true);
}

#[sqlx::test]
async fn the_attendee_list_needs_an_admin(pool: PgPool) {
    let event_id = seed(&pool, "-1 hour").await;
    let (status, _) = get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/attendees"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) =
        get_raw_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/attendees.csv"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn an_open_event_is_not_closed_or_claimed(pool: PgPool) {
    let event_id = seed(&pool, "2 days").await;
    let cookie = login(&pool).await;
    let (_, body) =
        get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/attendees"), Some(&cookie)).await;
    assert_eq!(body["event"]["bookings_closed"], false);
    assert_eq!(claim_closed_events(&pool, None).await.unwrap(), Vec::<i64>::new());
}

#[sqlx::test]
async fn a_closed_event_is_claimed_once(pool: PgPool) {
    let event_id = seed(&pool, "-1 hour").await;
    assert_eq!(claim_closed_events(&pool, None).await.unwrap(), vec![event_id]);
    assert_eq!(claim_closed_events(&pool, None).await.unwrap(), Vec::<i64>::new(), "never twice");
}

#[sqlx::test]
async fn archived_unpublished_and_long_past_events_are_never_claimed(pool: PgPool) {
    seed(&pool, "-1 hour").await;
    sqlx::query("UPDATE events SET archived_at = now()").execute(&pool).await.unwrap();
    assert!(claim_closed_events(&pool, None).await.unwrap().is_empty());

    sqlx::query("UPDATE events SET archived_at = NULL, published_at = NULL").execute(&pool).await.unwrap();
    assert!(claim_closed_events(&pool, None).await.unwrap().is_empty());

    // Closed and started days before deploy: no burst of old emails.
    sqlx::query("UPDATE events SET published_at = now(), starts_at = now() - interval '3 days', rsvp_closes_at = NULL")
        .execute(&pool)
        .await
        .unwrap();
    assert!(claim_closed_events(&pool, None).await.unwrap().is_empty());
}
