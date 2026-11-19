//! Phase 37: rentals in the cart and the delivery fee (requirement.md
//! Decisions 3–17). The delivery backend is a fake, so nothing here touches
//! PDOK or Google.

mod common;

use api::delivery::{DeliveryConfig, FakeBackend};
use axum::http::StatusCode;
use common::{
    TEST_ADDRESS_ID, build_test_app_with_delivery, get, get_with_cookie, insert_admin, post, post_on,
    post_with_cookie, put_with_cookie, session_cookie, test_delivery_backend,
};
use serde_json::{Value, json};
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

async fn seed(pool: &PgPool) {
    sqlx::query("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO menu_items (id, category_id, name, price) VALUES (1, 1, 'Jollof Rice', 13.00)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO rentals (id, name, price, is_available) VALUES \
         (1, 'Chafing Dish – 10 L', 15.00, true), (2, 'Cooler', 10.00, false)",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Delivery on, from the fake backend's "origin" address.
async fn enable_delivery(pool: &PgPool, max_distance_km: Option<&str>) {
    sqlx::query(
        "UPDATE delivery_settings SET origin_address = 'Grote Markt 1, 1315JG Almere', \
         origin_lat = 52.3702, origin_lon = 5.2141, delivery_enabled = true, \
         max_distance_km = $1::numeric",
    )
    .bind(max_distance_km)
    .execute(pool)
    .await
    .unwrap();
}

async fn days_from_today(pool: &PgPool, days: i32) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT to_char((now() AT TIME ZONE 'Europe/Amsterdam')::date + $1, 'YYYY-MM-DD')",
    )
    .bind(days)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn payload(items: Value) -> Value {
    json!({
        "customer_name": "Ada Lovelace",
        "customer_email": "ada@example.com",
        "customer_phone": "+31 6 1234 5678",
        "delivery_type": "pickup",
        "notes": null,
        "items": items,
    })
}

fn delivery_payload(expected_fee: &str) -> Value {
    let mut body = payload(json!([{ "menu_item_id": 1, "quantity": 2 }]));
    body["delivery_type"] = json!("delivery");
    body["delivery_address_id"] = json!(TEST_ADDRESS_ID);
    body["delivery_address_extra"] = json!("3rd floor");
    body["expected_delivery_fee"] = json!(expected_fee);
    body
}

fn field_errors(body: &Value) -> Vec<(String, String)> {
    body["fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .map(|f| (f["field"].as_str().unwrap().to_string(), f["message"].as_str().unwrap().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn has_error(body: &Value, field: &str, message: &str) -> bool {
    field_errors(body).iter().any(|(f, m)| f == field && m == message)
}

async fn order_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM orders").fetch_one(pool).await.unwrap()
}

#[sqlx::test]
async fn rentals_and_food_order_together_with_a_rental_date(pool: PgPool) {
    seed(&pool).await;
    let date = days_from_today(&pool, 3).await;
    let mut body = payload(json!([
        { "menu_item_id": 1, "quantity": 1 },
        { "rental_id": 1, "quantity": 2 },
    ]));
    body["rental_date"] = json!(date);

    let (status, order) = post(pool.clone(), "/api/orders", body).await;

    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(order["subtotal"], "43.00");
    assert_eq!(order["total"], "43.00");
    assert_eq!(order["rental_date"], json!(date));
    assert_eq!(order["items"][1]["kind"], "rental");
    assert_eq!(order["items"][1]["item_name"], "Chafing Dish – 10 L");
    assert_eq!(order["items"][1]["unit_price"], "15.00");

    let (kind, rental_id): (String, Option<i64>) =
        sqlx::query_as("SELECT kind, rental_id FROM order_items WHERE item_name = 'Chafing Dish – 10 L'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((kind.as_str(), rental_id), ("rental", Some(1)));

    // The tracking page reads the same.
    let token = order["public_token"].as_str().unwrap();
    let (_, tracked) = get(pool.clone(), &format!("/api/orders/by-token/{token}")).await;
    assert_eq!(tracked["rental_date"], json!(date));
    assert_eq!(tracked["items"][1]["kind"], "rental");
}

#[sqlx::test]
async fn rental_date_rules(pool: PgPool) {
    seed(&pool).await;
    let rental = json!([{ "rental_id": 1, "quantity": 1 }]);

    let (status, body) = post(pool.clone(), "/api/orders", payload(rental.clone())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "rental_date", "required"), "{body}");

    let mut today = payload(rental.clone());
    today["rental_date"] = json!(days_from_today(&pool, 0).await);
    let (_, body) = post(pool.clone(), "/api/orders", today).await;
    assert!(has_error(&body, "rental_date", "too_soon"), "{body}");

    let mut far = payload(rental);
    far["rental_date"] = json!(days_from_today(&pool, 366).await);
    let (_, body) = post(pool.clone(), "/api/orders", far).await;
    assert!(has_error(&body, "rental_date", "too_far"), "{body}");

    let mut food = payload(json!([{ "menu_item_id": 1, "quantity": 1 }]));
    food["rental_date"] = json!(days_from_today(&pool, 3).await);
    let (_, body) = post(pool.clone(), "/api/orders", food).await;
    assert!(has_error(&body, "rental_date", "not_allowed"), "{body}");
    assert_eq!(order_count(&pool).await, 0);
}

#[sqlx::test]
async fn rental_lines_are_checked(pool: PgPool) {
    seed(&pool).await;
    let date = days_from_today(&pool, 3).await;

    let mut unavailable = payload(json!([{ "rental_id": 2, "quantity": 1 }]));
    unavailable["rental_date"] = json!(date);
    let (status, body) = post(pool.clone(), "/api/orders", unavailable).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "items[0].rental_id", "unavailable"), "{body}");

    let mut both = payload(json!([{ "menu_item_id": 1, "rental_id": 1, "quantity": 1 }]));
    both["rental_date"] = json!(date);
    let (_, body) = post(pool.clone(), "/api/orders", both).await;
    assert!(has_error(&body, "items[0]", "invalid"), "{body}");

    let mut too_many = payload(json!([{ "rental_id": 1, "quantity": 100 }]));
    too_many["rental_date"] = json!(date);
    let (_, body) = post(pool.clone(), "/api/orders", too_many).await;
    assert!(has_error(&body, "items[0].quantity", "too_many"), "{body}");
}

