//! Phase 38: menu translations (requirement.md Decisions 1–9, 20). DeepL is
//! a fake that answers "[nl] text", so nothing here touches the network.

mod common;

use api::translation::{FakeBackend, TranslationConfig};
use api::translation_sweep::sweep_once;
use axum::http::StatusCode;
use common::{build_test_app_with_translation, get, insert_admin, post, post_with_cookie, send_json_on, session_cookie};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::Ordering;

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

/// An unpublished item, so its saves write the live columns directly.
async fn seed_item(pool: &PgPool) {
    sqlx::query("INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes')").execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO menu_items (id, category_id, name, description, image_url, published_at) \
         VALUES (1, 1, 'Jollof Rice', 'Smoky party rice', '/images/jollof.jpg', NULL)",
    )
    .execute(pool)
    .await
    .unwrap();
}

fn item_fields(name: &str, large_price: &str) -> Value {
    json!({
        "category_id": 1,
        "name": name,
        "description": "Smoky party rice",
        "price": null,
        "price_options": [
            { "label": "Small", "price": "8.00" },
            { "label": "Large", "price": large_price },
        ],
        "image_url": "/images/jollof.jpg",
        "is_featured": false,
        "min_quantity": 1,
    })
}

async fn save_item(app: &axum::Router, cookie: &str, mut fields: Value, draft_updated_at: Value) -> Value {
    fields["draft_updated_at"] = draft_updated_at;
    let (status, body) =
        send_json_on(app.clone(), "PUT", "/api/admin/menu-items/1/draft", Some(fields), Some(cookie)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// Saves like the form's autosave, then presses Publish; returns the item
/// as admin sees it afterwards.
async fn save_and_publish(app: &axum::Router, cookie: &str, fields: Value, draft_updated_at: Value) -> Value {
    let saved = save_item(app, cookie, fields, draft_updated_at).await;
    let (status, body) = send_json_on(
        app.clone(),
        "POST",
        "/api/admin/publish",
        Some(json!({ "entries": [{ "kind": "menu_item", "id": 1, "draft_updated_at": saved["draft_updated_at"] }] })),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, item) = send_json_on(app.clone(), "GET", "/api/admin/menu-items/1", None, Some(cookie)).await;
    assert_eq!(status, StatusCode::OK);
    item
}

/// The form sends back what it loaded: the item's and each option's.
fn with_translations(mut fields: Value, from: &Value) -> Value {
    fields["translations"] = from["translations"].clone();
    for index in 0..2 {
        fields["price_options"][index]["translations"] = from["price_options"][index]["translations"].clone();
    }
    fields
}

fn fake(count: u64) -> (Arc<FakeBackend>, TranslationConfig) {
    let fake = Arc::new(FakeBackend::new(count, 1_000_000));
    (fake.clone(), TranslationConfig::fake(fake))
}

#[sqlx::test]
async fn autosave_sends_nothing_and_publish_translates(pool: PgPool) {
    seed_item(&pool).await;
    let cookie = login(&pool).await;
    let (fake, config) = fake(0);
    let app = build_test_app_with_translation(pool.clone(), config).await;

    // Autosaves while typing: no DeepL.
    let first = save_item(&app, &cookie, item_fields("Jollof R", "14.00"), Value::Null).await;
    let saved = save_item(&app, &cookie, item_fields("Jollof Rice", "14.00"), first["draft_updated_at"].clone()).await;
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert_eq!(saved["translation_status"], json!({ "nl": "missing", "fr": "missing" }));

    let body = save_and_publish(&app, &cookie, item_fields("Jollof Rice", "14.00"), saved["draft_updated_at"].clone()).await;
    assert_eq!(body["translations"]["nl"]["name"], "[nl] Jollof Rice");
    assert_eq!(body["translations"]["fr"]["description"], "[fr] Smoky party rice");
    assert_eq!(body["price_options"][1]["translations"]["nl"]["label"], "[nl] Large");
    assert_eq!(body["translation_status"], json!({ "nl": "ok", "fr": "ok" }));
    // One request per language for the item and its options together.
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);

    sqlx::query("UPDATE categories SET published_at = now()").execute(&pool).await.unwrap();
    let (status, items) = get(pool.clone(), "/api/menu-items?locale=nl").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(items[0]["name"], "[nl] Jollof Rice");
    assert_eq!(items[0]["description"], "[nl] Smoky party rice");
    assert_eq!(items[0]["price_options"][1]["label"], "[nl] Large");
    assert_eq!(items[0]["price_options"][1]["labels"]["fr"], "[fr] Large");
    assert_eq!(items[0]["names"], json!({ "en": "Jollof Rice", "nl": "[nl] Jollof Rice", "fr": "[fr] Jollof Rice" }));

    let (_, items) = get(pool.clone(), "/api/menu-items?locale=xx").await;
    assert_eq!(items[0]["name"], "Jollof Rice");
    let (_, items) = get(pool.clone(), "/api/menu-items").await;
    assert_eq!(items[0]["name"], "Jollof Rice");
}

#[sqlx::test]
async fn a_price_only_change_sends_nothing(pool: PgPool) {
    seed_item(&pool).await;
    let cookie = login(&pool).await;
    let (fake, config) = fake(0);
    let app = build_test_app_with_translation(pool.clone(), config).await;

    let first = save_and_publish(&app, &cookie, item_fields("Jollof Rice", "14.00"), Value::Null).await;
    let fields = with_translations(item_fields("Jollof Rice", "15.50"), &first);
    let after = save_and_publish(&app, &cookie, fields, first["draft_updated_at"].clone()).await;
    assert_eq!(after["price_options"][1]["price"], "15.50");
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2, "no DeepL call for a price change");
}

