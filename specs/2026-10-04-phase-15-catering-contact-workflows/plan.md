# Phase 15 — Catering & Contact Workflows: Implementation Plan

Groups 1–9 touch only `apps/api`. Groups 10–22 touch only `apps/web`. No
`docker-compose*.yml` / infra changes this phase — both new tables live in
the existing Postgres instance, both new endpoints run in the existing
`api` container, no new env vars, and the rate limiter (Group 3) needs no
Caddyfile change (see its note on why).

**Revision note (2026-10-04):** this plan was revisited before
implementation started to resolve all three open risks flagged during the
original planning pass (see requirement.md's "Risks resolved in this
revisit"). That added Group 3 and Group 17 below, and touched Groups 1,
4, 5, 7, 9–11, 13–16, and 18 — every other group is unchanged from the
original plan.

## Group 0 — Branch

Branched `2026-10-04-phase-15-catering-contact-workflows` off `master`
(Phase 14 already merged).

## Group 1 — New migration: `catering_enquiries` / `contact_messages`

Depends on: nothing. Run `sqlx migrate add -r add_enquiries` from
`apps/api` to get the real timestamp-prefixed filenames (shown here as
`<ts>` — don't hand-pick one).

**`apps/api/migrations/<ts>_add_enquiries.up.sql`:**

```sql
CREATE TABLE catering_enquiries (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT NOT NULL,
    event_type TEXT NOT NULL
        CHECK (event_type IN ('wedding', 'birthday', 'corporate', 'family_gathering', 'outdoor_event', 'other')),
    event_date DATE NOT NULL,
    guest_count INTEGER NOT NULL CHECK (guest_count > 0),
    location TEXT NOT NULL,
    services_required TEXT[] NOT NULL,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE contact_messages (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT NOT NULL,
    subject TEXT NOT NULL
        CHECK (subject IN ('general', 'catering', 'event_rental', 'menu', 'order', 'other')),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

(requirement.md Decision 4 — every enum in this migration, including
`contact_messages.subject`, is snake_case, consistent with the rest of the
schema. `contact/page.tsx`'s existing static `<option value="eventRental">`
is updated to `event_rental` in Group 16 as part of swapping in the real
form — see "Risks resolved in this revisit" in requirement.md. Note that
neither table has a `website` column — the honeypot field (Group 3) is
deliberately never persisted, real or tripped.)

**`apps/api/migrations/<ts>_add_enquiries.down.sql`:**

```sql
DROP TABLE contact_messages;
DROP TABLE catering_enquiries;
```

## Group 2 — `apps/api/src/validation.rs` (new): shared validators

Depends on: nothing. Moved out of `orders.rs` verbatim (requirement.md
Decision 3) — same function bodies, same consts, just relocated.

```rust
const ASCENDING_DIGITS: &str = "01234567890123456789";
const DESCENDING_DIGITS: &str = "09876543210987654321";

pub fn is_valid_email(value: &str) -> bool {
    match value.split_once('@') {
        Some((local, domain)) => !local.is_empty() && domain.contains('.'),
        None => false,
    }
}

pub fn is_valid_phone(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return false };
    let rest = chars.as_str();

    let starts_ok = match first {
        '+' => rest.chars().next().is_some_and(|c| c.is_ascii_digit() && c != '0'),
        '0' => true,
        _ => false,
    };
    if !starts_ok {
        return false;
    }
    if !rest.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '(' | ')' | ' ')) {
        return false;
    }

    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 7 || digits.len() > 15 {
        return false;
    }
    if digits.bytes().all(|b| b == digits.as_bytes()[0]) {
        return false;
    }
    if ASCENDING_DIGITS.contains(&digits) || DESCENDING_DIGITS.contains(&digits) {
        return false;
    }

    true
}
```

**`apps/api/src/routes/orders.rs`:** delete the inlined copies of
`is_valid_email`/`is_valid_phone`/the two digit consts; add
`use crate::validation::{is_valid_email, is_valid_phone};`. No other
change to this file — `create_order`'s own body, `validate()`, and every
test in `tests/order_submission.rs` are unaffected (same function
signatures, same behavior, just a different module).

**`apps/api/src/lib.rs`:** add `pub mod validation;` alongside the
existing `pub mod error;` / `pub mod routes;` declarations.

## Group 3 — `apps/api/src/rate_limit.rs` (new): shared spam/abuse infra

Depends on: nothing. New for this revisit (requirement.md Decision 14) —
backs both Group 4 and Group 5's honeypot check and Group 7's
`GovernorLayer` registration. Full rationale in `tech-stack.md`'s "Spam/Abuse
Mitigation (Phase 15)" section; this group is the implementation of that
design.

**`apps/api/Cargo.toml`:** add `tower_governor = { version = "0.8", features = ["axum"] }`.
Already-present `axum = "0.8"` / `tower = "0.5"` match what `tower_governor
0.8`'s `axum` feature requires — no version bump to either needed.

**Why a custom key extractor, not the crate's built-in ones:** this app's
browser traffic reaches `apps/api` only through `apps/web`'s same-origin
Route Handler proxy (Decision 10) — a server-to-server `fetch` over the
Docker-internal network. The TCP peer `apps/api` sees for every request on
that path is the `web` container itself, not the visitor, so a peer-IP (or
"smart IP" guessing from headers it didn't ask for) extractor would put
every visitor in the same bucket. Instead: Group 10's proxies forward the
real client IP (already on their own incoming request's `X-Forwarded-For`,
set by Caddy) straight through to `apps/api` in the same header, and this
extractor reads it.

```rust
use axum::extract::Request;

