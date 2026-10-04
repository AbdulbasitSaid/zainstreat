use crate::chrono_to_offset;
use crate::error::{AppError, FieldError};
use crate::MediaConfig;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PriceOptionInput {
    pub label: String,
    pub price: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AdminPriceOptionResponse {
    pub id: i64,
    pub label: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
}

struct PriceOptionRow {
    id: i64,
    menu_item_id: i64,
    label: String,
    price: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AdminMenuItemResponse {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub is_featured: bool,
    pub is_archived: bool,
    pub price_options: Vec<AdminPriceOptionResponse>,
    pub updated_at: DateTime<Utc>,
}

struct MenuItemRow {
    id: i64,
    category_id: i64,
    category_name: String,
    name: String,
    description: Option<String>,
    price: Option<Decimal>,
    image_url: Option<String>,
    is_available: bool,
    is_featured: bool,
    deleted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertMenuItemRequest {
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub price: Option<Decimal>,
    #[serde(default)]
    pub price_options: Vec<PriceOptionInput>,
    pub image_url: Option<String>,
    pub is_featured: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AvailabilityRequest {
    pub is_available: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveMenuItemRequest {
    pub updated_at: DateTime<Utc>,
}

/// Decision 14's addendum (open risk 4) — `image_url`, if present, must be a
/// key this API itself issued via `POST /api/admin/media`, never an
/// arbitrary string. Checks the same prefix `media_key_from_url` below
/// strips unconditionally for deletion.
fn is_valid_media_url(url: &str, public_base_url: &str) -> bool {
    url.strip_prefix(public_base_url)
        .and_then(|rest| rest.strip_prefix("/api/media/"))
        .is_some_and(|key| !key.is_empty() && !key.contains('/'))
}

async fn validate_menu_item(
    pool: &PgPool,
    media: &MediaConfig,
    payload: &UpsertMenuItemRequest,
    existing_image_url: Option<&str>,
) -> Result<(), AppError> {
    let mut errors = Vec::new();

    if payload.name.trim().is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    // requirement.md Decision 5 — exactly one of price / price_options.
    match (&payload.price, payload.price_options.is_empty()) {
        (Some(price), true) if *price > Decimal::ZERO => {}
        (Some(_), true) => errors.push(FieldError {
            field: "price".into(),
            message: "must_be_positive".into(),
        }),
        (None, false) => {
            for option in &payload.price_options {
                if option.label.trim().is_empty() {
                    errors.push(FieldError {
                        field: "price_options".into(),
                        message: "label_required".into(),
                    });
                }
                if option.price <= Decimal::ZERO {
                    errors.push(FieldError {
                        field: "price_options".into(),
                        message: "must_be_positive".into(),
                    });
                }
            }
        }
        (Some(_), false) => errors.push(FieldError {
            field: "price_options".into(),
            message: "exclusive_with_price".into(),
        }),
        (None, true) => errors.push(FieldError {
            field: "price".into(),
            message: "required_unless_price_options".into(),
        }),
    }

    // requirement.md Decision 14 addendum (open risk 4) — close the
    // size/type-check bypass: an image_url that isn't one this API issued
    // never gets persisted. Grandfathers a row's own unchanged image_url
    // through an edit (e.g. Phase 7's seed data, which predates this
    // phase's upload pipeline and uses relative asset paths) — only a
    // *new* or *changed* value must come from this API's own upload
    // endpoint, so the bypass this closes (an attacker- or bug-supplied
    // external URL actually reaching storage) stays closed without
    // bricking edits to every pre-existing row.
    if let Some(url) = &payload.image_url
        && existing_image_url != Some(url.as_str())
        && !is_valid_media_url(url, &media.public_base_url)
    {
        errors.push(FieldError { field: "image_url".into(), message: "invalid".into() });
    }

    // requirement.md Decision 7 addendum (open risk 5) — this remains the
    // fast common-case check; Group 1a's trigger is the atomic backstop for
    // the race this alone can't close.
    let category_active: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM categories WHERE id = $1 AND deleted_at IS NULL)"#,
        payload.category_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);
    if !category_active {
        errors.push(FieldError { field: "category_id".into(), message: "invalid".into() });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(errors))
    }
}

fn rows_to_responses(
    items: Vec<MenuItemRow>,
    option_rows: Vec<PriceOptionRow>,
) -> Vec<AdminMenuItemResponse> {
    let mut options_by_item: HashMap<i64, Vec<AdminPriceOptionResponse>> = HashMap::new();
    for row in option_rows {
        options_by_item
            .entry(row.menu_item_id)
            .or_default()
            .push(AdminPriceOptionResponse { id: row.id, label: row.label, price: row.price });
    }

    items
        .into_iter()
        .map(|item| AdminMenuItemResponse {
            price_options: options_by_item.remove(&item.id).unwrap_or_default(),
            id: item.id,
            category_id: item.category_id,
            category_name: item.category_name,
            name: item.name,
            description: item.description,
            price: item.price,
            image_url: item.image_url,
            is_available: item.is_available,
            is_featured: item.is_featured,
            is_archived: item.deleted_at.is_some(),
            updated_at: item.updated_at,
        })
        .collect()
}

pub async fn list_menu_items(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AdminMenuItemResponse>>, AppError> {
    let items = sqlx::query_as!(
        MenuItemRow,
        r#"
        SELECT mi.id, mi.category_id, c.name as category_name, mi.name, mi.description,
               mi.price, mi.image_url, mi.is_available, mi.is_featured,
               mi.deleted_at as "deleted_at: DateTime<Utc>",
               mi.updated_at as "updated_at: DateTime<Utc>"
        FROM menu_items mi
        JOIN categories c ON c.id = mi.category_id
        ORDER BY mi.display_order, mi.id
        "#
    )
    .fetch_all(&pool)
    .await?;

    let option_rows = sqlx::query_as!(
        PriceOptionRow,
        r#"
        SELECT id, menu_item_id, label, price
        FROM menu_item_price_options
        WHERE deleted_at IS NULL
        ORDER BY menu_item_id, display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows_to_responses(items, option_rows)))
}

/// Shared by every write handler's success path and by `menu_item_or_conflict`
/// so a create/update/toggle/archive response always matches what a
/// subsequent GET would return. `deleted_at IS NULL` is intentionally absent
/// on `menu_items` itself (unlike `list_menu_items`'s admin listing, which
/// also needs archived rows) — the admin detail view must still load an
/// archived item.
pub async fn get_menu_item(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let item = sqlx::query_as!(
        MenuItemRow,
        r#"
        SELECT mi.id, mi.category_id, c.name as category_name, mi.name, mi.description,
               mi.price, mi.image_url, mi.is_available, mi.is_featured,
               mi.deleted_at as "deleted_at: DateTime<Utc>",
               mi.updated_at as "updated_at: DateTime<Utc>"
        FROM menu_items mi
        JOIN categories c ON c.id = mi.category_id
        WHERE mi.id = $1
        "#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let option_rows = sqlx::query_as!(
        PriceOptionRow,
        r#"
        SELECT id, menu_item_id, label, price
        FROM menu_item_price_options
        WHERE menu_item_id = $1 AND deleted_at IS NULL
        ORDER BY display_order, id
        "#,
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows_to_responses(vec![item], option_rows).into_iter().next().expect("one row in, one row out"),
    ))
}

/// Disambiguates a 0-row conditional UPDATE the same way Group 7's
/// `category_or_conflict` does: row gone -> 404, row moved on -> 409 with
/// the live item (reuses `get_menu_item` so the "current" shape in a 409 is
/// identical to what a GET would return).
async fn menu_item_or_conflict(pool: PgPool, id: i64) -> AppError {
    match get_menu_item(State(pool), Path(id)).await {
        Ok(Json(current)) => AppError::Conflict(serde_json::to_value(current).expect("serializable")),
        Err(err) => err,
    }
}

async fn insert_price_options(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item_id: i64,
    options: &[PriceOptionInput],
) -> Result<(), sqlx::Error> {
    for (index, option) in options.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO menu_item_price_options (menu_item_id, label, price, display_order)
            VALUES ($1, $2, $3, $4)
            "#,
            item_id,
            option.label.trim(),
            option.price,
            index as i32,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn create_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(payload): Json<UpsertMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    validate_menu_item(&pool, &media, &payload, None).await?;

    let mut tx = pool.begin().await?;

    let item_id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO menu_items (category_id, name, description, price, image_url, is_featured, display_order)
        VALUES ($1, $2, $3, $4, $5, $6,
            (SELECT COALESCE(MAX(display_order), -1) + 1 FROM menu_items WHERE category_id = $1))
        RETURNING id
        "#,
        payload.category_id,
        payload.name.trim(),
        payload.description,
        payload.price,
        payload.image_url,
        payload.is_featured,
    )
    .fetch_one(&mut *tx)
    .await?;

    insert_price_options(&mut tx, item_id, &payload.price_options).await?;
    tx.commit().await?;

    get_menu_item(State(pool), Path(item_id)).await
}

