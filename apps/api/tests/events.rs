mod common;

use api::events::release_expired_holds;
use axum::http::StatusCode;
use common::{get, get_raw_with_cookie, get_with_cookie, insert_admin, patch_with_cookie, post, post_with_cookie, put_with_cookie, session_cookie};
use serde_json::{Value, json};
use sqlx::PgPool;

const SLUG: &str = "the-abula-experience-2026";
const ADDRESS: &str = "Secret Street 1, Utrecht";

/// A published event with two €15 plates. Returns (event id, abula id, jollof id).
async fn seed_event(pool: &PgPool, capacity: i32) -> (i64, i64, i64) {
    let event_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO events (slug, content, starts_at, area, address, capacity, max_party_size,
                            meal_orders_close_at, published_at)
        VALUES ($1, '{"en": {"title": "The Abula Experience", "terms": "No refunds after the cutoff."},
                      "nl": {"title": "De Abula-ervaring"}}',
                now() + interval '40 days', 'A home in Utrecht', $2, $3, 6,
                now() + interval '33 days', now())
        RETURNING id
        "#,
    )
    .bind(SLUG)
    .bind(ADDRESS)
    .bind(capacity)
    .fetch_one(pool)
    .await
    .unwrap();
    let mut plates = Vec::new();
    for (position, name) in ["Abula", "Smoky jollof rice with salad"].iter().enumerate() {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO event_plates (event_id, position, content, price) VALUES ($1, $2, $3, 15.00) RETURNING id",
        )
        .bind(event_id)
        .bind(position as i32)
        .bind(json!({ "en": { "name": name, "description": "" }, "nl": { "name": format!("{name} (nl)") } }))
        .fetch_one(pool)
        .await
        .unwrap();
        plates.push(id);
    }
    (event_id, plates[0], plates[1])
}

fn booking(guests: Value) -> Value {
    json!({
        "customer_name": "Amina Bello",
        "customer_email": "amina@example.com",
        "customer_phone": "0612345678",
        "guests": guests,
        "accept_terms": true,
        "locale": "nl",
    })
}

async fn book(pool: &PgPool, body: Value) -> (StatusCode, Value) {
    post(pool.clone(), &format!("/api/events/{SLUG}/bookings"), body).await
}