#[sqlx::test]
async fn delivery_starts_switched_off(pool: PgPool) {
    seed(&pool).await;

    let (status, info) = get(pool.clone(), "/api/delivery/info").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info, json!({ "enabled": false, "rate_per_km": "1.00", "minimum_fee": "0.00", "max_distance_km": null }));

    let (status, body) = post(pool.clone(), "/api/orders", delivery_payload("12.30")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "delivery_type", "unavailable"), "{body}");

    let (status, body) = post(pool.clone(), "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "delivery_unavailable");
}

#[sqlx::test]
async fn quote_shows_distance_and_fee_but_never_the_origin(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;

    let (status, suggestions) = get(pool.clone(), "/api/delivery/suggest?q=stationsplein").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(suggestions[0]["id"], TEST_ADDRESS_ID);

    let (status, quote) = post(pool.clone(), "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;
    assert_eq!(status, StatusCode::OK, "{quote}");
    assert_eq!(quote["distance_km"], "12.3");
    assert_eq!(quote["fee"], "12.30");
    assert_eq!(quote["estimated_by"], "route");
    assert_eq!(quote["address"], "Stationsplein 1, 1315NT Almere");

    for (uri, body) in [
        ("/api/delivery/info", get(pool.clone(), "/api/delivery/info").await.1),
        ("/api/delivery/quote", quote),
    ] {
        let text = body.to_string().to_lowercase();
        assert!(!text.contains("origin") && !text.contains("grote markt") && !text.contains("52.37"), "{uri}: {text}");
    }

    let (status, body) = get(pool.clone(), "/api/delivery/suggest?q=ab").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[sqlx::test]
async fn delivery_order_stores_the_server_fee(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;

    let (status, order) = post(pool.clone(), "/api/orders", delivery_payload("12.30")).await;

    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(order["delivery_fee"], "12.30");
    assert_eq!(order["delivery_distance_km"], "12.3");
    assert_eq!(order["delivery_estimated_by"], "route");
    assert_eq!(order["delivery_address"], "Stationsplein 1, 1315NT Almere, 3rd floor");
    assert_eq!(order["subtotal"], "26.00");
    assert_eq!(order["total"], "38.30");
    assert_eq!(order["delivery_fee_pending"], false);

    let rate: String = sqlx::query_scalar("SELECT delivery_rate_per_km::text FROM orders").fetch_one(&pool).await.unwrap();
    assert_eq!(rate, "1.00");
}

#[sqlx::test]
async fn a_changed_fee_is_confirmed_before_ordering(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;

    let (status, body) = post(pool.clone(), "/api/orders", delivery_payload("10.00")).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, json!({ "error": "delivery_fee_changed", "fee": "12.30", "distance_km": "12.3" }));
    assert_eq!(order_count(&pool).await, 0);

    let mut no_expected = delivery_payload("12.30");
    no_expected["expected_delivery_fee"] = Value::Null;
    let (_, body) = post(pool.clone(), "/api/orders", no_expected).await;
    assert!(has_error(&body, "expected_delivery_fee", "required"), "{body}");
}

#[sqlx::test]
async fn minimum_fee_and_maximum_distance_apply(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, Some("10")).await;

    let (status, body) = post(pool.clone(), "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "out_of_range");
    let (_, body) = post(pool.clone(), "/api/orders", delivery_payload("12.30")).await;
    assert!(has_error(&body, "delivery_address_id", "out_of_range"), "{body}");

    sqlx::query("UPDATE delivery_settings SET max_distance_km = NULL, minimum_fee = 15.00, rate_per_km = 1.10")
        .execute(&pool)
        .await
        .unwrap();
    let (_, quote) = post(pool.clone(), "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;
    assert_eq!(quote["fee"], "15.00");
}

#[sqlx::test]
async fn google_down_falls_back_to_the_straight_line(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;
    let app = build_test_app_with_delivery(
        pool.clone(),
        DeliveryConfig::fake(FakeBackend { route_km: None, ..test_delivery_backend() }),
    )
    .await;

    let (status, quote) = post_on(app, "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;

    assert_eq!(status, StatusCode::OK, "{quote}");
    assert_eq!(quote["estimated_by"], "straight_line");
    // ~0.5 km apart as the crow flies, × 1.3.
    assert_eq!(quote["distance_km"], "0.7");
}

#[sqlx::test]
async fn typed_address_only_when_lookup_is_down(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;
    let mut typed = delivery_payload("0");
    typed["delivery_address_id"] = Value::Null;
    typed["expected_delivery_fee"] = Value::Null;
    typed["delivery_address"] = json!("Somewhere 12, Almere");

    // PDOK answers: the lookup must be used.
    let (status, body) = post(pool.clone(), "/api/orders", typed.clone()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "delivery_address_id", "required"), "{body}");

    // PDOK down: the order goes through with the fee to be confirmed.
    let down = DeliveryConfig::fake(FakeBackend { lookup_down: true, ..test_delivery_backend() });
    let app = build_test_app_with_delivery(pool.clone(), down).await;
    let (status, order) = post_on(app.clone(), "/api/orders", typed).await;
    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(order["delivery_fee_pending"], true);
    assert_eq!(order["delivery_fee"], "0");
    assert_eq!(order["delivery_address"], "Somewhere 12, Almere, 3rd floor");

    let (status, body) = post_on(app.clone(), "/api/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "address_lookup_down");

    // No paying until the owner sets the fee.
    let token = order["public_token"].as_str().unwrap();
    let (status, body) = post(pool.clone(), &format!("/api/orders/by-token/{token}/payment-claim"), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "delivery_fee_pending");

    // The owner sets it from the (now working) lookup.
    let cookie = login(&pool).await;
    let id = order["id"].as_i64().unwrap();
    let (_, detail) = get_with_cookie(pool.clone(), &format!("/api/admin/orders/{id}"), Some(&cookie)).await;
    assert_eq!(detail["delivery_fee_pending"], true);
    let (status, detail) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/delivery-fee"),
        json!({ "updated_at": detail["updated_at"], "address_id": TEST_ADDRESS_ID }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["delivery_fee_pending"], false);
    assert_eq!(detail["delivery_fee"], "12.30");
    assert_eq!(detail["total"], "38.30");
    assert_eq!(detail["delivery_estimated_by"], "route");

    let (status, claimed) = post(pool.clone(), &format!("/api/orders/by-token/{token}/payment-claim"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["status"], "payment_claimed");

    // Once a payment is claimed, the fee is fixed.
    let (_, detail) = get_with_cookie(pool.clone(), &format!("/api/admin/orders/{id}"), Some(&cookie)).await;
    let (status, _) = put_with_cookie(
        pool.clone(),
        &format!("/api/admin/orders/{id}/delivery-fee"),
        json!({ "updated_at": detail["updated_at"], "fee": "5.00" }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn owner_can_set_a_manual_fee(pool: PgPool) {
    seed(&pool).await;
    enable_delivery(&pool, None).await;
    let (_, order) = post(pool.clone(), "/api/orders", delivery_payload("12.30")).await;
    let cookie = login(&pool).await;
    let id = order["id"].as_i64().unwrap();
    let (_, detail) = get_with_cookie(pool.clone(), &format!("/api/admin/orders/{id}"), Some(&cookie)).await;

    let uri = format!("/api/admin/orders/{id}/delivery-fee");
    let (status, updated) =
        put_with_cookie(pool.clone(), &uri, json!({ "updated_at": detail["updated_at"], "fee": "9.50" }), Some(&cookie)).await;

    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["delivery_fee"], "9.50");
    assert_eq!(updated["total"], "35.50");
    assert_eq!(updated["delivery_estimated_by"], "manual");

    // A stale form conflicts.
    let (status, _) =
        put_with_cookie(pool.clone(), &uri, json!({ "updated_at": detail["updated_at"], "fee": "8.00" }), Some(&cookie)).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn admin_delivery_settings(pool: PgPool) {
    let (status, _) = get_with_cookie(pool.clone(), "/api/admin/delivery-settings", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let cookie = login(&pool).await;
    let (status, settings) = get_with_cookie(pool.clone(), "/api/admin/delivery-settings", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings["delivery_enabled"], false);
    assert_eq!(settings["origin_located"], false);

    let save = |settings: &Value, extra: Value| {
        let mut body = json!({
            "updated_at": settings["updated_at"],
            "rate_per_km": "1.00",
            "minimum_fee": "0.00",
            "max_distance_km": null,
            "delivery_enabled": true,
        });
        body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        body
    };

    // Can't switch on without a located origin (Decision 6).
    let (status, body) =
        put_with_cookie(pool.clone(), "/api/admin/delivery-settings", save(&settings, json!({})), Some(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "delivery_enabled", "origin_required"), "{body}");

    // The owner's "Test an address" works while delivery is off.
    sqlx::query("UPDATE delivery_settings SET origin_lat = 52.3702, origin_lon = 5.2141, origin_address = 'x'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, quote, _) =
        post_with_cookie(pool.clone(), "/api/admin/delivery/quote", json!({ "address_id": TEST_ADDRESS_ID }), Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK, "{quote}");
    assert_eq!(quote["fee"], "12.30");

    let (status, saved) = put_with_cookie(
        pool.clone(),
        "/api/admin/delivery-settings",
        save(&settings, json!({ "origin_address_id": "adr-test-origin", "rate_per_km": "1.50" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["origin_address"], "Grote Markt 1, 1315JG Almere");
    assert_eq!(saved["delivery_enabled"], true);
    assert_eq!(saved["rate_per_km"], "1.50");

    let (_, info) = get(pool.clone(), "/api/delivery/info").await;
    assert_eq!(info["enabled"], true);
    assert_eq!(info["rate_per_km"], "1.50");

    // The first form is now stale.
    let (status, _) =
        put_with_cookie(pool.clone(), "/api/admin/delivery-settings", save(&settings, json!({})), Some(&cookie)).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, body) = put_with_cookie(
        pool.clone(),
        "/api/admin/delivery-settings",
        save(&saved, json!({ "rate_per_km": "-1", "max_distance_km": "0" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(has_error(&body, "rate_per_km", "invalid") && has_error(&body, "max_distance_km", "invalid"), "{body}");
}

#[sqlx::test]
async fn seeded_origin_text_is_located_on_first_use(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("UPDATE delivery_settings SET origin_address = 'Grote Markt 1, 1315JG Almere'")
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login(&pool).await;

    let (_, settings) = get_with_cookie(pool.clone(), "/api/admin/delivery-settings", Some(&cookie)).await;

    assert_eq!(settings["origin_located"], true);
    let lat: Option<f64> = sqlx::query_scalar("SELECT origin_lat FROM delivery_settings").fetch_one(&pool).await.unwrap();
    assert_eq!(lat, Some(52.3702));
}
