# Phase 14 — Admin: Menu & Categories: Implementation Plan

Groups 1–14 touch only `apps/api`. Groups 15–16 touch
`docker-compose*.yml` / `apps/web/next.config.ts` (infra/config, not
app code). Groups 17–23 touch only `apps/web`.

**Revised 2026-10-04 (open risks 3, 4, 5 resolved with the user — see
`requirement.md` Decision 17 and the addenda to Decisions 7 and 14):**
every column/table this phase's `menu_items`/`categories` reads or
writes already existed, **except** the new open-risk-5 guard, which adds
one migration (Group 1a) — the single exception to this phase's original
"no migration needed" plan. Groups 7 and 8 below are updated in place for
optimistic concurrency control (Decision 17); Group 2 gains the two new
`AppError` variants OCC and the risk-5 trigger need; Group 21 adds the
frontend crop step.

## Group 0 — Branch

Branched `2026-10-04-phase-14-admin-menu-categories` off `master` (Phase
13 already merged).

## Group 1 — `apps/api/Cargo.toml`: new dependencies

```toml
axum = { version = "0.8", features = ["multipart"] }
aws-sdk-s3 = "1"
uuid = { version = "1", features = ["v4"] }
```

`axum`'s existing bare `"0.8"` line gains the `multipart` feature
(backs `extract::Multipart`, used by Group 5). No version bump needed
for any existing dependency.

## Group 1a — New migration: guard archived categories at the DB level

Depends on: nothing. Added 2026-10-04 for `requirement.md` Decision 7's
addendum (open risk 5). Run `sqlx migrate add -r
guard_menu_item_category_not_archived` from `apps/api` to get the real
timestamp-prefixed filenames (shown here as `<ts>` — don't hand-pick a
timestamp).

**`apps/api/migrations/<ts>_guard_menu_item_category_not_archived.up.sql`:**

```sql
CREATE FUNCTION guard_menu_item_category_not_archived() RETURNS trigger AS $$
DECLARE
    category_deleted_at timestamptz;
BEGIN
    SELECT deleted_at INTO category_deleted_at
    FROM categories
    WHERE id = NEW.category_id
    FOR SHARE;

    IF category_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION 'menu item category % is archived', NEW.category_id
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER menu_items_category_not_archived
    BEFORE INSERT OR UPDATE OF category_id ON menu_items
    FOR EACH ROW
    EXECUTE FUNCTION guard_menu_item_category_not_archived();
```

**`apps/api/migrations/<ts>_guard_menu_item_category_not_archived.down.sql`:**

```sql
DROP TRIGGER menu_items_category_not_archived ON menu_items;
DROP FUNCTION guard_menu_item_category_not_archived();
```

The `FOR SHARE` lock on the referenced `categories` row is what actually
closes the race: it blocks until any concurrent transaction holding a
conflicting lock on that row — namely `archive_category`'s own `UPDATE
categories SET deleted_at = now() WHERE id = $1` (Group 7), which takes
an exclusive row lock — commits or rolls back, then reads the
post-commit value. The trigger and the archive can therefore never both
act on a stale view of the category. `BEFORE ... UPDATE OF category_id`
(a column-level trigger) means it only fires on insert or when
`category_id` is actually part of the `UPDATE`'s `SET` list — harmless
that `update_menu_item`'s full-replace `PATCH` always includes
`category_id` even when unchanged (Group 8); it just re-validates every
time, which is correct, not wasteful.

## Group 2 — `apps/api/src/error.rs`: two new variants

Depends on: nothing.

```rust
pub struct BlockingMenuItem {
    pub id: i64,
    pub name: String,
}

pub enum AppError {
    Database(sqlx::Error),
    Validation(Vec<FieldError>),
    ItemsUnavailable(Vec<UnavailableItem>),
    Unauthorized,
    InvalidCredentials,
    Session(tower_sessions::session::Error),
    NotFound,
    CategoryHasActiveItems(Vec<BlockingMenuItem>),
    CategoryArchived,              // new — Group 1a's trigger, open risk 5
    Conflict(serde_json::Value),   // new — OCC mismatch, requirement.md Decision 17
}
```

In `IntoResponse`:

```rust
AppError::NotFound => {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" }))).into_response()
}
AppError::CategoryHasActiveItems(items) => {
    tracing::warn!(?items, "category archive rejected: active menu items remain");
    (
        StatusCode::CONFLICT,
        Json(json!({ "error": "category_has_active_items", "items": items })),
    )
        .into_response()
}
AppError::CategoryArchived => (
    StatusCode::BAD_REQUEST,
    Json(json!({
        "error": "validation_error",
        "fields": [{ "field": "category_id", "message": "invalid" }],
    })),
)
    .into_response(),
AppError::Conflict(current) => (
    StatusCode::CONFLICT,
    Json(json!({ "error": "conflict", "current": current })),
)
    .into_response(),
```

`NotFound` decouples the media `GET` handler (Group 6) from
`sqlx::Error::RowNotFound` — it has no row to not-find, it has an S3
`NoSuchKey`. `BlockingMenuItem` needs `#[derive(Debug, Serialize)]` next
to the existing `FieldError`/`UnavailableItem` structs.

**`CategoryArchived`'s source:** wherever `sqlx::Error` currently
converts into `AppError::Database` (this file's existing `From<sqlx::Error>
for AppError`, or the `?` conversion site — check which this codebase
already uses), add a branch that inspects the error *before* falling
through to the generic `Database` mapping:

```rust
if let sqlx::Error::Database(db_err) = &err {
    if db_err.code().as_deref() == Some("23514")
        && db_err.message().contains("menu item category")
    {
        return AppError::CategoryArchived;
    }
}
```

Matching on the exception message (set verbatim by Group 1a's `RAISE
EXCEPTION 'menu item category % is archived'`) rather than a generic
`23514` catch-all avoids swallowing some *other*, unrelated
`check_violation` under the wrong error shape. Without this branch, the
rare race-condition case would surface as a raw, unexplained `500`
instead of the same `400 validation_error` the common-case pre-check in
Group 8 already returns.

## Group 3 — `apps/api/src/lib.rs`: `AppState`/`FromRef`

Depends on: nothing. This is the one structural change everything else
in `apps/api` builds on.

```rust
use axum::extract::FromRef;

#[derive(Clone)]
pub struct MediaConfig {
    pub client: aws_sdk_s3::Client,
    pub bucket: String,
    pub public_base_url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub media: MediaConfig,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}

impl FromRef<AppState> for MediaConfig {
    fn from_ref(state: &AppState) -> MediaConfig {
        state.media.clone()
    }
}

pub async fn build_app(
    pool: PgPool,
    cookie_secure: bool,
    media: MediaConfig,
) -> Result<Router, sqlx::Error> {
    // ...unchanged body...
    Ok(Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health))
        .layer(session_layer)
        .layer(/* ...unchanged... */)
        .with_state(AppState { pool, media }))
}
```

Every existing handler's `State(pool): State<PgPool>` (in
`categories.rs`, `menu_items.rs`, `orders.rs`, `admin/auth.rs`,
`admin/orders.rs`) keeps compiling **unchanged** — axum's `State`
extractor resolves through `FromRef<AppState>` automatically once the
router the handler is mounted in is generic over `AppState` instead of
`PgPool` (Group 9). Not one existing handler body changes.

## Group 4 — `apps/api/src/main.rs`: configure MinIO at startup

Depends on: Group 3. Same fail-fast pattern as `DATABASE_URL`.

```rust
use aws_sdk_s3::config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region};

fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        tracing::error!("{key} must be set");
        std::process::exit(1);
    })
}

// ...inside main(), after the Postgres pool/migrations block...

let minio_endpoint = require_env("MINIO_ENDPOINT");
let minio_access_key = require_env("MINIO_ACCESS_KEY");
let minio_secret_key = require_env("MINIO_SECRET_KEY");
let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "menu-images".into());
let public_base_url = require_env("PUBLIC_API_URL");

let s3_config = S3ConfigBuilder::new()
    .behavior_version(BehaviorVersion::latest())
    .region(Region::new("us-east-1")) // MinIO ignores region; the SDK still requires one
    .endpoint_url(minio_endpoint)
    .credentials_provider(Credentials::new(
        minio_access_key,
        minio_secret_key,
        None,
        None,
        "minio-static",
    ))
    .force_path_style(true) // MinIO requires path-style bucket addressing
    .build();
let client = aws_sdk_s3::Client::from_conf(s3_config);

// Idempotent — tolerates the bucket already existing from a prior boot,
// same "fix drift on startup" spirit as sqlx::migrate!() below.
match client.create_bucket().bucket(&bucket).send().await {
    Ok(_) => tracing::info!(%bucket, "created media bucket"),
    Err(err) if err.as_service_error().is_some_and(|e| e.is_bucket_already_owned_by_you() || e.is_bucket_already_exists()) => {
        tracing::info!(%bucket, "media bucket already exists");
    }
    Err(err) => {
        tracing::error!(%err, %bucket, "failed to create media bucket");
        std::process::exit(1);
    }
}

let media = api::MediaConfig { client, bucket, public_base_url };

// ...
let app = match api::build_app(pool, cookie_secure, media).await { /* unchanged */ };
```

## Group 5 — `apps/api/src/routes/admin/media.rs` (new): upload

Depends on: Groups 2, 3. Protected (nested under Phase 11's
`protected` sub-router).

```rust
use crate::error::{AppError, FieldError};
use crate::MediaConfig;
use axum::{extract::{DefaultBodyLimit, Multipart, State}, Json};
use serde::Serialize;

const MAX_BYTES: usize = 5 * 1024 * 1024;

fn extension_for(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub url: String,
}

pub async fn upload(
    State(media): State<MediaConfig>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    // Single "file" field is the only thing this form ever sends.
    let Some(field) = multipart.next_field().await.map_err(|_| {
        AppError::Validation(vec![FieldError { field: "file".into(), message: "missing".into() }])
    })?
    else {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "missing".into(),
        }]));
    };

    let content_type = field.content_type().unwrap_or("").to_string();
    let Some(ext) = extension_for(&content_type) else {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "unsupported_type".into(),
        }]));
    };

    let bytes = field.bytes().await.map_err(|_| {
        AppError::Validation(vec![FieldError { field: "file".into(), message: "unreadable".into() }])
    })?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::Validation(vec![FieldError {
            field: "file".into(),
            message: "too_large".into(),
        }]));
    }

    let key = format!("{}.{ext}", uuid::Uuid::new_v4());

    media
        .client
        .put_object()
        .bucket(&media.bucket)
        .key(&key)
        .content_type(content_type)
        .body(bytes.into())
        .send()
        .await
        .map_err(|err| {
            tracing::error!(%err, "upload to MinIO failed");
            AppError::NotFound // placeholder — see note below
        })?;

    Ok(Json(UploadResponse {
        url: format!("{}/api/media/{key}", media.public_base_url),
    }))
}

pub fn upload_router() -> axum::routing::MethodRouter<crate::AppState> {
    axum::routing::post(upload).layer(DefaultBodyLimit::max(6 * 1024 * 1024))
}
```

The `AppError::NotFound` placeholder on a `put_object` failure is
wrong and flagged here on purpose — give S3 write failures their own
mapping (e.g. extend `AppError` with a plain `Internal` variant, or
reuse the existing `Database`-style 500 logging shape) when
implementing; don't ship a `500` that claims `"not_found"`.

`.layer(DefaultBodyLimit::max(...))` is attached to this one route via
`upload_router()`'s returned `MethodRouter`, not globally — Group 9
mounts it with `.route("/media", admin::media::upload_router())`
instead of `.route("/media", post(admin::media::upload))`, so no other
admin endpoint's body limit changes (requirement.md Decision 14).

## Group 6 — `apps/api/src/routes/media.rs` (new): public serve

Depends on: Groups 2, 3. **Not** nested under `/admin` — registered
directly in `routes::api_router()` (Group 9), no session, no
`require_admin`.

```rust
use crate::error::AppError;
use crate::MediaConfig;
use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};

pub async fn get_media(
    State(media): State<MediaConfig>,
    Path(key): Path<String>,
) -> Result<Response, AppError> {
    let object = media
        .client
        .get_object()
        .bucket(&media.bucket)
        .key(&key)
        .send()
        .await
        .map_err(|err| {
            if err.as_service_error().is_some_and(|e| e.is_no_such_key()) {
                AppError::NotFound
            } else {
                tracing::error!(%err, %key, "media fetch from MinIO failed");
                AppError::NotFound // see Group 5's note — give this its own 500 mapping
            }
        })?;

    let content_type = object
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = object
        .body
        .collect()
        .await
        .map_err(|_| AppError::NotFound) // same placeholder caveat as above
        .map(|data| data.into_bytes())?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        bytes,
    )
        .into_response())
}
```

`key` is a flat object name with no `/` in it (Decision 4's `{uuid}.{ext}`),
so a plain `Path<String>` segment is enough — no wildcard/`{*key}` route
syntax needed.

## Group 7 — `apps/api/src/routes/admin/categories.rs` (new)

Depends on: Group 2. **Revised 2026-10-04** for `requirement.md` Decision
17 (optimistic concurrency, open risk 3) — `update_category` and
`archive_category` below are conditioned on a caller-supplied `updated_at`
and `archive_category` now takes a JSON body where it previously took
none.

```rust
use crate::error::{AppError, BlockingMenuItem, FieldError};
use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct AdminCategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_archived: bool,
    pub updated_at: DateTime<Utc>, // new — Decision 17's OCC token
}

