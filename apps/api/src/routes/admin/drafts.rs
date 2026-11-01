//! Phase 25 draft → preview → publish (requirement.md Decisions 15–21):
//! the preview reads, the Changes list, Publish and Discard, for site
//! content, menu items and categories.

use crate::error::{AppError, FieldError};
use crate::routes::admin::categories::{
    apply_category_fields, parse_category_draft, validate_category, CategoryFields,
};
use crate::routes::admin::menu_items::{
    apply_menu_item_fields, delete_media_object, effective_fields, parse_draft, validate_menu_item,
    MenuItemFields, PriceOptionInput,
};
use crate::routes::categories::CategoryResponse;
use crate::routes::menu_items::{ListMenuItemsQuery, MenuItemResponse, PriceOptionResponse};
use crate::site_content::SiteContent;
use crate::MediaConfig;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::{BTreeSet, HashMap};

const KIND_SITE_CONTENT: &str = "site_content";
const KIND_MENU_ITEM: &str = "menu_item";
const KIND_CATEGORY: &str = "category";

struct MenuItemSourceRow {
    id: i64,
    category_id: i64,
    name: String,
    description: Option<String>,
    price: Option<rust_decimal::Decimal>,
    image_url: Option<String>,
    is_available: bool,
    is_featured: bool,
    draft: Option<Value>,
    draft_updated_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
}

struct LiveOptionRow {
    id: i64,
    menu_item_id: i64,
    label: String,
    price: rust_decimal::Decimal,
}

/// Every non-archived item with its live price options.
async fn load_menu_item_sources(
    pool: &PgPool,
) -> Result<(Vec<MenuItemSourceRow>, HashMap<i64, Vec<LiveOptionRow>>), sqlx::Error> {
    let items = sqlx::query_as!(
        MenuItemSourceRow,
        r#"
        SELECT id, category_id, name, description, price, image_url, is_available, is_featured,
               draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM menu_items
        WHERE deleted_at IS NULL
        ORDER BY display_order, id
        "#
    )
    .fetch_all(pool)
    .await?;

    let option_rows = sqlx::query_as!(
        LiveOptionRow,
        r#"
        SELECT id, menu_item_id, label, price
        FROM menu_item_price_options
        WHERE deleted_at IS NULL
        ORDER BY menu_item_id, display_order, id
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut options: HashMap<i64, Vec<LiveOptionRow>> = HashMap::new();
    for row in option_rows {
        options.entry(row.menu_item_id).or_default().push(row);
    }
    Ok((items, options))
}

fn live_fields(row: &MenuItemSourceRow, options: &[LiveOptionRow]) -> MenuItemFields {
    MenuItemFields {
        category_id: row.category_id,
        name: row.name.clone(),
        description: row.description.clone(),
        price: row.price,
        price_options: options
            .iter()
            .map(|option| PriceOptionInput { label: option.label.clone(), price: option.price })
            .collect(),
        image_url: row.image_url.clone(),
        is_featured: row.is_featured,
    }
}

/// `GET /api/admin/preview/menu-items`: the public response shape, with
/// drafts overlaid and unpublished items included. A draft's price
/// options have no ids yet, so they get negative placeholder ids (ordering
/// is disabled in the preview anyway).
pub async fn preview_menu_items(
    State(pool): State<PgPool>,
    Query(params): Query<ListMenuItemsQuery>,
) -> Result<Json<Vec<MenuItemResponse>>, AppError> {
    let (items, mut options) = load_menu_item_sources(&pool).await?;

    let response = items
        .into_iter()
        .filter_map(|row| {
            let live_options = options.remove(&row.id).unwrap_or_default();
            let draft = row.draft.clone().and_then(|draft| parse_draft(row.id, draft));
            let price_options = match &draft {
                Some(draft) => draft
                    .price_options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| PriceOptionResponse {
                        id: -(index as i64) - 1,
                        label: option.label.clone(),
                        price: option.price,
                    })
                    .collect(),
                None => live_options
                    .iter()
                    .map(|option| PriceOptionResponse {
                        id: option.id,
                        label: option.label.clone(),
                        price: option.price,
                    })
                    .collect(),
            };
            let fields = effective_fields(live_fields(&row, &live_options), draft);
            if params.category_id.is_some_and(|id| id != fields.category_id) {
                return None;
            }
            Some(MenuItemResponse {
                id: row.id,
                category_id: fields.category_id,
                name: fields.name,
                description: fields.description,
                price: fields.price,
                image_url: fields.image_url,
                is_available: row.is_available,
                is_featured: fields.is_featured,
                price_options,
            })
        })
        .collect();

    Ok(Json(response))
}