async fn booking_page(pool: &PgPool, token: &str) -> Value {
    let (status, body) = get(pool.clone(), &format!("/api/events/bookings/{token}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

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

#[sqlx::test]
async fn public_event_never_shows_the_address(pool: PgPool) {
    seed_event(&pool, 16).await;

    let (status, list) = get(pool.clone(), "/api/events?locale=nl").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["upcoming"][0]["title"], "De Abula-ervaring");
    assert_eq!(list["upcoming"][0]["seats_left"], 16);
    assert!(!list.to_string().contains(ADDRESS));

    let (status, event) = get(pool.clone(), &format!("/api/events/{SLUG}?locale=fr")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(event["title"], "The Abula Experience", "fr falls back to en");
    assert_eq!(event["plates"].as_array().unwrap().len(), 2);
    assert_eq!(event["meals_open"], true);
    assert!(!event.to_string().contains(ADDRESS));
}

#[sqlx::test]
async fn unpublished_events_are_hidden(pool: PgPool) {
    seed_event(&pool, 16).await;
    sqlx::query("UPDATE events SET published_at = NULL").execute(&pool).await.unwrap();

    let (status, _) = get(pool.clone(), &format!("/api/events/{SLUG}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = get(pool.clone(), "/api/events").await;
    assert_eq!(list["upcoming"].as_array().unwrap().len(), 0);
    let (status, _) = book(&pool, booking(json!([null]))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn booking_with_meals_creates_an_event_order_and_holds_seats(pool: PgPool) {
    let (_, abula, jollof) = seed_event(&pool, 16).await;

    let (status, created) = book(&pool, booking(json!([abula, abula, jollof, null]))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["status"], "awaiting_payment");

    let page = booking_page(&pool, created["token"].as_str().unwrap()).await;
    assert_eq!(page["total"], "45.00");
    assert_eq!(page["party_size"], 4);
    assert_eq!(page["address"], Value::Null, "no address before payment");
    assert_eq!(page["event"]["title"], "De Abula-ervaring", "the guest's locale");
    let order = &page["order"];
    assert_eq!(order["delivery_type"], "event");
    assert_eq!(order["total"], "45.00");
    assert_eq!(order["status"], "awaiting_payment");
    assert_eq!(order["items"].as_array().unwrap().len(), 2);
    assert_eq!(order["items"][0]["item_name"], "The Abula Experience — Abula");
    assert_eq!(order["items"][0]["quantity"], 2);
    assert_eq!(order["event_booking_token"], created["token"]);
    // Guests read plates in their language; the order and admin use English.
    assert_eq!(page["guests"][0]["plate_name"], "Abula (nl)");
    assert_eq!(order["items"][1]["item_name"], "The Abula Experience — Smoky jollof rice with salad");

    let (_, event) = get(pool.clone(), &format!("/api/events/{SLUG}")).await;
    assert_eq!(event["seats_left"], 12);
}

#[sqlx::test]
async fn free_rsvp_is_confirmed_with_the_address(pool: PgPool) {
    seed_event(&pool, 16).await;
    let (status, created) = book(&pool, booking(json!([null, null]))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["status"], "confirmed");

    let page = booking_page(&pool, created["token"].as_str().unwrap()).await;
    assert_eq!(page["address"], ADDRESS);
    assert_eq!(page["order"], Value::Null);
}

#[sqlx::test]
async fn validation_rules(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 16).await;

    let (status, body) = book(&pool, booking(json!([null, null, null, null, null, null, null]))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "max party size");
    assert!(body.to_string().contains("too_many"));

    let mut no_terms = booking(json!([abula]));
    no_terms["accept_terms"] = json!(false);
    let (status, body) = book(&pool, no_terms).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.to_string().contains("accept_terms"));

    let (status, _) = book(&pool, booking(json!([999_999]))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "unknown plate");

    sqlx::query("UPDATE events SET meal_orders_close_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, body) = book(&pool, booking(json!([abula]))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.to_string().contains("meals_closed"));
    let (status, _) = book(&pool, booking(json!([null]))).await;
    assert_eq!(status, StatusCode::CREATED, "RSVPs stay open after the meal cutoff");
}

#[sqlx::test]
async fn honeypot_fakes_success(pool: PgPool) {
    seed_event(&pool, 16).await;
    let mut body = booking(json!([null]));
    body["website"] = json!("http://spam.example");
    let (status, _) = book(&pool, body).await;
    assert_eq!(status, StatusCode::CREATED);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_bookings").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test]
async fn full_event_offers_the_waitlist(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 3).await;
    let (status, _) = book(&pool, booking(json!([abula, null]))).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = book(&pool, booking(json!([abula, abula]))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "not_enough_seats");
    assert_eq!(body["seats_left"], 1);

    let mut waitlist = booking(json!([abula, abula]));
    waitlist["join_waitlist"] = json!(true);
    let (status, created) = book(&pool, waitlist).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["status"], "waitlisted");
    let page = booking_page(&pool, created["token"].as_str().unwrap()).await;
    assert_eq!(page["order"], Value::Null);

    let (_, event) = get(pool.clone(), &format!("/api/events/{SLUG}")).await;
    assert_eq!(event["seats_left"], 1, "the waitlist holds no seats");
}

#[sqlx::test]
async fn racing_bookings_never_oversell(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 2).await;
    let (a, b) = tokio::join!(
        book(&pool, booking(json!([abula, abula]))),
        book(&pool, booking(json!([abula, abula]))),
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
}

#[sqlx::test]
async fn confirming_payment_confirms_the_booking(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 16).await;
    let cookie = login(&pool).await;
    let (_, created) = book(&pool, booking(json!([abula]))).await;
    let token = created["token"].as_str().unwrap().to_string();
    let order_id = booking_page(&pool, &token).await["order"]["id"].as_i64().unwrap();

    let (status, order) = get_with_cookie(pool.clone(), &format!("/api/admin/orders/{order_id}"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(order["event_title"], "The Abula Experience");

    let (status, body) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{order_id}/status"),
        json!({ "status": "processing" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let page = booking_page(&pool, &token).await;
    assert_eq!(page["status"], "confirmed");
    assert_eq!(page["address"], ADDRESS);

    let (_, order) = get_with_cookie(pool.clone(), &format!("/api/admin/orders/{order_id}"), Some(&cookie)).await;
    assert_eq!(order["next_statuses"], json!(["completed", "cancelled"]));
}

#[sqlx::test]
async fn cancelling_the_order_frees_the_seats(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 16).await;
    let cookie = login(&pool).await;
    let (_, created) = book(&pool, booking(json!([abula, null]))).await;
    let token = created["token"].as_str().unwrap().to_string();
    let order_id = booking_page(&pool, &token).await["order"]["id"].as_i64().unwrap();

    let (status, _) = patch_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{order_id}/status"),
        json!({ "status": "cancelled" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page = booking_page(&pool, &token).await;
    assert_eq!(page["status"], "cancelled");
    assert_eq!(page["cancel_reason"], "admin");
    let (_, event) = get(pool.clone(), &format!("/api/events/{SLUG}")).await;
    assert_eq!(event["seats_left"], 16);
}

#[sqlx::test]
async fn sweeper_releases_expired_holds_but_not_claimed_payments(pool: PgPool) {
    let (_, abula, _) = seed_event(&pool, 16).await;
    let (_, unpaid) = book(&pool, booking(json!([abula]))).await;
    let (_, claimed) = book(&pool, booking(json!([abula, abula]))).await;
    let claimed_token = claimed["token"].as_str().unwrap().to_string();
    let order_token = booking_page(&pool, &claimed_token).await["order"]["public_token"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, _) = post(pool.clone(), &format!("/api/orders/by-token/{order_token}/payment-claim"), json!({})).await;
    assert_eq!(status, StatusCode::OK);

    sqlx::query("UPDATE event_bookings SET hold_expires_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let released = release_expired_holds(&pool).await.unwrap();
    assert_eq!(released.len(), 1);
    assert!(release_expired_holds(&pool).await.unwrap().is_empty(), "runs once per booking");

    let page = booking_page(&pool, unpaid["token"].as_str().unwrap()).await;
    assert_eq!(page["status"], "cancelled");
    assert_eq!(page["cancel_reason"], "expired");
    assert_eq!(page["order"]["status"], "cancelled");

    let page = booking_page(&pool, &claimed_token).await;
    assert_eq!(page["status"], "awaiting_payment");
    assert_eq!(page["order"]["status"], "payment_claimed");
}

#[sqlx::test]
async fn owner_offers_seats_to_the_waitlist(pool: PgPool) {
    let (event_id, abula, _) = seed_event(&pool, 2).await;
    let cookie = login(&pool).await;
    let (_, first) = book(&pool, booking(json!([null, null]))).await;
    let mut waitlist = booking(json!([abula]));
    waitlist["join_waitlist"] = json!(true);
    let (_, waiting) = book(&pool, waitlist).await;
    let waiting_id: i64 = sqlx::query_scalar("SELECT id FROM event_bookings WHERE public_token = $1::uuid")
        .bind(waiting["token"].as_str().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();

    let (status, body, _) =
        post_with_cookie(pool.clone(), &format!("/api/admin/events/bookings/{waiting_id}/offer"), json!({}), Some(&cookie)).await;
    assert_eq!(status, StatusCode::CONFLICT, "no room yet");
    assert_eq!(body["error"], "not_enough_seats");

    let (status, _) = post(
        pool.clone(),
        &format!("/api/events/bookings/{}/cancel", first["token"].as_str().unwrap()),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "guest cancel");

    let (status, body, _) =
        post_with_cookie(pool.clone(), &format!("/api/admin/events/bookings/{waiting_id}/offer"), json!({}), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let page = booking_page(&pool, waiting["token"].as_str().unwrap()).await;
    assert_eq!(page["status"], "awaiting_payment");
    assert_eq!(page["order"]["total"], "15.00");

    let (_, bookings) =
        get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/bookings"), Some(&cookie)).await;
    assert_eq!(bookings["seats_taken"], 1);
    assert_eq!(bookings["plate_counts"][0]["plate_name"], "Abula");
    assert_eq!(bookings["plate_counts"][0]["held"], 1);
}

#[sqlx::test]
async fn guests_cannot_cancel_after_the_meal_cutoff(pool: PgPool) {
    seed_event(&pool, 16).await;
    let (_, created) = book(&pool, booking(json!([null]))).await;
    sqlx::query("UPDATE events SET meal_orders_close_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = post(
        pool.clone(),
        &format!("/api/events/bookings/{}/cancel", created["token"].as_str().unwrap()),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn admin_creates_publishes_and_previews_an_event(pool: PgPool) {
    let cookie = login(&pool).await;
    let input = json!({
        "content": { "en": { "title": "Àmàlà Night", "summary": "Swallow and soup" } },
        "starts_at": "2026-12-27T14:00",
        "ends_at": "2026-12-27T18:00",
        "area": "Utrecht",
        "address": ADDRESS,
        "capacity": 16,
        "max_party_size": 6,
        "meal_orders_close_at": "2026-12-20T23:59",
        "rsvp_closes_at": "",
        "payment_hold_hours": 48,
        "cover_image_url": null,
        "plates": [{ "id": null, "content": { "en": { "name": "Abula" } }, "price": "15.00", "is_available": true }],
    });
    let (status, created, _) = post_with_cookie(pool.clone(), "/api/admin/events", input.clone(), Some(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["slug"], "amala-night");
    assert_eq!(created["starts_at"], "2026-12-27T14:00");
    assert_eq!(created["published"], false);
    let id = created["id"].as_i64().unwrap();

    let (status, _) = get(pool.clone(), "/api/events/amala-night").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "drafts are private");
    let (status, preview) = get_with_cookie(pool.clone(), "/api/admin/preview/events/amala-night", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["published"], false);

    let (status, _, _) = post_with_cookie(pool.clone(), &format!("/api/admin/events/{id}/publish"), json!({}), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = get(pool.clone(), "/api/events/amala-night").await;
    assert_eq!(status, StatusCode::OK);

    let mut again = input.clone();
    again["plates"] = json!([]);
    let (status, second, _) = post_with_cookie(pool.clone(), "/api/admin/events", again, Some(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(second["slug"], "amala-night-2");

    let (status, photos) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/events/{id}/photos"),
        json!([{ "image_url": "http://localhost:8080/api/media/a.jpg", "caption": "The table" }]),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(photos["photos"][0]["caption"], "The table");

    let (status, _) = get(pool.clone(), "/api/admin/events").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Phase 32: gives the seeded plates the flyer's option groups.
async fn add_options(pool: &PgPool, abula: i64, jollof: i64) {
    let protein_abula = json!([{ "key": "protein", "label": { "en": "Protein", "nl": "Eiwit" }, "choices": [
        { "key": "assorted-meat", "label": { "en": "Assorted meat", "nl": "Gemengd vlees" } },
        { "key": "fish", "label": { "en": "Fish", "nl": "Vis" } }] }]);
    let jollof_groups = json!([
        { "key": "side", "label": { "en": "Side" }, "choices": [
            { "key": "fried-plantain", "label": { "en": "Fried plantain", "nl": "Gebakken bakbanaan" } },
            { "key": "moin-moin", "label": { "en": "Moin moin" } },
            { "key": "fresh-salad", "label": { "en": "Fresh salad" } }] },
        { "key": "protein", "label": { "en": "Protein" }, "choices": [
            { "key": "chicken", "label": { "en": "Chicken", "nl": "Kip" } },
            { "key": "turkey", "label": { "en": "Turkey" } }] }]);
    for (id, options) in [(abula, protein_abula), (jollof, jollof_groups)] {
        sqlx::query("UPDATE event_plates SET options = $2 WHERE id = $1")
            .bind(id)
            .bind(options)
            .execute(pool)
            .await
            .unwrap();
    }
}

fn guest(plate_id: i64, choices: Value) -> Value {
    json!({ "plate_id": plate_id, "choices": choices })
}

#[sqlx::test]
async fn plate_options_are_public_and_translated(pool: PgPool) {
    let (_, abula, jollof) = seed_event(&pool, 16).await;
    add_options(&pool, abula, jollof).await;
    let (_, event) = get(pool.clone(), &format!("/api/events/{SLUG}?locale=nl")).await;
    assert_eq!(event["plates"][0]["options"][0]["label"], "Eiwit");
    assert_eq!(event["plates"][0]["options"][0]["choices"][1], json!({ "key": "fish", "label": "Vis" }));
    assert_eq!(event["plates"][1]["options"][0]["choices"].as_array().unwrap().len(), 3);
    assert_eq!(event["plates"][1]["options"][1]["label"], "Protein", "nl falls back to English");
}

#[sqlx::test]
async fn choices_become_order_lines_and_show_on_the_booking(pool: PgPool) {
    let (event_id, abula, jollof) = seed_event(&pool, 16).await;
    add_options(&pool, abula, jollof).await;
    let cookie = login(&pool).await;

    let (status, created) = book(
        &pool,
        booking(json!([
            guest(abula, json!({ "protein": "fish" })),
            guest(abula, json!({ "protein": "assorted-meat" })),
            guest(jollof, json!({ "protein": "chicken", "side": "fried-plantain" })),
            guest(abula, json!({ "protein": "assorted-meat" })),
            { "plate_id": null },
        ])),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let page = booking_page(&pool, created["token"].as_str().unwrap()).await;
    assert_eq!(page["total"], "60.00");
    assert_eq!(page["guests"][0]["choices"], "Vis", "the booking's locale");
    assert_eq!(page["guests"][2]["choices"], "Gebakken bakbanaan, Kip", "group order");
    assert_eq!(page["guests"][4]["choices"], "");
    let items = page["order"]["items"].as_array().unwrap();
    let lines: Vec<(&str, i64)> = items
        .iter()
        .map(|item| (item["item_name"].as_str().unwrap(), item["quantity"].as_i64().unwrap()))
        .collect();
    assert_eq!(
        lines,
        [
            ("The Abula Experience — Abula (Fish)", 1),
            ("The Abula Experience — Abula (Assorted meat)", 2),
            ("The Abula Experience — Smoky jollof rice with salad (Fried plantain, Chicken)", 1),
        ]
    );

    let (_, bookings) =
        get_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/bookings"), Some(&cookie)).await;
    let counts: Vec<(Value, &str, i64)> = bookings["plate_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["plate_name"].clone(), c["choices"].as_str().unwrap(), c["held"].as_i64().unwrap()))
        .collect();
    assert!(counts.contains(&(json!("Abula"), "Assorted meat", 2)), "{counts:?}");
    assert!(counts.contains(&(json!("Abula"), "Fish", 1)), "{counts:?}");
    assert_eq!(bookings["bookings"][0]["guests"][2]["choices"], "Fried plantain, Chicken");

    let (status, _, csv) =
        get_raw_with_cookie(pool.clone(), &format!("/api/admin/events/{event_id}/bookings.csv"), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.contains("Abula (Fish); Abula (Assorted meat); Smoky jollof rice with salad (Fried plantain, Chicken)"), "{csv}");
}

#[sqlx::test]
async fn choices_are_validated(pool: PgPool) {
    let (_, abula, jollof) = seed_event(&pool, 16).await;
    add_options(&pool, abula, jollof).await;
    for (guests, message) in [
        (json!([guest(jollof, json!({ "protein": "chicken" }))]), "option_required"),
        (json!([guest(abula, json!({ "protein": "beef" }))]), "unknown_option"),
        (json!([guest(abula, json!({ "protein": "fish", "side": "moin-moin" }))]), "unknown_option"),
        (json!([{ "plate_id": null, "choices": { "protein": "fish" } }]), "unknown_option"),
        (json!([abula]), "option_required"),
    ] {
        let (status, body) = book(&pool, booking(guests.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{guests}");
        assert!(body.to_string().contains(message), "{guests}: {body}");
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_bookings").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test]
async fn admin_saves_plate_options_and_needs_an_address_to_publish(pool: PgPool) {
    let cookie = login(&pool).await;
    let mut input = json!({
        "content": { "en": { "title": "Jollof Sunday" } },
        "starts_at": "2026-12-27T14:00",
        "area": "Almere",
        "address": ADDRESS,
        "capacity": 16,
        "max_party_size": 6,
        "payment_hold_hours": 48,
        "plates": [{ "id": null, "content": { "en": { "name": "Jollof" } }, "price": "15.00", "is_available": true,
                     "options": [{ "label": { "en": "Protein" }, "choices": [{ "label": { "en": "Chicken" } }] }] }],
    });
    let (status, body, _) = post_with_cookie(pool.clone(), "/api/admin/events", input.clone(), Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "one choice isn't a choice");
    assert!(body.to_string().contains("plates[0].options[0].choices"), "{body}");

    input["plates"][0]["options"][0]["choices"] =
        json!([{ "label": { "en": "Chicken" } }, { "label": { "en": "Turkey", "nl": "Kalkoen" } }]);
    let (status, created, _) = post_with_cookie(pool.clone(), "/api/admin/events", input, Some(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let options = &created["plates"][0]["options"][0];
    assert_eq!(options["key"], "protein");
    assert_eq!(options["choices"][1]["key"], "turkey");
    assert_eq!(options["choices"][1]["label"]["nl"], "Kalkoen");

    let id = created["id"].as_i64().unwrap();
    sqlx::query("UPDATE events SET address = '' WHERE id = $1").bind(id).execute(&pool).await.unwrap();
    let (status, body, _) =
        post_with_cookie(pool.clone(), &format!("/api/admin/events/{id}/publish"), json!({}), Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.to_string().contains("address"), "{body}");
}