pub fn client_ip_key<B>(req: &Request<B>) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        // Caddy appends (never replaces) an incoming X-Forwarded-For, so
        // the *last* entry is always the hop Caddy itself observed —
        // trusting the first entry would let a client spoof its own key.
        .and_then(|value| value.rsplit(',').next())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        // No header at all (local dev, no Caddy in front): one shared
        // bucket rather than wiring up `ConnectInfo` for a peer-IP
        // fallback that only matters where this app isn't actually
        // deployed — every real deployment sits behind Caddy.
        .unwrap_or_else(|| "unknown".to_string())
}
```

**Verify before writing real code:** the exact shape of `tower_governor`
0.8's `KeyExtractor` trait (method name, associated error type) and
`GovernorConfigBuilder`/`GovernorLayer` construction against that crate's
current docs.rs page — this plan was written without a compiled example to
check against. The function above (the actual IP-selection logic, which
is this plan's own design, not the crate's) should plug into whatever the
real trait signature turns out to be; adapt the wrapper, not the logic.

```rust
// Illustrative — confirm exact trait/builder shapes against tower_governor
// 0.8's docs before committing to this.
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::KeyExtractor, GovernorLayer};

#[derive(Clone)]
pub struct ForwardedForKeyExtractor;

impl KeyExtractor for ForwardedForKeyExtractor {
    type Key = String;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, impl axum::response::IntoResponse> {
        Ok::<_, std::convert::Infallible>(client_ip_key(req))
    }
}

pub fn enquiry_rate_limiter() -> GovernorLayer<ForwardedForKeyExtractor> {
    let config = GovernorConfigBuilder::default()
        .key_extractor(ForwardedForKeyExtractor)
        .per_second(30) // refill one token every 30s...
        .burst_size(5)  // ...allowing short bursts up to 5
        .finish()
        .expect("rate limiter config is valid");

    GovernorLayer::new(config)
}
```

Two independent instances of this layer are applied in Group 7 — one on
`/catering-enquiries`, one on `/contact-messages` — each with its own
bucket per client IP, not a combined budget across both endpoints. If the
crate recommends a periodic `retain_recent()`/cleanup background task to
bound the rate limiter's own memory use (check its docs), spawn it once at
startup in `main.rs` alongside the existing `tokio::spawn` calls, if any,
or directly in `main`'s async body before `axum::serve`.

**Honeypot:** no separate function needed here — it's a one-line check
duplicated in Group 4 and Group 5's handlers (`payload.website`), not
worth its own shared helper for a single `str::is_empty` check on two
otherwise-unrelated request structs.

## Group 4 — `apps/api/src/routes/catering_enquiries.rs` (new): public create

Depends on: Group 2, Group 3. Registered as a top-level public route
(Group 7), no session, with Group 3's rate limiter layered on in Group 7.

```rust
use crate::error::{AppError, FieldError};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{http::StatusCode, Json};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use axum::extract::State;

const EVENT_TYPES: [&str; 6] =
    ["wedding", "birthday", "corporate", "family_gathering", "outdoor_event", "other"];