struct CategorySourceRow {
    id: i64,
    name: String,
    description: Option<String>,
    draft: Option<Value>,
    draft_updated_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
}

async fn load_category_sources(pool: &PgPool) -> Result<Vec<CategorySourceRow>, sqlx::Error> {
    sqlx::query_as!(
        CategorySourceRow,
        r#"
        SELECT id, name, description,
               draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM categories
        WHERE deleted_at IS NULL
        ORDER BY display_order, id
        "#
    )
    .fetch_all(pool)
    .await
}

fn effective_category(row: &CategorySourceRow) -> CategoryFields {
    let live = CategoryFields { name: row.name.clone(), description: row.description.clone() };
    row.draft.clone().and_then(|draft| parse_category_draft(row.id, draft)).unwrap_or(live)
}

/// `GET /api/admin/preview/categories`.
pub async fn preview_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let rows = load_category_sources(&pool).await?;
    Ok(Json(
        rows.iter()
            .map(|row| {
                let fields = effective_category(row);
                CategoryResponse { id: row.id, name: fields.name, description: fields.description }
            })
            .collect(),
    ))
}

struct SiteContentRow {
    published: Value,
    draft: Option<Value>,
    draft_updated_at: Option<DateTime<Utc>>,
}

async fn load_site_content_row(
    executor: impl sqlx::PgExecutor<'_>,
    lock: bool,
) -> Result<(SiteContent, Option<SiteContent>, Option<DateTime<Utc>>), AppError> {
    let row = if lock {
        sqlx::query_as!(
            SiteContentRow,
            r#"SELECT published, draft, draft_updated_at as "draft_updated_at: DateTime<Utc>" FROM site_content FOR UPDATE"#
        )
        .fetch_one(executor)
        .await?
    } else {
        sqlx::query_as!(
            SiteContentRow,
            r#"SELECT published, draft, draft_updated_at as "draft_updated_at: DateTime<Utc>" FROM site_content"#
        )
        .fetch_one(executor)
        .await?
    };
    let parse = |value: Value| -> Result<SiteContent, AppError> {
        serde_json::from_value(value).map_err(|err| {
            tracing::error!(%err, "unreadable site_content document");
            AppError::Internal
        })
    };
    let published = parse(row.published)?;
    let draft = row.draft.map(parse).transpose()?;
    Ok((published, draft, row.draft_updated_at))
}

pub(crate) async fn load_site_content(
    pool: &PgPool,
) -> Result<(SiteContent, Option<SiteContent>, Option<DateTime<Utc>>), AppError> {
    load_site_content_row(pool, false).await
}

pub(crate) async fn load_site_content_for_update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(SiteContent, Option<SiteContent>, Option<DateTime<Utc>>), AppError> {
    load_site_content_row(&mut **tx, true).await
}

/// `GET /api/admin/preview/site-content`: the draft if there is one.
pub async fn preview_site_content(State(pool): State<PgPool>) -> Result<Json<SiteContent>, AppError> {
    let (published, draft, _) = load_site_content(&pool).await?;
    Ok(Json(draft.unwrap_or(published)))
}