#[sqlx::test]
async fn an_owner_edit_is_kept_when_the_english_changes(pool: PgPool) {
    seed_item(&pool).await;
    let cookie = login(&pool).await;
    let (_fake, config) = fake(0);
    let app = build_test_app_with_translation(pool.clone(), config).await;

    let mut fields = item_fields("Jollof Rice", "14.00");
    fields["translations"] = json!({ "nl": { "name": "Jollof rijst", "description": "Rokerige rijst", "edited": true } });
    let first = save_and_publish(&app, &cookie, fields, Value::Null).await;
    assert_eq!(first["translations"]["nl"]["name"], "Jollof rijst");
    assert_eq!(first["translations"]["nl"]["manual"], true);
    assert_eq!(first["translations"]["fr"]["name"], "[fr] Jollof Rice");

    let fields = with_translations(item_fields("Jollof Rice Special", "14.00"), &first);
    let second = save_and_publish(&app, &cookie, fields, first["draft_updated_at"].clone()).await;
    assert_eq!(second["translations"]["nl"]["name"], "Jollof rijst");
    assert_eq!(second["translations"]["fr"]["name"], "[fr] Jollof Rice Special");
    assert_eq!(second["translation_status"]["nl"], "manual_stale");

    // Re-translate: the form drops the locale, then publishes.
    let mut fields = with_translations(item_fields("Jollof Rice Special", "14.00"), &second);
    fields["translations"].as_object_mut().unwrap().remove("nl");
    let third = save_and_publish(&app, &cookie, fields, second["draft_updated_at"].clone()).await;
    assert_eq!(third["translations"]["nl"]["name"], "[nl] Jollof Rice Special");
}