const SERVICES: [&str; 4] = ["catering", "meal_delivery", "event_rental", "other"];
const LOCATION_MIN_LENGTH: usize = 5;
const MESSAGE_MAX_LENGTH: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct CreateCateringEnquiryRequest {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub location: String,
    pub services_required: Vec<String>,
    pub message: Option<String>,
    // Honeypot (tech-stack.md "Spam/Abuse Mitigation"). Real users never
    // see or fill this field; `#[serde(default)]` so older/other callers
    // that omit it entirely still deserialize.
    #[serde(default)]
    pub website: String,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

fn validate(payload: &CreateCateringEnquiryRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    let trimmed_phone = payload.phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError { field: "phone".into(), message: "required".into() });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError { field: "phone".into(), message: "invalid".into() });
    }

    if payload.email.trim().is_empty() {
        errors.push(FieldError { field: "email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.email) {
        errors.push(FieldError { field: "email".into(), message: "invalid".into() });
    }

    if !EVENT_TYPES.contains(&payload.event_type.as_str()) {
        errors.push(FieldError { field: "event_type".into(), message: "invalid".into() });
    }

    // requirement.md Decision 7 — must not be in the past.
    if payload.event_date < Utc::now().date_naive() {
        errors.push(FieldError { field: "event_date".into(), message: "must_be_future".into() });
    }

    if payload.guest_count <= 0 {
        errors.push(FieldError { field: "guest_count".into(), message: "must_be_positive".into() });
    }

    let trimmed_location = payload.location.trim();
    if trimmed_location.is_empty() {
        errors.push(FieldError { field: "location".into(), message: "required".into() });
    } else if trimmed_location.chars().count() < LOCATION_MIN_LENGTH {
        errors.push(FieldError { field: "location".into(), message: "too_short".into() });
    }

    // requirement.md Decision 5 — non-empty, every element a known value.
    if payload.services_required.is_empty() {
        errors.push(FieldError { field: "services_required".into(), message: "required".into() });
    } else if payload.services_required.iter().any(|s| !SERVICES.contains(&s.as_str())) {
        errors.push(FieldError { field: "services_required".into(), message: "invalid".into() });
    }

    if let Some(message) = &payload.message
        && message.chars().count() > MESSAGE_MAX_LENGTH
    {
        errors.push(FieldError { field: "message".into(), message: "too_long".into() });
    }

    errors
}

pub async fn create_catering_enquiry(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateCateringEnquiryRequest>,
) -> Result<(StatusCode, Json<CateringEnquiryResponse>), AppError> {
    // Honeypot tripped: pretend success, touch nothing. Checked before
    // validate()/the DB so a scripted submitter gets no signal at all
    // about which of its other fields were wrong.
    if !payload.website.trim().is_empty() {
        return Ok((
            StatusCode::CREATED,
            Json(CateringEnquiryResponse { id: 0, created_at: Utc::now() }),
        ));
    }

    let errors = validate(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required, message)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, created_at
        "#,
        payload.name.trim(),
        payload.phone.trim(),
        payload.email.trim(),
        payload.event_type,
        payload.event_date,
        payload.guest_count,
        payload.location.trim(),
        &payload.services_required,
        payload.message.as_deref().map(str::trim),
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(CateringEnquiryResponse { id: row.id, created_at: row.created_at })))
}
```

`EVENT_TYPES`/`SERVICES` mirror the migration's `CHECK` constraint values
exactly (Group 1) — keep both in sync if either list ever changes, same
fragility `ORDER_STATUSES` already has against `orders.status`'s `CHECK`.

## Group 5 — `apps/api/src/routes/contact_messages.rs` (new): public create

Depends on: Group 2, Group 3. Same shape as Group 4, simpler payload.

```rust
use crate::error::{AppError, FieldError};
use crate::validation::{is_valid_email, is_valid_phone};
use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const SUBJECTS: [&str; 6] = ["general", "catering", "event_rental", "menu", "order", "other"];
const MESSAGE_MAX_LENGTH: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct CreateContactMessageRequest {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub subject: String,
    pub message: String,
    // Honeypot — see Group 4's identical field for the full rationale.
    #[serde(default)]
    pub website: String,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

fn validate(payload: &CreateContactMessageRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    let trimmed_name = payload.name.trim();
    if trimmed_name.is_empty() {
        errors.push(FieldError { field: "name".into(), message: "required".into() });
    }

    if payload.email.trim().is_empty() {
        errors.push(FieldError { field: "email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.email) {
        errors.push(FieldError { field: "email".into(), message: "invalid".into() });
    }

    let trimmed_phone = payload.phone.trim();
    if trimmed_phone.is_empty() {
        errors.push(FieldError { field: "phone".into(), message: "required".into() });
    } else if !is_valid_phone(trimmed_phone) {
        errors.push(FieldError { field: "phone".into(), message: "invalid".into() });
    }

    if !SUBJECTS.contains(&payload.subject.as_str()) {
        errors.push(FieldError { field: "subject".into(), message: "invalid".into() });
    }

    let trimmed_message = payload.message.trim();
    if trimmed_message.is_empty() {
        errors.push(FieldError { field: "message".into(), message: "required".into() });
    } else if trimmed_message.chars().count() > MESSAGE_MAX_LENGTH {
        errors.push(FieldError { field: "message".into(), message: "too_long".into() });
    }

    errors
}

pub async fn create_contact_message(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateContactMessageRequest>,
) -> Result<(StatusCode, Json<ContactMessageResponse>), AppError> {
    // Honeypot tripped — see Group 4's identical check for the rationale.
    if !payload.website.trim().is_empty() {
        return Ok((
            StatusCode::CREATED,
            Json(ContactMessageResponse { id: 0, created_at: Utc::now() }),
        ));
    }

    let errors = validate(&payload);
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO contact_messages (name, email, phone, subject, message)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, created_at
        "#,
        payload.name.trim(),
        payload.email.trim(),
        payload.phone.trim(),
        payload.subject,
        payload.message.trim(),
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(ContactMessageResponse { id: row.id, created_at: row.created_at })))
}
```

## Group 6 — `apps/api/src/routes/admin/enquiries.rs` (new): read-only admin list/detail

Depends on: Group 1. Protected (nested under Phase 11's `protected`
sub-router). One file for both resources (requirement.md Decision 11).

```rust
use crate::error::AppError;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn clamp(params: &ListQuery) -> (i64, i64) {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);
    (limit, offset)
}