#[derive(Debug, Serialize)]
pub struct ChangedField {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Serialize)]
pub struct ChangeEntry {
    pub kind: &'static str,
    pub id: Option<i64>,
    pub label: String,
    pub changed_fields: Vec<ChangedField>,
    pub draft_updated_at: Option<DateTime<Utc>>,
    pub is_new: bool,
    /// For a menu item, the category its preview should open on.
    pub category_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ChangesQuery {
    /// `?count=1` returns just `{count}`, for the admin nav badge.
    pub count: Option<u8>,
}

fn price_options_text(options: &[PriceOptionInput]) -> Value {
    if options.is_empty() {
        return Value::Null;
    }
    Value::String(
        options.iter().map(|o| format!("{} €{}", o.label, o.price)).collect::<Vec<_>>().join(", "),
    )
}

fn menu_item_values(fields: &MenuItemFields, category_names: &HashMap<i64, String>) -> Vec<(&'static str, Value)> {
    vec![
        ("name", json!(fields.name)),
        ("description", json!(fields.description)),
        ("category", json!(category_names.get(&fields.category_id))),
        ("price", json!(fields.price.map(|price| price.to_string()))),
        ("price_options", price_options_text(&fields.price_options)),
        ("image_url", json!(fields.image_url)),
        ("is_featured", json!(fields.is_featured)),
    ]
}

/// Field-level before/after; for a new row every non-empty field, with
/// `before: null`.
fn diff(before: Option<Vec<(&'static str, Value)>>, after: Vec<(&'static str, Value)>) -> Vec<ChangedField> {
    match before {
        Some(before) => before
            .into_iter()
            .zip(after)
            .filter(|((_, b), (_, a))| b != a)
            .map(|((field, before), (_, after))| ChangedField { field: field.into(), before, after })
            .collect(),
        None => after
            .into_iter()
            .filter(|(_, value)| !value.is_null() && *value != json!(false))
            .map(|(field, after)| ChangedField { field: field.into(), before: Value::Null, after })
            .collect(),
    }
}

fn site_content_diff(published: &SiteContent, draft: &SiteContent) -> Vec<ChangedField> {
    let mut changes = Vec::new();
    for ((slot, before), (_, after)) in published.images.slots().into_iter().zip(draft.images.slots()) {
        if before != after {
            changes.push(ChangedField { field: format!("images.{slot}"), before: json!(before), after: json!(after) });
        }
    }

    let locales: BTreeSet<&String> = published.home_text.keys().chain(draft.home_text.keys()).collect();
    for locale in locales {
        let empty = Default::default();
        let before_texts = published.home_text.get(locale).unwrap_or(&empty);
        let after_texts = draft.home_text.get(locale).unwrap_or(&empty);
        let keys: BTreeSet<&String> = before_texts.keys().chain(after_texts.keys()).collect();
        for key in keys {
            let (before, after) = (before_texts.get(key), after_texts.get(key));
            if before != after {
                changes.push(ChangedField {
                    field: format!("home_text.{locale}.{key}"),
                    before: json!(before),
                    after: json!(after),
                });
            }
        }
    }
    changes
}

async fn collect_changes(pool: &PgPool) -> Result<Vec<ChangeEntry>, AppError> {
    let mut entries = Vec::new();

    let (published, draft, draft_updated_at) = load_site_content(pool).await?;
    if let Some(draft) = draft {
        entries.push(ChangeEntry {
            kind: KIND_SITE_CONTENT,
            id: None,
            label: "Site content".into(),
            changed_fields: site_content_diff(&published, &draft),
            draft_updated_at,
            is_new: false,
            category_id: None,
        });
    }

    let categories = load_category_sources(pool).await?;
    let category_names: HashMap<i64, String> =
        categories.iter().map(|row| (row.id, effective_category(row).name)).collect();

    for row in &categories {
        let is_new = row.published_at.is_none();
        let draft = row.draft.clone().and_then(|draft| parse_category_draft(row.id, draft));
        if !is_new && draft.is_none() {
            continue;
        }
        let live = CategoryFields { name: row.name.clone(), description: row.description.clone() };
        let values = |fields: &CategoryFields| -> Vec<(&'static str, Value)> {
            vec![("name", json!(fields.name)), ("description", json!(fields.description))]
        };
        let changed_fields = match &draft {
            Some(draft) => diff(Some(values(&live)), values(draft)),
            None => diff(None, values(&live)),
        };
        entries.push(ChangeEntry {
            kind: KIND_CATEGORY,
            id: Some(row.id),
            label: effective_category(row).name,
            changed_fields,
            draft_updated_at: row.draft_updated_at,
            is_new,
            category_id: Some(row.id),
        });
    }

    let (items, options) = load_menu_item_sources(pool).await?;
    for row in &items {
        let is_new = row.published_at.is_none();
        let draft = row.draft.clone().and_then(|draft| parse_draft(row.id, draft));
        if !is_new && draft.is_none() {
            continue;
        }
        let live = live_fields(row, options.get(&row.id).map(Vec::as_slice).unwrap_or_default());
        let changed_fields = match &draft {
            Some(draft) => diff(Some(menu_item_values(&live, &category_names)), menu_item_values(draft, &category_names)),
            None => diff(None, menu_item_values(&live, &category_names)),
        };
        let fields = effective_fields(live, draft);
        entries.push(ChangeEntry {
            kind: KIND_MENU_ITEM,
            id: Some(row.id),
            label: fields.name,
            changed_fields,
            draft_updated_at: row.draft_updated_at,
            is_new,
            category_id: Some(fields.category_id),
        });
    }

    Ok(entries)
}

/// `GET /api/admin/changes`: every pending draft and unpublished row.
pub async fn list_changes(
    State(pool): State<PgPool>,
    Query(params): Query<ChangesQuery>,
) -> Result<Json<Value>, AppError> {
    let entries = collect_changes(&pool).await?;
    if params.count == Some(1) {
        return Ok(Json(json!({ "count": entries.len() })));
    }
    Ok(Json(json!(entries)))
}

#[derive(Debug, Deserialize)]
pub struct PublishEntry {
    pub kind: String,
    pub id: Option<i64>,
    pub draft_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub entries: Vec<PublishEntry>,
}

#[derive(Debug, Serialize)]
pub struct PublishedEntry {
    pub kind: &'static str,
    pub id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PublishResponse {
    pub published: Vec<PublishedEntry>,
}

fn stale(kind: &str, id: Option<i64>) -> AppError {
    AppError::Conflict(json!({ "kind": kind, "id": id }))
}

fn invalid_entry(index: usize, field: &str) -> AppError {
    AppError::Validation(vec![FieldError { field: format!("entries[{index}].{field}"), message: "invalid".into() }])
}

/// `POST /api/admin/publish` (requirement.md Decision 19): every entry in
/// one transaction. Any stale `draft_updated_at` rolls the whole request
/// back with `409`; superseded media is deleted only after commit.
pub async fn publish(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(payload): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, AppError> {
    let mut tx = pool.begin().await?;
    let mut superseded: Vec<String> = Vec::new();
    let mut published = Vec::new();

    for (index, entry) in payload.entries.iter().enumerate() {
        match (entry.kind.as_str(), entry.id) {
            (KIND_SITE_CONTENT, _) => {
                let (live, draft, draft_updated_at) = load_site_content_for_update(&mut tx).await?;
                if draft_updated_at != entry.draft_updated_at {
                    return Err(stale(KIND_SITE_CONTENT, None));
                }
                let Some(draft) = draft else { continue };
                let errors = draft.validate(&[&live, &draft], &media.public_base_url);
                if !errors.is_empty() {
                    return Err(AppError::Validation(errors));
                }
                superseded.extend(
                    live.images.urls().filter(|url| !draft.images.urls().any(|kept| kept == *url)).map(str::to_string),
                );
                sqlx::query!(
                    r#"
                    UPDATE site_content
                    SET published = $1, draft = NULL, draft_updated_at = NULL,
                        published_at = now(), updated_at = now()
                    "#,
                    serde_json::to_value(&draft).expect("serializable"),
                )
                .execute(&mut *tx)
                .await?;
                published.push(PublishedEntry { kind: KIND_SITE_CONTENT, id: None });
            }
            (KIND_MENU_ITEM, Some(id)) => {
                let row = sqlx::query!(
                    r#"
                    SELECT category_id, name, description, price, image_url, is_featured, draft,
                           draft_updated_at as "draft_updated_at: DateTime<Utc>",
                           deleted_at as "deleted_at: DateTime<Utc>"
                    FROM menu_items WHERE id = $1
                    FOR UPDATE
                    "#,
                    id
                )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| stale(KIND_MENU_ITEM, Some(id)))?;
                if row.deleted_at.is_some() || row.draft_updated_at != entry.draft_updated_at {
                    return Err(stale(KIND_MENU_ITEM, Some(id)));
                }

                let live_options = sqlx::query_as!(
                    PriceOptionInput,
                    r#"
                    SELECT label, price FROM menu_item_price_options
                    WHERE menu_item_id = $1 AND deleted_at IS NULL
                    ORDER BY display_order, id
                    "#,
                    id
                )
                .fetch_all(&mut *tx)
                .await?;
                let live = MenuItemFields {
                    category_id: row.category_id,
                    name: row.name,
                    description: row.description,
                    price: row.price,
                    price_options: live_options,
                    image_url: row.image_url.clone(),
                    is_featured: row.is_featured,
                };
                let draft = row.draft.and_then(|draft| parse_draft(id, draft));
                let has_draft = draft.is_some();
                let fields = effective_fields(live, draft);

                // Re-validated: the category may have been archived since
                // the draft was saved (requirement.md Decision 18).
                validate_menu_item(&pool, &media, &fields, row.image_url.as_deref()).await?;
                if has_draft {
                    apply_menu_item_fields(&mut tx, id, &fields).await?;
                    if let Some(previous) = row.image_url
                        && fields.image_url.as_deref() != Some(previous.as_str())
                    {
                        superseded.push(previous);
                    }
                }
                sqlx::query!(
                    r#"
                    UPDATE menu_items
                    SET published_at = COALESCE(published_at, now()), draft = NULL,
                        draft_updated_at = NULL, updated_at = now()
                    WHERE id = $1
                    "#,
                    id
                )
                .execute(&mut *tx)
                .await?;
                published.push(PublishedEntry { kind: KIND_MENU_ITEM, id: Some(id) });
            }
            (KIND_CATEGORY, Some(id)) => {
                let row = sqlx::query!(
                    r#"
                    SELECT name, description, draft,
                           draft_updated_at as "draft_updated_at: DateTime<Utc>",
                           deleted_at as "deleted_at: DateTime<Utc>"
                    FROM categories WHERE id = $1
                    FOR UPDATE
                    "#,
                    id
                )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| stale(KIND_CATEGORY, Some(id)))?;
                if row.deleted_at.is_some() || row.draft_updated_at != entry.draft_updated_at {
                    return Err(stale(KIND_CATEGORY, Some(id)));
                }
                if let Some(draft) = row.draft.and_then(|draft| parse_category_draft(id, draft)) {
                    validate_category(&draft)?;
                    apply_category_fields(&mut tx, id, &draft).await?;
                }
                sqlx::query!(
                    r#"
                    UPDATE categories
                    SET published_at = COALESCE(published_at, now()), draft = NULL,
                        draft_updated_at = NULL, updated_at = now()
                    WHERE id = $1
                    "#,
                    id
                )
                .execute(&mut *tx)
                .await?;
                published.push(PublishedEntry { kind: KIND_CATEGORY, id: Some(id) });
            }
            (KIND_MENU_ITEM | KIND_CATEGORY, None) => return Err(invalid_entry(index, "id")),
            _ => return Err(invalid_entry(index, "kind")),
        }
    }