#[sqlx::test]
async fn past_the_stop_line_nothing_is_sent_and_the_owner_types_it(pool: PgPool) {
    seed_item(&pool).await;
    let cookie = login(&pool).await;
    let (fake, config) = fake(850_000);
    let app = build_test_app_with_translation(pool.clone(), config).await;

    let mut fields = item_fields("Jollof Rice", "14.00");
    fields["translations"] = json!({ "nl": { "name": "Jollof rijst", "edited": true } });
    let body = save_and_publish(&app, &cookie, fields, Value::Null).await;
    assert_eq!(body["translations"]["nl"]["name"], "Jollof rijst");
    assert!(body["translations"].get("fr").is_none());
    assert_eq!(body["translation_status"]["fr"], "missing");
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);

    // The owner raises the line: the next publish goes through.
    let (status, settings) = send_json_on(app.clone(), "GET", "/api/admin/translations/status", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings["usage"], json!({ "count": 850_000, "limit": 1_000_000 }));
    assert_eq!(settings["stop_line"], 800_000);
    let (status, _) = send_json_on(
        app.clone(),
        "PUT",
        "/api/admin/translations/settings",
        Some(json!({
            "auto_translate": true,
            "stop_at_percent": 95,
            "keep_words": settings["keep_words"],
            "updated_at": settings["updated_at"],
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let fields = with_translations(item_fields("Jollof Rice", "14.00"), &body);
    let body = save_and_publish(&app, &cookie, fields, body["draft_updated_at"].clone()).await;
    assert_eq!(body["translations"]["fr"]["name"], "[fr] Jollof Rice");
}

#[sqlx::test]
async fn the_sweep_backfills_everything_once(pool: PgPool) {
    sqlx::raw_sql(
        "INSERT INTO categories (id, name, description) VALUES (1, 'Rice Dishes', 'Rice and more'); \
         INSERT INTO menu_items (id, category_id, name, image_url) VALUES (1, 1, 'Fried Rice', '/x.jpg'); \
         INSERT INTO menu_item_price_options (menu_item_id, label, price) VALUES (1, 'Large', 12); \
         INSERT INTO rentals (id, name, price) VALUES (1, 'Cooler', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    // A draft waiting to be published is left to Publish.
    sqlx::query(
        r#"UPDATE categories SET draft = '{"name": "Rice", "description": null}',
                                 draft_updated_at = now() - interval '10 minutes'"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let (fake, config) = fake(0);

    let sent = sweep_once(&pool, &config).await.unwrap();
    assert!(sent > 0);
    let item: Value = sqlx::query_scalar("SELECT translations FROM menu_items WHERE id = 1").fetch_one(&pool).await.unwrap();
    assert_eq!(item["nl"]["name"], "[nl] Fried Rice");
    let option: Value =
        sqlx::query_scalar("SELECT translations FROM menu_item_price_options").fetch_one(&pool).await.unwrap();
    assert_eq!(option["fr"]["label"], "[fr] Large");
    let rental: Value = sqlx::query_scalar("SELECT translations FROM rentals").fetch_one(&pool).await.unwrap();
    assert_eq!(rental["nl"]["name"], "[nl] Cooler");
    let category: (Value, Value) =
        sqlx::query_as("SELECT translations, draft FROM categories").fetch_one(&pool).await.unwrap();
    assert_eq!(category.0["fr"]["description"], "[fr] Rice and more");
    assert!(category.1.get("translations").is_none(), "{}", category.1);

    let calls = fake.calls.load(Ordering::SeqCst);
    assert_eq!(sweep_once(&pool, &config).await.unwrap(), 0);
    assert_eq!(fake.calls.load(Ordering::SeqCst), calls, "nothing left to translate");

    // A new keep list redoes the automatic translations.
    sqlx::query("UPDATE translation_settings SET keep_version = keep_version + 1").execute(&pool).await.unwrap();
    assert!(sweep_once(&pool, &config).await.unwrap() > 0);
}

#[sqlx::test]
async fn the_sweep_does_nothing_when_switched_off(pool: PgPool) {
    sqlx::query("INSERT INTO rentals (id, name, price) VALUES (1, 'Cooler', 10)").execute(&pool).await.unwrap();
    sqlx::query("UPDATE translation_settings SET auto_translate = false").execute(&pool).await.unwrap();
    let (fake, config) = fake(0);
    assert_eq!(sweep_once(&pool, &config).await.unwrap(), 0);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[sqlx::test]
async fn a_changed_keep_list_bumps_its_version(pool: PgPool) {
    let cookie = login(&pool).await;
    let (_fake, config) = fake(0);
    let app = build_test_app_with_translation(pool.clone(), config).await;
    let (_, settings) = send_json_on(app.clone(), "GET", "/api/admin/translations/status", None, Some(&cookie)).await;

    let save = |words: Value, updated_at: Value| {
        send_json_on(
            app.clone(),
            "PUT",
            "/api/admin/translations/settings",
            Some(json!({ "auto_translate": true, "stop_at_percent": 80, "keep_words": words, "updated_at": updated_at })),
            Some(&cookie),
        )
    };
    let (status, saved) = save(settings["keep_words"].clone(), settings["updated_at"].clone()).await;
    assert_eq!(status, StatusCode::OK);
    let version: i32 = sqlx::query_scalar("SELECT keep_version FROM translation_settings").fetch_one(&pool).await.unwrap();
    assert_eq!(version, 1, "same list, same version");

    let (status, saved) = save(json!(["Jollof", " Suya ", "jollof"]), saved["updated_at"].clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["keep_words"], json!(["Jollof", "Suya"]));
    let version: i32 = sqlx::query_scalar("SELECT keep_version FROM translation_settings").fetch_one(&pool).await.unwrap();
    assert_eq!(version, 2);

    // A stale token is a conflict.
    let (status, _) = save(json!([]), settings["updated_at"].clone()).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn orders_keep_the_customers_language_and_admin_keeps_english(pool: PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes');
        INSERT INTO menu_items (id, category_id, name, price, translations)
            VALUES (1, 1, 'Fried Rice', 12.00, '{"nl": {"name": "Gebakken rijst"}}');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let order = |locale: &str| {
        json!({
            "customer_name": "Ada Lovelace",
            "customer_email": "ada@example.com",
            "customer_phone": "+31 6 1234 5678",
            "delivery_type": "pickup",
            "notes": null,
            "locale": locale,
            "items": [{ "menu_item_id": 1, "quantity": 1 }],
        })
    };

    let (status, placed) = post(pool.clone(), "/api/orders", order("nl")).await;
    assert_eq!(status, StatusCode::CREATED, "{placed}");
    assert_eq!(placed["items"][0]["item_name"], "Gebakken rijst");
    let token = placed["public_token"].as_str().unwrap();
    let (_, tracked) = get(pool.clone(), &format!("/api/orders/by-token/{token}")).await;
    assert_eq!(tracked["items"][0]["item_name"], "Gebakken rijst");
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT item_name, item_name_local FROM order_items").fetch_one(&pool).await.unwrap();
    assert_eq!(row, ("Fried Rice".to_string(), Some("Gebakken rijst".to_string())));

    // French has no translation yet: English, and nothing extra stored.
    post(pool.clone(), "/api/orders", order("fr")).await;
    let local: Option<String> = sqlx::query_scalar("SELECT item_name_local FROM order_items ORDER BY id DESC LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(local, None);
}

#[sqlx::test]
async fn the_sweep_leaves_unpublished_rows_to_publish(pool: PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO categories (id, name) VALUES (1, 'Rice Dishes');
        INSERT INTO menu_items (id, category_id, name, image_url, published_at)
            VALUES (1, 1, 'Fried Ri', '/x.jpg', NULL);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let (_fake, config) = fake(0);
    sweep_once(&pool, &config).await.unwrap();
    let item: Value = sqlx::query_scalar("SELECT translations FROM menu_items").fetch_one(&pool).await.unwrap();
    assert_eq!(item, json!({}), "the open form would overwrite it");

    sqlx::query("UPDATE menu_items SET published_at = now()").execute(&pool).await.unwrap();
    sweep_once(&pool, &config).await.unwrap();
    let item: Value = sqlx::query_scalar("SELECT translations FROM menu_items").fetch_one(&pool).await.unwrap();
    assert_eq!(item["nl"]["name"], "[nl] Fried Ri");
}
