use crate::MediaConfig;
use crate::chrono_to_offset;
use crate::error::{AppError, FieldError};
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceOptionInput {
    pub label: String,
    #[serde(serialize_with = "rust_decimal::serde::str::serialize")]
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
    /// Phase 25 Decision 15: new items start unpublished.
    pub is_published: bool,
    pub has_draft: bool,
    /// The pending edit of a published item; the form loads this instead
    /// of the live columns when present.
    pub draft: Option<MenuItemFields>,
    /// The optimistic-concurrency token for draft saves and publish.
    pub draft_updated_at: Option<DateTime<Utc>>,
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
    draft: Option<serde_json::Value>,
    draft_updated_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
}

/// A menu item's editable content: the create body, the draft-save body
/// and the shape stored in `menu_items.draft` (Phase 25 Decision 15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItemFields {
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(serialize_with = "rust_decimal::serde::str_option::serialize")]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub price_options: Vec<PriceOptionInput>,
    pub image_url: Option<String>,
    pub is_featured: bool,
}

#[derive(Debug, Deserialize)]
pub struct SaveMenuItemDraftRequest {
    #[serde(flatten)]
    pub fields: MenuItemFields,
    /// `null` for the first draft of a published item, or for an
    /// unpublished item that has never been saved since it was created.
    pub draft_updated_at: Option<DateTime<Utc>>,
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
pub(crate) fn is_valid_media_url(url: &str, public_base_url: &str) -> bool {
    url.strip_prefix(public_base_url)
        .and_then(|rest| rest.strip_prefix("/api/media/"))
        .is_some_and(|key| !key.is_empty() && !key.contains('/'))
}

pub(crate) async fn validate_menu_item(
    pool: &PgPool,
    media: &MediaConfig,
    payload: &MenuItemFields,
    existing_image_url: Option<&str>,
) -> Result<(), AppError> {
    let mut errors = Vec::new();

    if payload.name.trim().is_empty() {
        errors.push(FieldError {
            field: "name".into(),
            message: "required".into(),
        });
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
    match &payload.image_url {
        None => errors.push(FieldError {
            field: "image_url".into(),
            message: "required".into(),
        }),
        Some(url)
            if existing_image_url != Some(url.as_str())
                && !is_valid_media_url(url, &media.public_base_url) =>
        {
            errors.push(FieldError {
                field: "image_url".into(),
                message: "invalid".into(),
            });
        }
        Some(_) => {}
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
        errors.push(FieldError {
            field: "category_id".into(),
            message: "invalid".into(),
        });
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
            .push(AdminPriceOptionResponse {
                id: row.id,
                label: row.label,
                price: row.price,
            });
    }

    items
        .into_iter()
        .map(|item| AdminMenuItemResponse {
            price_options: options_by_item.remove(&item.id).unwrap_or_default(),
            is_published: item.published_at.is_some(),
            has_draft: item.draft.is_some(),
            draft: item.draft.and_then(|draft| parse_draft(item.id, draft)),
            draft_updated_at: item.draft_updated_at,
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
               mi.updated_at as "updated_at: DateTime<Utc>",
               mi.draft, mi.draft_updated_at as "draft_updated_at: DateTime<Utc>",
               mi.published_at as "published_at: DateTime<Utc>"
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
               mi.updated_at as "updated_at: DateTime<Utc>",
               mi.draft, mi.draft_updated_at as "draft_updated_at: DateTime<Utc>",
               mi.published_at as "published_at: DateTime<Utc>"
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
        rows_to_responses(vec![item], option_rows)
            .into_iter()
            .next()
            .expect("one row in, one row out"),
    ))
}

/// Disambiguates a 0-row conditional UPDATE the same way Group 7's
/// `category_or_conflict` does: row gone -> 404, row moved on -> 409 with
/// the live item (reuses `get_menu_item` so the "current" shape in a 409 is
/// identical to what a GET would return).
async fn menu_item_or_conflict(pool: PgPool, id: i64) -> AppError {
    match get_menu_item(State(pool), Path(id)).await {
        Ok(Json(current)) => {
            AppError::Conflict(serde_json::to_value(current).expect("serializable"))
        }
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

/// New items start unpublished (Phase 25 Decision 15): the column defaults
/// to now() for hand-inserted rows, so NULL is set explicitly here.
pub async fn create_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(payload): Json<MenuItemFields>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    validate_menu_item(&pool, &media, &payload, None).await?;

    let mut tx = pool.begin().await?;

    let item_id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO menu_items (category_id, name, description, price, image_url, is_featured, display_order, published_at)
        VALUES ($1, $2, $3, $4, $5, $6,
            (SELECT COALESCE(MAX(display_order), -1) + 1 FROM menu_items WHERE category_id = $1), NULL)
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

/// Phase 25 Decision 15: a published item's edit is stored in `draft`
/// and only reaches the live columns on publish; an unpublished item has no
/// live audience, so its edit writes the live columns directly. Either way
/// `draft_updated_at` is the concurrency token, and no media is deleted
/// here — the preview may still be showing the previous image.
pub async fn save_menu_item_draft(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<SaveMenuItemDraftRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"
        SELECT image_url, deleted_at as "deleted_at: DateTime<Utc>",
               draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM menu_items WHERE id = $1
        FOR UPDATE
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.deleted_at.is_some() {
        return Err(AppError::NotFound);
    }
    if row.draft_updated_at != payload.draft_updated_at {
        tx.rollback().await?;
        return Err(menu_item_or_conflict(pool, id).await);
    }

    // The grandfathering check is against the live image (requirement.md
    // Decision 18).
    validate_menu_item(&pool, &media, &payload.fields, row.image_url.as_deref()).await?;

    if row.published_at.is_none() {
        apply_menu_item_fields(&mut tx, id, &payload.fields).await?;
        sqlx::query!("UPDATE menu_items SET draft_updated_at = now() WHERE id = $1", id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query!(
            "UPDATE menu_items SET draft = $2, draft_updated_at = now() WHERE id = $1",
            id,
            serde_json::to_value(&payload.fields).expect("serializable"),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    get_menu_item(State(pool), Path(id)).await
}

/// Writes `fields` to the live columns and fully replaces the price
/// options (requirement.md Decision 6). Shared by an unpublished item's
/// draft save and by publish. A check_violation here (Group 1a's trigger,
/// archived category) becomes `AppError::CategoryArchived` via `?`.
pub(crate) async fn apply_menu_item_fields(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: i64,
    fields: &MenuItemFields,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE menu_items
        SET category_id = $2, name = $3, description = $4, price = $5,
            image_url = $6, is_featured = $7, updated_at = now()
        WHERE id = $1
        "#,
        id,
        fields.category_id,
        fields.name.trim(),
        fields.description,
        fields.price,
        fields.image_url,
        fields.is_featured,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE menu_item_price_options SET deleted_at = now() WHERE menu_item_id = $1 AND deleted_at IS NULL"#,
        id
    )
    .execute(&mut **tx)
    .await?;
    insert_price_options(tx, id, &fields.price_options).await?;
    Ok(())
}

/// A stored draft always deserializes (it was validated on save); a row
/// that somehow doesn't is logged and treated as having no draft.
pub(crate) fn parse_draft(id: i64, draft: serde_json::Value) -> Option<MenuItemFields> {
    match serde_json::from_value(draft) {
        Ok(fields) => Some(fields),
        Err(err) => {
            tracing::error!(%err, id, "unreadable menu item draft");
            None
        }
    }
}

/// The content preview shows and publish applies: the draft when there is
/// one, otherwise the live columns. One function, so preview and publish
/// can't disagree (plan 5.5).
pub(crate) fn effective_fields(live: MenuItemFields, draft: Option<MenuItemFields>) -> MenuItemFields {
    draft.unwrap_or(live)
}

/// Deletes a MinIO object this API issued. A failure only leaves an orphan
/// behind, so it is logged and never fails the request (requirement.md
/// Decision 4 addendum).
pub(crate) async fn delete_media_object(media: &MediaConfig, url: &str) {
    let Some(key) = media_key_from_url(url, &media.public_base_url) else { return };
    if let Err(err) = media.client.delete_object().bucket(&media.bucket).key(&key).send().await {
        tracing::warn!(%err, %key, "failed to delete superseded media object");
    }
}

/// `image_url`s this API issues always look like
/// `{public_base_url}/api/media/{key}` (admin/media.rs) — strip that prefix
/// to recover the object key for a `DeleteObject` call.
pub(crate) fn media_key_from_url(url: &str, public_base_url: &str) -> Option<String> {
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

/// Archiving also discards a pending draft (Phase 25 Decision 16),
/// including the draft's own uploaded photo.
pub async fn archive_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<ArchiveMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let archived = sqlx::query!(
        r#"
        UPDATE menu_items mi
        SET deleted_at = now(), updated_at = now(), draft = NULL, draft_updated_at = NULL
        FROM (SELECT id, draft FROM menu_items WHERE id = $1 FOR UPDATE) before
        WHERE mi.id = before.id AND mi.updated_at = $2
        RETURNING mi.image_url, before.draft
        "#,
        id,
        chrono_to_offset(payload.updated_at),
    )
    .fetch_optional(&pool)
    .await?;

    let Some(archived) = archived else {
        return Err(menu_item_or_conflict(pool, id).await);
    };

    if let Some(draft_image) = archived.draft.and_then(|draft| parse_draft(id, draft)).and_then(|d| d.image_url)
        && archived.image_url.as_deref() != Some(draft_image.as_str())
    {
        delete_media_object(&media, &draft_image).await;
    }

    get_menu_item(State(pool), Path(id)).await
}