    tx.commit().await?;

    for url in &superseded {
        delete_media_object(&media, url).await;
    }

    Ok(Json(PublishResponse { published }))
}

/// `DELETE /api/admin/drafts/site_content`.
pub async fn discard_site_content_draft(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
) -> Result<StatusCode, AppError> {
    let mut tx = pool.begin().await?;
    let (live, draft, _) = load_site_content_for_update(&mut tx).await?;
    sqlx::query!("UPDATE site_content SET draft = NULL, draft_updated_at = NULL")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    if let Some(draft) = draft {
        for url in draft.images.urls().filter(|url| !live.images.urls().any(|kept| kept == *url)) {
            delete_media_object(&media, url).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/admin/drafts/{kind}/{id}` (requirement.md Decision 19):
/// clears a published row's draft, deleting the draft's own photo, or
/// archives an unpublished row.
pub async fn discard_draft(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path((kind, id)): Path<(String, i64)>,
) -> Result<StatusCode, AppError> {
    match kind.as_str() {
        KIND_MENU_ITEM => {
            let row = sqlx::query!(
                r#"
                UPDATE menu_items mi
                SET draft = NULL, draft_updated_at = NULL,
                    deleted_at = CASE WHEN mi.published_at IS NULL THEN now() ELSE mi.deleted_at END,
                    updated_at = CASE WHEN mi.published_at IS NULL THEN now() ELSE mi.updated_at END
                FROM (SELECT id, draft FROM menu_items WHERE id = $1 AND deleted_at IS NULL FOR UPDATE) before
                WHERE mi.id = before.id
                RETURNING mi.image_url, before.draft
                "#,
                id
            )
            .fetch_optional(&pool)
            .await?
            .ok_or(AppError::NotFound)?;

            if let Some(draft_image) = row.draft.and_then(|draft| parse_draft(id, draft)).and_then(|d| d.image_url)
                && row.image_url.as_deref() != Some(draft_image.as_str())
            {
                delete_media_object(&media, &draft_image).await;
            }
        }
        KIND_CATEGORY => {
            let mut tx = pool.begin().await?;
            let row = sqlx::query!(
                r#"
                SELECT published_at as "published_at: DateTime<Utc>"
                FROM categories WHERE id = $1 AND deleted_at IS NULL
                FOR UPDATE
                "#,
                id
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;

            if row.published_at.is_none() {
                let blocking = sqlx::query_as!(
                    crate::error::BlockingMenuItem,
                    r#"SELECT id, name FROM menu_items WHERE category_id = $1 AND deleted_at IS NULL"#,
                    id
                )
                .fetch_all(&mut *tx)
                .await?;
                if !blocking.is_empty() {
                    return Err(AppError::CategoryHasActiveItems(blocking));
                }
            }

            sqlx::query!(
                r#"
                UPDATE categories
                SET draft = NULL, draft_updated_at = NULL,
                    deleted_at = CASE WHEN published_at IS NULL THEN now() ELSE deleted_at END,
                    updated_at = CASE WHEN published_at IS NULL THEN now() ELSE updated_at END
                WHERE id = $1
                "#,
                id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        _ => return Err(AppError::NotFound),
    }
    Ok(StatusCode::NO_CONTENT)
}