#[derive(Debug, Serialize)]
pub struct CateringEnquirySummary {
    pub id: i64,
    pub name: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryDetail {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub location: String,
    pub services_required: Vec<String>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CateringEnquiryListResponse {
    pub enquiries: Vec<CateringEnquirySummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_catering_enquiries(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<CateringEnquiryListResponse>, AppError> {
    let (limit, offset) = clamp(&params);

    let enquiries = sqlx::query_as!(
        CateringEnquirySummary,
        r#"SELECT id, name, event_type, event_date, guest_count, created_at
           FROM catering_enquiries ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let total = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM catering_enquiries"#)
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    Ok(Json(CateringEnquiryListResponse { enquiries, total, limit, offset }))
}

pub async fn get_catering_enquiry(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<CateringEnquiryDetail>, AppError> {
    let enquiry = sqlx::query_as!(
        CateringEnquiryDetail,
        r#"SELECT id, name, phone, email, event_type, event_date, guest_count, location,
                  services_required, message, created_at
           FROM catering_enquiries WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(enquiry))
}

#[derive(Debug, Serialize)]
pub struct ContactMessageSummary {
    pub id: i64,
    pub name: String,
    pub subject: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageDetail {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub subject: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ContactMessageListResponse {
    pub messages: Vec<ContactMessageSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_contact_messages(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ContactMessageListResponse>, AppError> {
    let (limit, offset) = clamp(&params);

    let messages = sqlx::query_as!(
        ContactMessageSummary,
        r#"SELECT id, name, subject, created_at
           FROM contact_messages ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    let total = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM contact_messages"#)
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

    Ok(Json(ContactMessageListResponse { messages, total, limit, offset }))
}

pub async fn get_contact_message(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<ContactMessageDetail>, AppError> {
    let message = sqlx::query_as!(
        ContactMessageDetail,
        r#"SELECT id, name, email, phone, subject, message, created_at
           FROM contact_messages WHERE id = $1"#,
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(message))
}
```

## Group 7 — Route registration

Depends on: Groups 3, 4, 5, 6.

**`apps/api/src/routes/mod.rs`:**

```rust
pub mod catering_enquiries;
pub mod contact_messages;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
        .route(
            "/catering-enquiries",
            post(catering_enquiries::create_catering_enquiry)
                .route_layer(crate::rate_limit::enquiry_rate_limiter()), // new, public, rate-limited
        )
        .route(
            "/contact-messages",
            post(contact_messages::create_contact_message)
                .route_layer(crate::rate_limit::enquiry_rate_limiter()), // new, public, rate-limited
        )
        .route("/media/{key}", get(media::get_media))
        .nest("/admin", admin::admin_router())
}
```

Each route gets its own `enquiry_rate_limiter()` call — two independent
`GovernorLayer` instances, two independent per-IP buckets (Group 3), not
one shared budget across both endpoints.

**`apps/api/src/lib.rs`:** add `pub mod rate_limit;` alongside `pub mod
validation;`.

**`apps/api/src/routes/admin/mod.rs`:**

```rust
pub mod enquiries;

pub fn admin_router() -> Router<AppState> {
    let protected = Router::new()
        // ...existing routes unchanged...
        .route("/catering-enquiries", get(enquiries::list_catering_enquiries))
        .route("/catering-enquiries/{id}", get(enquiries::get_catering_enquiry))
        .route("/contact-messages", get(enquiries::list_contact_messages))
        .route("/contact-messages/{id}", get(enquiries::get_contact_message))
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
```

(No rate limiting on the admin `GET` routes — they're session-protected,
not public, out of this risk's scope.)

## Group 8 — Offline query cache

Depends on: Groups 1–7. With the dev stack up and `DATABASE_URL`
exported, from `apps/api`:

```bash
cargo sqlx prepare
```

Commit the updated `apps/api/.sqlx/`.

## Group 9 — Backend integration tests

Depends on: Groups 4–8.

**`apps/api/tests/catering_enquiries.rs` (new):**

| Test | Asserts |
|---|---|
| `catering_enquiry_create_succeeds_with_valid_payload` | full valid payload → `201`, `id` present |
| `catering_enquiry_create_requires_no_session` | regression guard: no cookie → still `201` (this is public, not `401`) |
| `catering_enquiry_create_rejects_an_empty_name` | → `400`, `fields[0].field == "name"` |
| `catering_enquiry_create_rejects_an_invalid_phone` | e.g. `"1234567890"` (ascending run) → `400`, `fields` names `phone` |
| `catering_enquiry_create_rejects_an_unknown_event_type` | `"quinceañera"` → `400`, `fields` names `event_type` |
| `catering_enquiry_create_rejects_a_past_event_date` | yesterday's date → `400`, `fields` names `event_date`, message `"must_be_future"` |
| `catering_enquiry_create_rejects_a_zero_guest_count` | `guest_count: 0` → `400`, `fields` names `guest_count` |
| `catering_enquiry_create_rejects_an_empty_services_required` | `services_required: []` → `400`, `fields` names `services_required` |
| `catering_enquiry_create_rejects_an_unknown_service` | `services_required: ["catering", "bouncy_castle"]` → `400`, `fields` names `services_required` |
| `catering_enquiry_create_accepts_a_null_message` | `message: null` → `201` |
| `catering_enquiry_create_persists_services_required_as_submitted` | `["catering", "event_rental"]` → re-fetched via a direct query, array matches exactly and in order |
| `catering_enquiry_create_honeypot_filled_reports_success_but_persists_nothing` | valid payload plus `website: "http://spam.example"` → `201`, then a direct `SELECT COUNT(*) FROM catering_enquiries` confirms no row was inserted |

**`apps/api/tests/contact_messages.rs` (new):**

| Test | Asserts |
|---|---|
| `contact_message_create_succeeds_with_valid_payload` | full valid payload → `201` |
| `contact_message_create_rejects_an_unknown_subject` | `subject: "refund"` → `400`, `fields` names `subject` |
| `contact_message_create_rejects_an_empty_message` | `message: "   "` → `400`, `fields` names `message`, message `"required"` |
| `contact_message_create_rejects_an_overlong_message` | 1001+ chars → `400`, `fields` names `message`, message `"too_long"` |
| `contact_message_create_accepts_event_rental_subject` | `subject: "event_rental"` → `201` (regression guard for the snake_case fix) |
| `contact_message_create_honeypot_filled_reports_success_but_persists_nothing` | valid payload plus `website: "http://spam.example"` → `201`, `SELECT COUNT(*) FROM contact_messages` confirms no row was inserted |

**`apps/api/tests/rate_limit.rs` (new):**

| Test | Asserts |
|---|---|
| `catering_enquiry_create_rate_limits_after_burst_size_requests` | 6 valid submissions in quick succession from the test client (same synthetic `X-Forwarded-For`) → the 6th is `429`, not `201`/`400` |
| `catering_enquiry_create_rate_limits_independently_per_forwarded_for_value` | 5 requests with `X-Forwarded-For: 1.1.1.1` exhaust that bucket; a 6th request with `X-Forwarded-For: 2.2.2.2` still succeeds (`201`) |
| `contact_message_create_rate_limits_after_burst_size_requests` | same shape as the first catering test, for `/api/contact-messages` |

**`apps/api/tests/admin_enquiries.rs` (new):**

| Test | Asserts |
|---|---|
| `catering_enquiries_list_requires_a_session` | no cookie → `401` |
| `catering_enquiries_list_orders_newest_first` | seed three enquiries, list → order matches insertion reverse |
| `catering_enquiry_detail_requires_a_session` | no cookie → `401` |
| `catering_enquiry_detail_404s_for_an_unknown_id` | → `404 not_found` |
| `catering_enquiry_detail_returns_every_field` | seed one with every field populated → response includes `phone`, `location`, `services_required`, `message` (not just the summary fields) |
| `contact_messages_list_requires_a_session` | no cookie → `401` |
| `contact_message_detail_requires_a_session` | no cookie → `401` |
| `contact_message_detail_404s_for_an_unknown_id` | → `404 not_found` |
| `admin_list_pagination_respects_limit_and_offset` | seed 3 catering enquiries, `?limit=1&offset=1` → exactly the second-newest one, `total == 3` |

`apps/api/tests/common.rs` needs no new helpers for the enquiry/contact
tests — `send()`/the existing cookie-login helper already cover every case
above (no multipart, no MinIO dependency, unlike Phase 14's media tests).
`tests/rate_limit.rs` does need a small helper to set a custom
`X-Forwarded-For` header on an outgoing test request if `common.rs`'s
`send()` doesn't already expose request-header overrides — add one there
rather than duplicating request-building logic in the new test file.

## Group 10 — `apps/web/app/api/catering-enquiries/route.ts` / `apps/web/app/api/contact-messages/route.ts` (new): proxies

Depends on: Group 7. Both are *almost* a verbatim copy of
`apps/web/app/api/orders/route.ts` — the one addition is forwarding the
real client IP for Group 3's rate limiter (requirement.md Decision 14;
`apps/web/app/api/orders/route.ts` itself is untouched, `/api/orders` has
no rate limiter to feed).

**`apps/web/app/api/catering-enquiries/route.ts`:**

```ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();
  const forwardedFor = request.headers.get("x-forwarded-for");

  const apiResponse = await fetch(`${API_BASE_URL}/api/catering-enquiries`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(forwardedFor ? { "X-Forwarded-For": forwardedFor } : {}),
    },
    body,
  });

  const responseBody = await apiResponse.text();
  return new NextResponse(responseBody, {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
```

**`apps/web/app/api/contact-messages/route.ts`:** identical, forwarding to
`${API_BASE_URL}/api/contact-messages`.

Local dev has no Caddy in front of `web`, so `request.headers.get("x-forwarded-for")`
is normally `null` there — `forwardedFor` is omitted entirely rather than
sent as an empty string, so `apps/api`'s extractor (Group 3) falls through
to its `"unknown"` shared-bucket default, same as a direct `curl` to
`apps/api` with no header.

## Group 11 — `apps/web/lib/catering.ts` / `apps/web/lib/contact.ts` (new): client submit helpers

Depends on: Group 10. Both copy `apps/web/lib/orders.ts`'s
`SubmitOrderResult`/`isFieldError`/`submitOrder` shape, swapping the
payload/response types and dropping the `items_unavailable` branch
(nothing here has availability to conflict over). Both payload types also
carry the honeypot field (Group 3/4/5) — always sent, always empty for a
real submission; Groups 13/14 are the only code that ever sets it to
anything.

**`apps/web/lib/catering.ts`:**

```ts
export interface CreateCateringEnquiryRequest {
  name: string;
  phone: string;
  email: string;
  event_type: string;
  event_date: string; // "YYYY-MM-DD"
  guest_count: number;
  location: string;
  services_required: string[];
  message: string | null;
  website: string; // honeypot — always "" for a real submission
}

export interface CateringEnquiryResponse {
  id: number;
  created_at: string;
}

export interface FieldError {
  field: string;
  message: string;
}

export type SubmitCateringEnquiryResult =
  | { ok: true; enquiry: CateringEnquiryResponse }
  | { ok: false; kind: "validation_error"; fields: FieldError[] }
  | { ok: false; kind: "unknown" };

function isFieldError(value: unknown): value is FieldError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.field === "string" && typeof v.message === "string";
}

export async function submitCateringEnquiry(
  payload: CreateCateringEnquiryRequest,
): Promise<SubmitCateringEnquiryResult> {
  const response = await fetch("/api/catering-enquiries", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  const body: unknown = await response.json().catch(() => null);

  if (response.ok) {
    return { ok: true, enquiry: body as CateringEnquiryResponse };
  }

  if (
    response.status === 400 &&
    typeof body === "object" &&
    body !== null &&
    Array.isArray((body as Record<string, unknown>).fields) &&
    (body as Record<string, unknown[]>).fields.every(isFieldError)
  ) {
    return { ok: false, kind: "validation_error", fields: (body as { fields: FieldError[] }).fields };
  }

  // Covers both a genuinely unexpected error and a 429 from Group 3's
  // rate limiter — the generic error Notice (Group 13) is the right UX
  // for a real user who hit a shared/misbehaving-client limit either way.
  return { ok: false, kind: "unknown" };
}
```

**`apps/web/lib/contact.ts`:** identical shape —
`CreateContactMessageRequest { name, email, phone, subject, message,
website }`, `ContactMessageResponse { id, created_at }`,
`submitContactMessage` posting to `/api/contact-messages`.

## Group 12 — `apps/web/lib/admin-enquiries.ts` / `apps/web/lib/admin-enquiries-api.ts` (new)

Depends on: Group 7. Mirrors `lib/admin-orders.ts` / `lib/admin-orders-api.ts`.

**`apps/web/lib/admin-enquiries.ts`:**

```ts
export const ENQUIRIES_PAGE_SIZE = 25;

export interface CateringEnquirySummary {
  id: number;
  name: string;
  event_type: string;
  event_date: string;
  guest_count: number;
  created_at: string;
}

export interface CateringEnquiryDetail extends CateringEnquirySummary {
  phone: string;
  email: string;
  location: string;
  services_required: string[];
  message: string | null;
}

export interface CateringEnquiryList {
  enquiries: CateringEnquirySummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface ContactMessageSummary {
  id: number;
  name: string;
  subject: string;
  created_at: string;
}

export interface ContactMessageDetail extends ContactMessageSummary {
  email: string;
  phone: string;
  message: string;
}

export interface ContactMessageList {
  messages: ContactMessageSummary[];
  total: number;
  limit: number;
  offset: number;
}
```

**`apps/web/lib/admin-enquiries-api.ts`:**

```ts
import { adminApiJson } from "@/lib/admin-api";
import {
  ENQUIRIES_PAGE_SIZE,
  type CateringEnquiryDetail,
  type CateringEnquiryList,
  type ContactMessageDetail,
  type ContactMessageList,
} from "@/lib/admin-enquiries";

function pageParams(page = 1) {
  return new URLSearchParams({
    limit: String(ENQUIRIES_PAGE_SIZE),
    offset: String((page - 1) * ENQUIRIES_PAGE_SIZE),
  });
}

export function getAdminCateringEnquiries(page = 1): Promise<CateringEnquiryList> {
  return adminApiJson<CateringEnquiryList>(`/api/admin/catering-enquiries?${pageParams(page)}`);
}

export function getAdminCateringEnquiry(id: number): Promise<CateringEnquiryDetail> {
  return adminApiJson<CateringEnquiryDetail>(`/api/admin/catering-enquiries/${id}`);
}

export function getAdminContactMessages(page = 1): Promise<ContactMessageList> {
  return adminApiJson<ContactMessageList>(`/api/admin/contact-messages?${pageParams(page)}`);
}

export function getAdminContactMessage(id: number): Promise<ContactMessageDetail> {
  return adminApiJson<ContactMessageDetail>(`/api/admin/contact-messages/${id}`);
}
```

## Group 13 — `apps/web/components/catering-enquiry-form.tsx` (new)

Depends on: Group 11. Same structure as
`apps/web/components/order-details-form.tsx`: a `validate()` function
returning a `FieldErrors` record, client-managed `values`/`errors` state,
`noValidate` on the `<form>`. New field types this form needs that
`OrderDetailsForm` didn't:

- Event type: a `<select>` populated from a local `EVENT_TYPES` array
  (mirrors `EVENT_TYPES` in `catering_enquiries.rs`, Group 4), each
  `<option>` label pulled from `t(\`eventType.${value}\`)`.
- Event date: `<input type="date" min={todayIsoDate()} />` — the
  browser-native date picker, `min` a client-side nudge only (the real
  enforcement is Group 4's server-side check; an attacker bypassing the
  `min` attribute still gets a `400`).
- Guest count: `<input type="number" min={1} />`.
- Services required: a `<fieldset>` of checkboxes (one per
  `SERVICES` value, same four as Group 4), collected into a `string[]`.
- **Honeypot** (tech-stack.md "Spam/Abuse Mitigation"): one extra text
  field, wired into the same `values` state as every other field (name
  `website`, always submitted, normally empty) but never meant to be seen
  or used by a person:

  ```tsx
  <div
    aria-hidden="true"
    style={{ position: "absolute", left: "-9999px", width: "1px", height: "1px", overflow: "hidden" }}
  >
    <label htmlFor="catering-website">Leave this field blank</label>
    <input
      id="catering-website"
      name="website"
      type="text"
      tabIndex={-1}
      autoComplete="off"
      value={values.website}
      onChange={(event) => setValues((current) => ({ ...current, website: event.target.value }))}
    />
  </div>
  ```

  Positioned off-screen (not `display:none`/`type="hidden"`, which some
  bots skip) and `aria-hidden` + `tabIndex={-1}` so it's excluded from the
  accessibility tree and tab order — a screen-reader user never encounters
  it, so its label text is deliberately plain hardcoded English, not a
  `next-intl` key (nothing to translate for an audience of none/bots).

Submission calls `submitCateringEnquiry` (Group 11) directly inside this
component (no separate "review" step, unlike the multi-step order flow —
this is a single-page enquiry form, same single-step shape the existing
disabled contact form already has) and renders a `Notice` success state
in place of the form on success, an inline `Notice` error on
`kind === "unknown"` (this now also covers a `429` from Group 3's rate
limiter — same generic-error copy, no special-cased "you're going too
fast" message), same visual pattern `OrderReview`'s `submitError` prop
already establishes elsewhere.

## Group 14 — `apps/web/components/contact-form.tsx` (new): wires up the existing stub

Depends on: Group 11. Extracts the `<form>` currently inlined in
`contact/page.tsx` (name, email, phone, subject select, message) into
its own client component, following the same `validate()`/submit/success
pattern as Group 13 (including its own copy of the same honeypot field,
`id="contact-website"`), calling `submitContactMessage`. The `Button
disabled` + `formComingSoon` `Notice` is removed — the button becomes a
real submit, disabled only while `submitting` (same `t("placingOrder")`-
style loading-label precedent `OrderReview`'s submit button already
uses). The `<option value="eventRental">` in the existing markup becomes
`<option value="event_rental">` (requirement.md Decision 4 fix) when this
form is extracted — there's no separate step for it, it's part of moving
this markup into its own component.

## Group 15 — `apps/web/components/contact-forms-tabs.tsx` (new): the tab/toggle

Depends on: Groups 13, 14. Plain client-side `useState<"general" |
"catering">`, seeded from an `initialTab` prop the page (Group 16) passes
in from a `?tab=` search param — switching tabs after that is still local
state, no navigation (requirement.md Decision 9).

```tsx
"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { ContactForm } from "@/components/contact-form";
import { CateringEnquiryForm } from "@/components/catering-enquiry-form";

type Tab = "general" | "catering";

export function ContactFormsTabs({ initialTab }: { initialTab: Tab }) {
  const t = useTranslations("ContactPage");
  const [tab, setTab] = useState<Tab>(initialTab);

  return (
    <div>
      <div role="tablist" aria-label={t("formTabsLabel")} className="mb-6 flex gap-2">
        {(["general", "catering"] as const).map((value) => (
          <button
            key={value}
            type="button"
            role="tab"
            aria-selected={tab === value}
            onClick={() => setTab(value)}
            className={`rounded-full px-4 py-2 text-sm font-semibold transition-colors duration-150 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              tab === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value === "general" ? t("tabGeneral") : t("tabCatering")}
          </button>
        ))}
      </div>

      {tab === "general" ? <ContactForm /> : <CateringEnquiryForm />}
    </div>
  );
}
```

`initialTab` is read once by `useState`'s initializer argument — later
changes to the URL (there aren't any; tabbing doesn't push a new URL)
don't re-seed it, same one-shot-seed pattern as any other
`useState(propValue)`.

## Group 16 — `apps/web/app/[locale]/contact/page.tsx`: swap the static form, read `?tab=`

Depends on: Group 15. Replace the inlined `<form>` block (and its
`Button`/`Notice` imports, now unused on this page) with
`<ContactFormsTabs initialTab={initialTab} />`, wrapped in the same
`<Reveal><section className="mx-auto max-w-[640px]">` container the
static form already used. No other change to this page — the `SplitRow`
info list above it is untouched.

The page becomes an `async` function accepting `searchParams`, same
`Promise<...>` shape the admin enquiries list page (Group 20) already
uses, to seed the deep link (requirement.md Decision 9 / "Risks resolved
in this revisit"):

```tsx
export default async function ContactPage({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  const initialTab = tab === "catering" ? "catering" : "general";

  // ...unchanged t()/PageHero/SplitRow above...

  return (
    // ...
    <ContactFormsTabs initialTab={initialTab} />
    // ...
  );
}
```

## Group 17 — `apps/web/app/[locale]/services/page.tsx`: CTAs deep-link to the catering tab

Depends on: Group 16 (the `?tab=catering` param must actually do something
before CTAs are pointed at it). New for this revisit — requirement.md
Decision 9 / "Risks resolved in this revisit". Both existing
`ButtonLink href="/contact"` CTAs (Catering's "Request Catering Quote" and
Event Rentals' "Request a Quote" — `event_rental` is one of the catering
form's `services_required` options, so both are catering-form topics)
become:

```tsx
<ButtonLink href="/contact?tab=catering" className="mt-4">
```

No other change to this page.

## Group 18 — `apps/web/messages/{en,nl}.json`: new keys

Depends on: nothing (can be done any time before Groups 13–16 are
exercised live). Added to the existing `ContactPage` block; remove
`formComingSoon` (requirement.md Decision 12). `eventType.event_rental`'s
key name tracks the Decision 4 fix — the label itself ("Event Rental") is
unchanged, only the JSON key spelling changes from what an `eventRental`
version would have used.

```json
"ContactPage": {
  "...": "...unchanged keys above...",
  "formTabsLabel": "Choose enquiry type",
  "tabGeneral": "General Enquiry",
  "tabCatering": "Catering & Event Enquiry",
  "cateringNameLabel": "Full Name",
  "cateringPhoneLabel": "Phone Number",
  "cateringEmailLabel": "Email",
  "cateringEventTypeLabel": "Event Type",
  "eventType": {
    "wedding": "Wedding",
    "birthday": "Birthday",
    "corporate": "Corporate",
    "family_gathering": "Family Gathering",
    "outdoor_event": "Outdoor Event",
    "other": "Other"
  },
  "cateringEventDateLabel": "Event Date",
  "cateringGuestCountLabel": "Number of Guests",
  "cateringLocationLabel": "Location",
  "cateringServicesLabel": "Services Required",
  "service": {
    "catering": "Catering",
    "meal_delivery": "Meal Delivery",
    "event_rental": "Event Rental",
    "other": "Other"
  },
  "cateringMessageLabel": "Message (optional)",
  "submitEnquiry": "Request a Quote",
  "submittingEnquiry": "Submitting…",
  "cateringSuccessHeading": "Thank you!",
  "cateringSuccessBody": "We've received your enquiry and will be in touch to discuss your event.",
  "contactSuccessHeading": "Message Sent",
  "contactSuccessBody": "Thanks for reaching out — we'll get back to you soon.",
  "submitMessage": "Send Message",
  "submittingMessage": "Sending…",
  "submitErrorGeneric": "Something went wrong submitting this — please try again, or reach us on WhatsApp.",
  "errors": {
    "required": "{field} is required.",
    "invalid": "Please enter a valid {field}.",
    "tooShort": "{field} is too short.",
    "tooLong": "{field} is too long.",
    "mustBeFuture": "{field} must be a future date.",
    "mustBePositive": "{field} must be greater than zero."
  }
}
```

(`"formComingSoon"` key removed from both files.) Mirror every key into
`apps/web/messages/nl.json` with real Dutch copy, not a placeholder —
same bar Phase 3 set for every other page's translations. The honeypot
field's hardcoded "Leave this field blank" label (Groups 13/14) is
deliberately **not** mirrored here — see those groups' note on why.

## Group 19 — `apps/web/components/admin-nav.tsx`: add "Enquiries"

Depends on: nothing.

```tsx
const NAV_ITEMS = [
  { label: "Dashboard", href: "/admin", enabled: true },
  { label: "Orders", href: "/admin/orders", enabled: true },
  { label: "Menu", href: "/admin/menu", enabled: true },
  { label: "Categories", href: "/admin/categories", enabled: true },
  { label: "Enquiries", href: "/admin/enquiries", enabled: true },
  { label: "Settings", href: "/admin/settings", enabled: false },
] as const;
```

## Group 20 — `apps/web/app/admin/(protected)/enquiries/page.tsx` (new): list, both resources behind a `?type=` tab

Depends on: Groups 12, 19. Same list-page shape as
`apps/web/app/admin/(protected)/orders/page.tsx`, but the "filter pills"
become a two-value type switch (`catering` default, `contact`) instead of
a status filter, and pagination is a plain `?page=` param scoped to
whichever `type` is active.

```tsx
import Link from "next/link";
import type { Metadata } from "next";
import { ENQUIRIES_PAGE_SIZE } from "@/lib/admin-enquiries";
import { getAdminCateringEnquiries, getAdminContactMessages } from "@/lib/admin-enquiries-api";
import { formatDateTime } from "@/lib/format";

export const metadata: Metadata = { title: "Enquiries | Zain's Admin" };

export default async function AdminEnquiriesPage({
  searchParams,
}: {
  searchParams: Promise<{ type?: string; page?: string }>;
}) {
  const { type: rawType, page: rawPage } = await searchParams;
  const type = rawType === "contact" ? "contact" : "catering";
  const parsedPage = Number.parseInt(rawPage ?? "1", 10);
  const page = Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1;

  const [catering, contact] = await Promise.all([
    type === "catering" ? getAdminCateringEnquiries(page) : null,
    type === "contact" ? getAdminContactMessages(page) : null,
  ]);

  const total = catering?.total ?? contact?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / ENQUIRIES_PAGE_SIZE));

  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Enquiries</h1>

      <div className="flex gap-2">
        {(["catering", "contact"] as const).map((value) => (
          <Link
            key={value}
            href={`/admin/enquiries?type=${value}`}
            aria-current={type === value ? "page" : undefined}
            className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider ${
              type === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value === "catering" ? "Catering" : "Contact"}
          </Link>
        ))}
      </div>

      {type === "catering" ? (
        catering!.enquiries.length === 0 ? (
          <p className="m-0 text-text-muted">No catering enquiries yet.</p>
        ) : (
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
                <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Event</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Date</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Guests</th>
                <th scope="col" className="py-3 font-semibold">Submitted</th>
              </tr>
            </thead>
            <tbody>
              {catering!.enquiries.map((e) => (
                <tr key={e.id} className="border-b border-text/10 hover:bg-background-soft">
                  <td className="py-3 pr-4">
                    <Link href={`/admin/enquiries/catering/${e.id}`} className="font-semibold text-primary underline">
                      {e.name}
                    </Link>
                  </td>
                  <td className="py-3 pr-4">{e.event_type}</td>
                  <td className="py-3 pr-4">{e.event_date}</td>
                  <td className="py-3 pr-4">{e.guest_count}</td>
                  <td className="py-3">{formatDateTime(e.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )
      ) : contact!.messages.length === 0 ? (
        <p className="m-0 text-text-muted">No contact messages yet.</p>
      ) : (
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Subject</th>
              <th scope="col" className="py-3 font-semibold">Submitted</th>
            </tr>
          </thead>
          <tbody>
            {contact!.messages.map((m) => (
              <tr key={m.id} className="border-b border-text/10 hover:bg-background-soft">
                <td className="py-3 pr-4">
                  <Link href={`/admin/enquiries/contact/${m.id}`} className="font-semibold text-primary underline">
                    {m.name}
                  </Link>
                </td>
                <td className="py-3 pr-4">{m.subject}</td>
                <td className="py-3">{formatDateTime(m.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {pageCount > 1 && (
        <div className="flex gap-2">
          {Array.from({ length: pageCount }, (_, i) => i + 1).map((p) => (
            <Link
              key={p}
              href={`/admin/enquiries?type=${type}&page=${p}`}
              aria-current={page === p ? "page" : undefined}
              className={`rounded-full px-3 py-1 text-xs font-semibold ${
                page === p ? "bg-primary text-white" : "bg-card text-text"
              }`}
            >
              {p}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
```

## Group 21 — `apps/web/app/admin/(protected)/enquiries/catering/[id]/page.tsx` (new): detail

Depends on: Group 12. Same `Field` helper shape as
`apps/web/app/admin/(protected)/orders/[id]/page.tsx` — no status badge,
no status-change control (read-only, requirement.md Decision 8), just
every field laid out plus a `services_required.join(", ")` line and
`message?.trim() || "—"`.

## Group 22 — `apps/web/app/admin/(protected)/enquiries/contact/[id]/page.tsx` (new): detail

Depends on: Group 12. Same shape as Group 21, scoped to
`ContactMessageDetail`'s five fields.