struct CategoryRow {
    id: i64,
    name: String,
    description: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<CategoryRow> for AdminCategoryResponse {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            is_archived: row.deleted_at.is_some(),
            updated_at: row.updated_at,
        }
    }
}

pub async fn list_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AdminCategoryResponse>>, AppError> {
    let rows = sqlx::query_as!(
        CategoryRow,
        r#"SELECT id, name, description, deleted_at, updated_at FROM categories ORDER BY display_order, id"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Disambiguates a 0-row conditional UPDATE: row gone entirely -> 404,
/// row still there but `updated_at` moved on -> 409 with the live row.
async fn category_or_conflict(pool: &PgPool, id: i64) -> AppError {
    match sqlx::query_as!(
        CategoryRow,
        r#"SELECT id, name, description, deleted_at, updated_at FROM categories WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(row)) => {
            let current: AdminCategoryResponse = row.into();
            AppError::Conflict(serde_json::to_value(current).expect("serializable"))
        }
        Ok(None) => AppError::NotFound,
        Err(err) => AppError::Database(err),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>, // new — Decision 17's OCC token
}

#[derive(Debug, Deserialize)]
pub struct ArchiveRequest {
    pub updated_at: DateTime<Utc>, // new — archive now takes a body
}

fn validate_category(payload: &UpsertCategoryRequest) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation(vec![FieldError {
            field: "name".into(),
            message: "required".into(),
        }]));
    }
    Ok(())
}

pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<UpsertCategoryRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload)?;

    // Appended last (requirement.md Decision 8) — no reorder UI this phase.
    // `updated_at` on create has nothing to conflict against yet — only
    // read back for the response, not checked.
    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        INSERT INTO categories (name, description, display_order)
        VALUES ($1, $2, (SELECT COALESCE(MAX(display_order), -1) + 1 FROM categories))
        RETURNING id, name, description, deleted_at, updated_at
        "#,
        payload.name.trim(),
        payload.description,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(row.into()))
}

pub async fn update_category(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpsertCategoryRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload)?;

    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        UPDATE categories SET name = $3, description = $4, updated_at = now()
        WHERE id = $1 AND updated_at = $2
        RETURNING id, name, description, deleted_at, updated_at
        "#,
        id,
        payload.updated_at,
        payload.name.trim(),
        payload.description,
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Json(row.into())),
        None => Err(category_or_conflict(&pool, id).await),
    }
}

pub async fn archive_category(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ArchiveRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    let blocking = sqlx::query_as!(
        BlockingMenuItem,
        r#"SELECT id, name FROM menu_items WHERE category_id = $1 AND deleted_at IS NULL"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    if !blocking.is_empty() {
        return Err(AppError::CategoryHasActiveItems(blocking));
    }

    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        UPDATE categories SET deleted_at = now(), updated_at = now()
        WHERE id = $1 AND updated_at = $2
        RETURNING id, name, description, deleted_at, updated_at
        "#,
        id,
        payload.updated_at,
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Json(row.into())),
        None => Err(category_or_conflict(&pool, id).await),
    }
}
```

`BlockingMenuItem` needs `sqlx::FromRow`-compatible field order (`id`,
`name`) matching the struct in `error.rs` exactly, or `query_as!` won't
compile — keep the two in sync (same file-crossing fragility
`OrderSummaryResponse` already has in Phase 13's code, no new pattern).

The blocking-items check in `archive_category` still runs *before* the
conditional update and is unaffected by OCC — a stale `updated_at` on an
otherwise-blocked archive attempt should still surface as the more
specific `409 category_has_active_items`, not a generic conflict, so the
ordering here (check blocking items first, then attempt the conditional
update) is load-bearing.

## Group 8 — `apps/api/src/routes/admin/menu_items.rs` (new)

Depends on: Group 7 (reuses the "category must exist and not be
archived" check), Group 3 (`update_menu_item` extracts `MediaConfig`
alongside `PgPool` for open risk 2's cleanup — `FromRef<AppState>`
already covers it, no further plumbing needed). **Revised 2026-10-04**
for Decision 14's addendum (open risk 4 — `image_url` must match this
API's own issued shape) and Decision 17 (OCC, open risk 3).

```rust
use crate::error::{AppError, FieldError};
use crate::MediaConfig;
use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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
    pub updated_at: DateTime<Utc>, // new — Decision 17's OCC token
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
    pub updated_at: DateTime<Utc>, // new — ignored by create_menu_item, required by update
}

#[derive(Debug, Deserialize)]
pub struct AvailabilityRequest {
    pub is_available: bool,
    pub updated_at: DateTime<Utc>, // new — OCC applies even to this single-field toggle
}

#[derive(Debug, Deserialize)]
pub struct ArchiveMenuItemRequest {
    pub updated_at: DateTime<Utc>, // new — archive now takes a body
}