pub async fn update_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<UpsertMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    // Read before validation — doubles as both the OCC-era image-cleanup
    // diff below and the grandfathering check inside `validate_menu_item`
    // for a row's own unchanged (possibly pre-this-phase) image_url.
    let previous_image_url: Option<String> =
        sqlx::query_scalar!("SELECT image_url FROM menu_items WHERE id = $1", id)
            .fetch_optional(&pool)
            .await?
            .flatten();

    validate_menu_item(&pool, &media, &payload, previous_image_url.as_deref()).await?;

    let mut tx = pool.begin().await?;

    // requirement.md Decision 17 — conditional on the caller's `updated_at`;
    // 0 affected rows means either the id is unknown or someone else updated
    // it first (disambiguated below). A check_violation here (Group 1a's
    // trigger, archived category) is converted to `AppError::CategoryArchived`
    // by the `?` below via `error.rs`'s `From<sqlx::Error>`.
    let affected = sqlx::query!(
        r#"
        UPDATE menu_items
        SET category_id = $3, name = $4, description = $5, price = $6,
            image_url = $7, is_featured = $8, updated_at = now()
        WHERE id = $1 AND updated_at = $2
        "#,
        id,
        chrono_to_offset(payload.updated_at),
        payload.category_id,
        payload.name.trim(),
        payload.description,
        payload.price,
        payload.image_url,
        payload.is_featured,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if affected == 0 {
        tx.rollback().await?;
        return Err(menu_item_or_conflict(pool, id).await);
    }

    // requirement.md Decision 6 — full replace, not a diff.
    sqlx::query!(
        r#"UPDATE menu_item_price_options SET deleted_at = now() WHERE menu_item_id = $1 AND deleted_at IS NULL"#,
        id
    )
    .execute(&mut *tx)
    .await?;
    insert_price_options(&mut tx, id, &payload.price_options).await?;

    tx.commit().await?;

    // requirement.md Decision 4 addendum (open risk 2) — the row is already
    // committed to the new image_url, so a delete failure here must never
    // fail the request; it only leaves one orphaned object behind, same as
    // today. Only fires when the image actually changed (including removal),
    // never on an edit that leaves it untouched.
    if previous_image_url.is_some()
        && previous_image_url != payload.image_url
        && let Some(key) =
            media_key_from_url(previous_image_url.as_deref().unwrap(), &media.public_base_url)
        && let Err(err) = media.client.delete_object().bucket(&media.bucket).key(&key).send().await
    {
        tracing::warn!(%err, %key, "failed to delete superseded media object");
    }

    get_menu_item(State(pool), Path(id)).await
}

/// `image_url`s this API issues always look like
/// `{public_base_url}/api/media/{key}` (admin/media.rs) — strip that prefix
/// to recover the object key for a `DeleteObject` call.
fn media_key_from_url(url: &str, public_base_url: &str) -> Option<String> {
    url.strip_prefix(public_base_url)?.strip_prefix("/api/media/").map(str::to_string)
}

pub async fn update_availability(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<AvailabilityRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let affected = sqlx::query!(
        r#"UPDATE menu_items SET is_available = $3, updated_at = now() WHERE id = $1 AND updated_at = $2"#,
        id,
        chrono_to_offset(payload.updated_at),
        payload.is_available,
    )
    .execute(&pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(menu_item_or_conflict(pool, id).await);
    }

    get_menu_item(State(pool), Path(id)).await
}

pub async fn archive_menu_item(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ArchiveMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let affected = sqlx::query!(
        r#"UPDATE menu_items SET deleted_at = now(), updated_at = now() WHERE id = $1 AND updated_at = $2"#,
        id,
        chrono_to_offset(payload.updated_at),
    )
    .execute(&pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(menu_item_or_conflict(pool, id).await);
    }

    get_menu_item(State(pool), Path(id)).await
}