/// Decision 14's addendum (open risk 4) — `image_url`, if present, must
/// be a key this API itself issued via `POST /api/admin/media`, never an
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
) -> Result<(), AppError> {
    let mut errors = Vec::new();

    if payload.name.trim().is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    // requirement.md Decision 5 — exactly one of price / price_options.
    match (&payload.price, payload.price_options.is_empty()) {
        (Some(price), true) if *price > Decimal::ZERO => {}
        (Some(_), true) => errors.push(FieldError { field: "price".into(), message: "must_be_positive".into() }),
        (None, false) => {
            for option in &payload.price_options {
                if option.label.trim().is_empty() {
                    errors.push(FieldError { field: "price_options".into(), message: "label_required".into() });
                }
                if option.price <= Decimal::ZERO {
                    errors.push(FieldError { field: "price_options".into(), message: "must_be_positive".into() });
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
    // size/type-check bypass: an image_url that isn't one this API
    // issued never gets persisted.
    if let Some(url) = &payload.image_url {
        if !is_valid_media_url(url, &media.public_base_url) {
            errors.push(FieldError { field: "image_url".into(), message: "invalid".into() });
        }
    }

    // requirement.md Decision 7 addendum (open risk 5) — this remains
    // the fast common-case check; Group 1a's trigger is the atomic
    // backstop for the race this alone can't close.
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

    if errors.is_empty() { Ok(()) } else { Err(AppError::Validation(errors)) }
}

/// Disambiguates a 0-row conditional UPDATE the same way Group 7's
/// `category_or_conflict` does: row gone -> 404, row moved on -> 409
/// with the live item (reuses `get_menu_item` so the "current" shape in
/// a 409 is identical to what a GET would return).
async fn menu_item_or_conflict(pool: PgPool, id: i64) -> AppError {
    match get_menu_item(State(pool), Path(id)).await {
        Ok(Json(current)) => {
            AppError::Conflict(serde_json::to_value(current).expect("serializable"))
        }
        Err(err) => err, // NotFound passes through unchanged
    }
}

// list_menu_items / get helpers: joins categories for category_name,
// left-joins price_options the same grouped-in-Rust way the public
// menu_items.rs already does (requirement.md Decision 9). Omitted here
// for brevity — same shape as menu_items.rs's existing query plus
// `deleted_at` on both sides for `is_archived` / archived-options
// filtering.

pub async fn create_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Json(payload): Json<UpsertMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    validate_menu_item(&pool, &media, &payload).await?;

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

    // Re-fetch through the shared list/get query so the response always
    // matches what a subsequent GET would return.
    get_menu_item(State(pool), Path(item_id)).await
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

pub async fn update_menu_item(
    State(pool): State<PgPool>,
    State(media): State<MediaConfig>,
    Path(id): Path<i64>,
    Json(payload): Json<UpsertMenuItemRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    validate_menu_item(&pool, &media, &payload).await?;

    // Read before the transaction — just needs the pre-update value to
    // diff against below for open risk 2's image cleanup.
    let previous_image_url: Option<String> =
        sqlx::query_scalar!("SELECT image_url FROM menu_items WHERE id = $1", id)
            .fetch_optional(&pool)
            .await?
            .flatten();

    let mut tx = pool.begin().await?;

    // requirement.md Decision 17 — conditional on the caller's
    // `updated_at`; 0 affected rows means either the id is unknown or
    // someone else updated it first (disambiguated below). A
    // check_violation here (Group 1a's trigger, archived category) is
    // converted to `AppError::CategoryArchived` by the `?` below — see
    // Group 2's new `From<sqlx::Error>` branch.
    let affected = sqlx::query!(
        r#"
        UPDATE menu_items
        SET category_id = $3, name = $4, description = $5, price = $6,
            image_url = $7, is_featured = $8, updated_at = now()
        WHERE id = $1 AND updated_at = $2
        "#,
        id,
        payload.updated_at,
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

    // requirement.md Decision 4 addendum (open risk 2) — the row is
    // already committed to the new image_url, so a delete failure here
    // must never fail the request; it only leaves one orphaned object
    // behind, same as today. Only fires when the image actually changed
    // (including removal), never on an edit that leaves it untouched.
    if previous_image_url.is_some() && previous_image_url != payload.image_url {
        if let Some(key) = media_key_from_url(previous_image_url.as_deref().unwrap(), &media.public_base_url) {
            if let Err(err) = media
                .client
                .delete_object()
                .bucket(&media.bucket)
                .key(&key)
                .send()
                .await
            {
                tracing::warn!(%err, %key, "failed to delete superseded media object");
            }
        }
    }

    get_menu_item(State(pool), Path(id)).await
}

/// `image_url`s this API issues always look like
/// `{public_base_url}/api/media/{key}` (Group 5) — strip that prefix to
/// recover the object key for a `DeleteObject` call.
fn media_key_from_url(url: &str, public_base_url: &str) -> Option<String> {
    url.strip_prefix(public_base_url)?
        .strip_prefix("/api/media/")
        .map(str::to_string)
}

pub async fn update_availability(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<AvailabilityRequest>,
) -> Result<Json<AdminMenuItemResponse>, AppError> {
    let affected = sqlx::query!(
        r#"UPDATE menu_items SET is_available = $3, updated_at = now() WHERE id = $1 AND updated_at = $2"#,
        id,
        payload.updated_at,
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
        payload.updated_at,
    )
    .execute(&pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(menu_item_or_conflict(pool, id).await);
    }

    get_menu_item(State(pool), Path(id)).await
}
```

`update_menu_item`/`update_availability`/`archive_menu_item`'s
`UPDATE ... WHERE id = $1 AND updated_at = $2` with no `RETURNING` (since
`rows_affected()` is cheaper than a full row round-trip when the common
case is "yes, it matched") doesn't itself distinguish a 404 from a 409 —
`menu_item_or_conflict` is what does, by re-fetching via `get_menu_item`
after a 0-row update: `NotFound` passes straight through (unknown id),
otherwise the live row comes back wrapped in `AppError::Conflict`. The
response/404/409 ordering here is load-bearing: `get_menu_item` must
exist and be callable with a bare `PgPool` before any of `create_
menu_item`/`update_menu_item`/`update_availability`/`archive_menu_item`/
`menu_item_or_conflict` can compile — implement it first. Implementing
`get_menu_item` (list/get queries) is this group's remaining work — see
the omitted-code note above; follow `menu_items.rs`'s existing
grouped-price-options pattern, scoped by `WHERE id = $1` instead of
`category_id`, select `updated_at` alongside the other columns (every
response struct now carries it), and include `deleted_at IS NULL` only
on the `menu_item_price_options` join (not on `menu_items` itself — the
admin detail view must still load an archived item).

## Group 9 — Route registration

Depends on: Groups 5, 6, 7, 8.

**`apps/api/src/routes/mod.rs`:**

```rust
pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
        .route("/media/{key}", get(media::get_media)) // new, public
        .nest("/admin", admin::admin_router())
}
```

(add `pub mod media;` alongside the other top-level route modules.)

**`apps/api/src/routes/admin/mod.rs`:**

```rust
pub mod categories;
pub mod media;
pub mod menu_items;

pub fn admin_router() -> Router<AppState> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/orders", get(orders::list_orders))
        .route("/orders/summary", get(orders::orders_summary))
        .route("/orders/{id}", get(orders::get_order))
        .route("/orders/{id}/status", patch(orders::update_order_status))
        .route("/categories", get(categories::list_categories).post(categories::create_category))
        .route("/categories/{id}", patch(categories::update_category))
        .route("/categories/{id}/archive", post(categories::archive_category))
        .route("/menu-items", get(menu_items::list_menu_items).post(menu_items::create_menu_item))
        .route("/menu-items/{id}", patch(menu_items::update_menu_item))
        .route("/menu-items/{id}/availability", patch(menu_items::update_availability))
        .route("/menu-items/{id}/archive", post(menu_items::archive_menu_item))
        .route("/media", media::upload_router())
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
```

Both router functions' return types change from `Router<PgPool>` to
`Router<AppState>` (Group 3) — their bodies are otherwise unchanged.
**Revised 2026-10-04:** the two archive routes (`POST
/categories/{id}/archive`, `POST /menu-items/{id}/archive`) now parse a
JSON body (`{"updated_at": ...}`, Decision 17/Group 7/Group 8) where they
previously took none — no change to the route table itself, both still
just `post(...)`, only the handler signatures changed.
`admin::categories::list_categories` is reused verbatim for the admin
listing (it already returns every category including archived ones —
see Group 7; there is no separate public-vs-admin category list query,
just one function nested at both `/api/categories` ... actually it is
**not** reused at the public path: Group 7's `list_categories` lives in
`routes/admin/categories.rs` and is distinct from the existing public
`routes/categories.rs::list_categories`, which stays untouched and
still filters `deleted_at IS NULL`. Same name, different module, no
collision — don't merge them.

## Group 10 — Offline query cache

Depends on: Groups 5–9. With the dev stack up and `DATABASE_URL`
exported, from `apps/api`:

```bash
cargo sqlx prepare
```

Commit the updated `apps/api/.sqlx/`.

## Group 11 — `apps/api/tests/common.rs`: new helpers

Depends on: nothing.

```rust
#[allow(dead_code)]
pub async fn post_multipart_with_cookie(
    pool: PgPool,
    uri: &str,
    field_name: &str,
    filename: &str,
    content_type: &str,
    bytes: Vec<u8>,
    cookie: Option<&str>,
) -> (StatusCode, Value) {
    let boundary = "x-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", format!("multipart/form-data; boundary={boundary}"));
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, resp_body) = send(pool, builder.body(Body::from(body)).unwrap()).await;
    (status, resp_body)
}

#[allow(dead_code)]
pub async fn insert_category(pool: &PgPool, name: &str) -> i64 {
    sqlx::query_scalar!("INSERT INTO categories (name) VALUES ($1) RETURNING id", name)
        .fetch_one(pool)
        .await
        .unwrap()
}
```

`send()` builds a real `AppState` via `build_app`, which now needs a
`MediaConfig` too — `common.rs`'s `send()` must construct a test
`MediaConfig` pointed at the `minio` service the test database container
runs alongside (same Docker network as `#[sqlx::test]`'s ephemeral
database; MinIO itself is **not** torn down/recreated per test the way
Postgres is, so `admin_media.rs`'s tests should use a per-test-random
bucket or tolerate a shared bucket across test runs — pick whichever is
simpler when implementing; a per-test UUID-suffixed bucket name is the
safer default and costs nothing extra given uploads already get random
keys).

## Group 12 — `apps/api/tests/admin_categories.rs` (new)

Depends on: Groups 7, 9, 11. **Revised 2026-10-04 (Decision 17, open risk
3):** every `PATCH`/archive request in every test below now includes the
correct current `updated_at` (read from the preceding create/list
response) in its body unless a test is specifically exercising a stale
one — this isn't called out per-row, it's a blanket requirement on every
existing row too.

| Test | Asserts |
|---|---|
| `categories_list_requires_a_session` | no cookie → `401` |
| `category_create_requires_a_session` | no cookie → `401` |
| `category_create_rejects_an_empty_name` | `{"name":"  "}` → `400 validation_error`, `fields[0].field == "name"` |
| `category_create_appends_after_the_current_max_display_order` | seed two categories at `display_order` 0/1, create a third → its `display_order` (read via a direct `psql`-style query in the test) is `2` |
| `category_rename_updates_name_and_description` | `PATCH {"name":"New","description":"d","updated_at":...}` → `200`, re-fetch confirms both changed, and `updated_at` in the response has moved on |
| `category_rename_404s_for_an_unknown_id` | → `404 not_found` |
| `category_rename_rejects_a_stale_updated_at` | create a category, rename it once (so its real `updated_at` moves on), then `PATCH` again using the *original* (now-stale) `updated_at` → `409 {"error":"conflict","current":{...}}`, `current.name` reflects the first rename, not a 3rd value |
| `category_archive_succeeds_when_no_active_items` | → `200`, `is_archived == true` |
| `category_archive_rejects_when_active_items_remain` | seed one non-archived menu item under the category → `409 {"error":"category_has_active_items","items":[{"id":...,"name":...}]}` (not a `conflict` shape — the blocking-items check runs before the `updated_at` check, requirement.md Decision 7's addendum) |
| `category_archive_succeeds_when_only_archived_items_remain` | seed one menu item, archive it, then archive the category → `200` |
| `category_archive_rejects_a_stale_updated_at` | create an empty category, rename it (moves `updated_at`), then archive using the original stale `updated_at` → `409 conflict`, category remains non-archived (verified via a direct query) |

## Group 13 — `apps/api/tests/admin_menu_items.rs` (new)

Depends on: Groups 7, 8, 9, 11, 12 (reuses `insert_category`). **Revised
2026-10-04 (Decision 17, open risk 3; Decision 14 addendum, open risk 4;
Decision 7 addendum, open risk 5):** same blanket `updated_at`-on-every-
write requirement as Group 12.

| Test | Asserts |
|---|---|
| `menu_items_list_requires_a_session` | no cookie → `401` |
| `menu_item_create_requires_a_session` | no cookie → `401` |
| `menu_item_create_with_flat_price_succeeds` | `{"category_id":...,"name":"Jollof","price":"12.50","price_options":[],...}` → `200`, `price == "12.50"`, `price_options == []` |
| `menu_item_create_with_price_options_succeeds` | `price: null`, two `price_options` → `200`, `price` is `null`, both options present with generated ids |
| `menu_item_create_rejects_both_price_and_price_options` | both set → `400 validation_error`, `fields` names `price_options` |
| `menu_item_create_rejects_neither_price_nor_price_options` | both absent/empty → `400`, `fields` names `price` |
| `menu_item_create_rejects_a_negative_or_zero_price` | `price: "0.00"` → `400` |
| `menu_item_create_rejects_an_archived_or_unknown_category` | archive a category, try to create under it → `400 validation_error`, `fields[0].field == "category_id"` |
| `menu_item_create_rejects_an_image_url_not_issued_by_this_api` | `image_url: "https://attacker.example/x.jpg"` → `400 validation_error`, `fields[0].field == "image_url"` (Decision 14 addendum, open risk 4 — the actual bypass-closing test; distinct from `admin_media.rs`'s size/type checks on the upload endpoint itself) |
| `menu_item_create_accepts_an_image_url_from_this_apis_own_upload` | upload a real image via `POST /api/admin/media`, create a menu item with the returned `url` as `image_url` → `200`, `image_url` matches |
| `menu_item_update_replaces_price_options` | create with two options, `PATCH` with one different option → old two are `deleted_at`-set (verified via a direct query), exactly one new row exists |
| `menu_item_update_404s_for_an_unknown_id` | → `404` |
| `menu_item_update_rejects_a_stale_updated_at` | create an item, update it once (moves `updated_at`), `PATCH` again with the original stale `updated_at` → `409 {"error":"conflict","current":{...}}`, `current` reflects the first update, not a 3rd value |
| `availability_toggle_rejects_a_stale_updated_at` | create an item, flip availability once, flip again with the original stale `updated_at` → `409 conflict`, `is_available` unchanged from the first flip |
| `menu_item_archive_rejects_a_stale_updated_at` | create an item, update its name (moves `updated_at`), archive using the original stale `updated_at` → `409 conflict`, item remains non-archived |
| `menu_item_update_deletes_the_superseded_image_from_minio` | create an item with an uploaded `image_url`, `PATCH` with a second uploaded image → `200`, the new `image_url` resolves via `GET /api/media/{key}` (`200`), the **old** key now `404`s (open risk 2 cleanup) |
| `menu_item_update_with_unchanged_image_url_does_not_delete_it` | `PATCH` an item re-sending its current `image_url` unchanged (e.g. editing only the name) → the image's `GET /api/media/{key}` still `200`s afterward (guards against over-eager deletion) |
| `menu_item_archive_does_not_delete_its_image` | upload an image, create an item with it, archive the item → the image's `GET /api/media/{key}` still `200`s (Decision 4 addendum scopes cleanup to edit-driven replace/removal only, never archive) |
| `availability_toggle_flips_is_available_only` | `PATCH .../availability {"is_available": false, "updated_at": ...}` → `200`, `is_available == false`, every other field unchanged |
| `menu_item_archive_sets_deleted_at_and_is_archived_true` | → `200`, `is_archived == true`; re-fetch via admin `GET .../{id}` still `200`s (archived items stay visible to admin) |
| `archived_menu_item_is_excluded_from_the_public_menu` | archive an item, then hit the existing public `GET /api/menu-items` → the item is absent (regression guard on `menu_items.rs`'s existing `deleted_at IS NULL` filter — nothing in this phase should have broken it) |
| `category_archived_trigger_rejects_a_direct_insert_under_an_archived_category` | database-level, not HTTP: archive a category via the admin endpoint, then attempt a raw `sqlx::query!` `INSERT INTO menu_items (category_id, ...) VALUES (...)` directly against that archived category's id → the `INSERT` itself errors with SQLSTATE `23514` (Group 1a's trigger). Confirms the guard holds independent of `validate_menu_item`'s application-level pre-check, which this test deliberately bypasses by not going through the handler. |

## Group 14 — `apps/api/tests/admin_media.rs` (new)

Depends on: Groups 5, 6, 9, 11. Needs a tiny valid image fixture — the
smallest possible PNG (a handful of bytes is fine; any real `image/png`
magic-byte-prefixed buffer PutObject/GetObject will round-trip) committed
inline as a `const` byte array in the test file, not a repo asset.

| Test | Asserts |
|---|---|
| `upload_requires_a_session` | no cookie → `401` |
| `upload_rejects_an_unsupported_content_type` | `text/plain` body → `400 validation_error`, `fields[0].field == "file"` |
| `upload_rejects_an_oversized_file` | a buffer over `MAX_BYTES` with a valid `image/png` content type → `400`, `fields[0].message == "too_large"` |
| `upload_then_get_round_trips_the_same_bytes` | `POST /api/admin/media` with a valid cookie and a small PNG → `200`, `body["url"]` ends with `/api/media/{uuid}.png`; a follow-up unauthenticated `GET` to that same path (no cookie — media is public, requirement.md Decision 10) → `200`, response body bytes equal the uploaded bytes, `Content-Type: image/png` |
| `get_media_404s_for_an_unknown_key` | `GET /api/media/does-not-exist.png` → `404 not_found` |

## Group 15 — `docker-compose.yml` / `docker-compose.prod.yml`: env vars

Depends on: nothing (can be done any time before Group 4 is exercised
live).

**`docker-compose.yml`**, `api.environment`:

```yaml
MINIO_ENDPOINT: http://minio:9000
MINIO_ACCESS_KEY: ${MINIO_ROOT_USER}
MINIO_SECRET_KEY: ${MINIO_ROOT_PASSWORD}
MINIO_BUCKET: ${MINIO_BUCKET:-menu-images}
PUBLIC_API_URL: ${PUBLIC_API_URL:-http://localhost:${API_PORT:-8080}}
```

**`docker-compose.prod.yml`**, `api.environment` (added to the existing
block):

```yaml
MINIO_ENDPOINT: http://minio:9000
MINIO_ACCESS_KEY: ${MINIO_ROOT_USER}
MINIO_SECRET_KEY: ${MINIO_ROOT_PASSWORD}
MINIO_BUCKET: ${MINIO_BUCKET:-menu-images}
PUBLIC_API_URL: https://api.${DOMAIN}
```

**`docker-compose.prod.yml`**, `web.environment` (new — `web` currently
gets no environment block in prod at all):

```yaml
environment:
  DOMAIN: ${DOMAIN}
```

`.env.example` gets one new commented line noting `MINIO_BUCKET` is
optional (default `menu-images`) — `PUBLIC_API_URL` needs no entry since
dev has a working default and prod derives it from the already-required
`DOMAIN`.

## Group 16 — `apps/web/next.config.ts`: second `remotePatterns` entry

Depends on: nothing.

```ts
const nextConfig: NextConfig = {
  output: "standalone",
  agentRules: false,
  images: {
    remotePatterns: [
      { protocol: "https", hostname: "placehold.co" },
      // Dev: the API container's published port, same convention default
      // as API_PORT everywhere else. Prod: api.$DOMAIN, Caddy-fronted
      // (requirement.md Decision 2) — DOMAIN is passed into the web
      // service's environment for exactly this (docker-compose.prod.yml).
      { protocol: "http", hostname: "localhost", port: "8080" },
      ...(process.env.DOMAIN
        ? [{ protocol: "https" as const, hostname: `api.${process.env.DOMAIN}` }]
        : []),
    ],
    dangerouslyAllowSVG: true,
    contentDispositionType: "attachment",
    contentSecurityPolicy: "default-src 'self'; script-src 'none'; sandbox;",
  },
};
```

The `placehold.co` entry and its SVG-specific options are untouched
(requirement.md Decision 13).

## Group 17 — `apps/web/components/admin-nav.tsx`: flip two flags

Depends on: nothing.

```tsx
const NAV_ITEMS = [
  { label: "Dashboard", href: "/admin", enabled: true },
  { label: "Orders", href: "/admin/orders", enabled: true },
  { label: "Menu", href: "/admin/menu", enabled: true },
  { label: "Categories", href: "/admin/categories", enabled: true },
  { label: "Settings", href: "/admin/settings", enabled: false },
] as const;
```

Exactly the change Phase 13 anticipated — nothing else in that file
changes.

## Group 18 — `apps/web/lib/admin-categories.ts` (new)

Depends on: nothing (parallels `lib/admin-orders.ts`'s shape).

```ts
import { adminApiJson } from "@/lib/admin-api";

export interface AdminCategory {
  id: number;
  name: string;
  description: string | null;
  is_archived: boolean;
  updated_at: string; // new — Decision 17's OCC token, echoed back on write
}

export function getAdminCategories(): Promise<AdminCategory[]> {
  return adminApiJson<AdminCategory[]>("/api/admin/categories");
}
```

## Group 19 — `apps/web/lib/admin-menu-items.ts` (new)

Depends on: Group 18 (shares nothing directly, just co-located).

```ts
import { adminApiJson } from "@/lib/admin-api";

export interface AdminPriceOption {
  id: number;
  label: string;
  price: string;
}

export interface AdminMenuItem {
  id: number;
  category_id: number;
  category_name: string;
  name: string;
  description: string | null;
  price: string | null;
  image_url: string | null;
  is_available: boolean;
  is_featured: boolean;
  is_archived: boolean;
  price_options: AdminPriceOption[];
  updated_at: string; // new — Decision 17's OCC token, echoed back on write
}

export function getAdminMenuItems(): Promise<AdminMenuItem[]> {
  return adminApiJson<AdminMenuItem[]>("/api/admin/menu-items");
}

export function getAdminMenuItem(id: number): Promise<AdminMenuItem> {
  return adminApiJson<AdminMenuItem>(`/api/admin/menu-items/${id}`);
}
```

No `isOrderStatus`-style guard needed here — neither page takes an
untrusted `searchParams`-driven union the way the orders list did.

## Group 20 — Route Handler proxies (new)

Depends on: Group 9. All follow
`app/api/admin/orders/[id]/status/route.ts`'s cookie-forwarding shape
(requirement.md Decision 1/9) — one file per admin write, each a thin
pass-through with no business logic of its own:

- `apps/web/app/api/admin/categories/route.ts` — `POST` → `/api/admin/categories`.
- `apps/web/app/api/admin/categories/[id]/route.ts` — `PATCH` → `/api/admin/categories/{id}`.
- `apps/web/app/api/admin/categories/[id]/archive/route.ts` — `POST` → `/api/admin/categories/{id}/archive`.
- `apps/web/app/api/admin/menu-items/route.ts` — `POST` → `/api/admin/menu-items`.
- `apps/web/app/api/admin/menu-items/[id]/route.ts` — `PATCH` → `/api/admin/menu-items/{id}`.
- `apps/web/app/api/admin/menu-items/[id]/availability/route.ts` — `PATCH` → `/api/admin/menu-items/{id}/availability`.
- `apps/web/app/api/admin/menu-items/[id]/archive/route.ts` — `POST` → `/api/admin/menu-items/{id}/archive`.
- `apps/web/app/api/admin/media/route.ts` — `POST`, the one proxy that
  forwards a body it never parses: pass `request.body` straight through
  along with the inbound `Content-Type` (which carries the multipart
  boundary) — do **not** read it as `request.text()`/`request.json()`
  first, that would corrupt the binary multipart body the existing JSON
  proxies' `.text()` pattern assumes.

```ts
// apps/web/app/api/admin/media/route.ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/media`, {
    method: "POST",
    headers: {
      "Content-Type": request.headers.get("content-type") ?? "",
      Cookie: request.headers.get("cookie") ?? "",
    },
    body: request.body,
    // @ts-expect-error -- `duplex` is required by undici for a streamed
    // request body but isn't in the Fetch API's TS types yet.
    duplex: "half",
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": apiResponse.headers.get("content-type") ?? "application/json" },
  });
}
```

A `413` from the API's `DefaultBodyLimit` layer (requirement.md open
risk 4) comes back as plain text, not JSON — this proxy still forwards
it faithfully with whatever `Content-Type` the API responded with
(`text/plain`, typically), so Group 21's upload component must not
assume every non-2xx response body is JSON.

**Revised 2026-10-04:** the two archive proxies (`categories/[id]/archive`,
`menu-items/[id]/archive`) now forward a JSON body (`{"updated_at":
...}`, Decision 17) where previously they forwarded none — no proxy code
change needed, since both already follow the generic
request-body-passthrough shape every other proxy in this group uses
(only the media proxy above is special-cased for its binary body); a
409 `{"error": "conflict", "current": {...}}` response passes through
these proxies completely unchanged, same as any other non-2xx JSON
response.

## Group 21 — New components

Depends on: Groups 18, 19, 20. **`apps/web/package.json` gains one new
dependency, added 2026-10-04 (open risk 4):**

```json
"react-easy-crop": "^6"
```

(`tech-stack.md`'s Object/Image Storage section.) No peer-dependency
pin needed beyond what's already in `package.json` — its `react`/
`react-dom` peer range (`>=16.4.0`) already covers this project's
React 19.

**`apps/web/components/admin-image-upload.tsx`** (`"use client"`) —
**revised 2026-10-04 for Decision 14's addendum (open risk 4):** a file
input that, on selection, opens an inline `react-easy-crop` `<Cropper>`
over the locally-chosen file's object URL with `aspect={1}` (fixed 1:1
square, no other ratio offered) and a "Use this photo" button. That
button reads `croppedAreaPixels` from `react-easy-crop`'s `onCropComplete`
callback, draws the selected region onto an offscreen `<canvas>` sized to
the crop (not the full original resolution — keeps the exported file
small by construction, on top of the backend's hard cap) via
`canvas.getContext("2d").drawImage(image, ...)`, and exports it with
`canvas.toBlob(..., file.type)` — preserving the original file's
`image/jpeg`/`image/png`/`image/webp` MIME type so the backend's
content-type check still sees one of the three it accepts. *That* `Blob`
— never the original `File` — is what gets uploaded via `POST
/api/admin/media`, immediately on confirming the crop. The component
shows a preview (`<img>`, not `next/image` — a locally-chosen file's
object URL isn't a configured remote pattern) while uploading, and calls
an `onUploaded(url: string)` prop on success. On a non-JSON (e.g. `413`)
response, show a generic "Upload failed — try a smaller image" `Notice`
rather than attempting `response.json()`.

**`apps/web/components/admin-price-options-editor.tsx`** (`"use client"`)
— toggles between "flat price" (single `field` input) and "multiple
sizes" (a dynamic list of label+price row pairs with add/remove
buttons), lifting its value up via an
`onChange({ price, price_options })` prop so the parent form owns the
actual submission. Mirrors `AdminOrderStatusSelect`'s local-state-plus-
callback shape, not a new pattern.

**`apps/web/components/admin-menu-item-form.tsx`** (`"use client"`) —
shared by both the "new" and "edit" pages (`initialValue?: AdminMenuItem`
prop, undefined for create). Fields: name, description, category
`<select>` (populated from `getAdminCategories()`, passed in as a prop
from the server component page — categories are fetched server-side,
not re-fetched client-side), the price-options editor, the image
upload component, an `is_featured` checkbox. Submits `POST
/api/admin/menu-items` or `PATCH /api/admin/menu-items/{id}` depending
on whether `initialValue` is set, then `router.push("/admin/menu")` and
`router.refresh()` the list. Renders field-level errors from a
`400 validation_error` response by matching `fields[].field` against
each input (same shape the backend emits, no client-side duplicate
validation beyond basic "don't submit while empty" UX). **Revised
2026-10-04 (Decision 17, open risk 3):** on edit, the submitted payload
includes `updated_at: initialValue.updated_at` unchanged from what was
loaded (a hidden value, not a visible field); create ignores/omits it
either way since there's nothing to conflict against yet. A `409
conflict` response (body: `{"current": {...fresh item}}`) is rendered as
a blocking `Notice` — *"This item was changed elsewhere since you opened
it"* — with the submit button disabled until the admin reloads the page
(`router.refresh()` plus re-reading `initialValue`), rather than silently
retrying or overwriting the other change.

**`apps/web/components/admin-category-form.tsx`** (`"use client"`) —
name + description, same create/edit dual-mode shape, much smaller.
Same `updated_at`-echo-on-edit and `409 conflict` handling as
`admin-menu-item-form.tsx` above.

## Group 22 — New pages

Depends on: Groups 18, 19, 21.

- `apps/web/app/admin/(protected)/menu/page.tsx` — list (table: image
  thumbnail or `ImageSlot`, name, category, price or "N sizes", available
  toggle [calls the `/availability` proxy directly with the row's current
  `updated_at` (Decision 17), optimistic update + revert-on-failure
  exactly like `AdminOrderStatusSelect` — a `409` reverts the toggle and
  shows "Changed elsewhere — refresh" the same way a network failure
  would], archived badge, edit link, archive button [also sends the row's
  `updated_at`; a `409` here shows the same "changed elsewhere" message
  instead of archiving] with a native `confirm()`-free pattern — a simple
  two-step "Archive? [Confirm]" inline toggle, since `window.confirm` is
  a blocking dialog this codebase has no precedent for and the
  GSAP/Claude-in-Chrome dialog-avoidance guidance applies here too).
- `apps/web/app/admin/(protected)/menu/new/page.tsx` — fetches
  `getAdminCategories()` server-side, renders `AdminMenuItemForm` with no
  `initialValue`.
- `apps/web/app/admin/(protected)/menu/[id]/page.tsx` — fetches
  `getAdminMenuItem(id)` and `getAdminCategories()`, renders
  `AdminMenuItemForm` with `initialValue`.
- `apps/web/app/admin/(protected)/categories/page.tsx` — list (name,
  description, archived badge, rename link, archive button [sends the
  row's `updated_at`, Decision 17]). An archive attempt that comes back
  `409 category_has_active_items` surfaces the blocking items by name in
  a `Notice` (e.g. *"Can't archive — Jollof Rice, Fried Rice still use
  this category"*) rather than a generic failure message, since the API
  already hands back exactly that list; a plain `409 conflict` (someone
  else edited the category first, no blocking items involved) shows the
  "changed elsewhere — refresh" message instead.
- `apps/web/app/admin/(protected)/categories/new/page.tsx` — renders
  `AdminCategoryForm` with no `initialValue`.
- `apps/web/app/admin/(protected)/categories/[id]/page.tsx` — fetches
  the one category (client-side filter over `getAdminCategories()`'s
  result, or add a tiny `getAdminCategory(id)` helper if a dedicated
  `GET /api/admin/categories/{id}` turns out cleaner when implementing —
  not currently in Group 9's route list, add it there if so) and renders
  `AdminCategoryForm` with `initialValue`.

All pages: `export const dynamic = "force-dynamic"` (same reason every
other admin page under `(protected)` already has it — the layout's own
auth check already forces this, but each page fetching fresh admin data
needs it independently uncached too, same precedent as Phase 13's order
pages). All use `next/link`, never `@/i18n/navigation` (requirement.md
Decision 15).

## Group 23 — Nothing else changes

- `apps/web/proxy.ts` — already excludes `/admin` and `/api` from the
  next-intl matcher. Untouched.
- `apps/web/messages/{en,nl}.json` — admin is English-only. Untouched.
- The public `GET /api/categories` / `GET /api/menu-items` handlers and
  the public menu page — unaffected by anything in this phase
  (requirement.md Out of scope).
- No database migration **beyond Group 1a** (added 2026-10-04 for open
  risk 5 — see this plan's intro).

## Verification

See `validation.md`.
