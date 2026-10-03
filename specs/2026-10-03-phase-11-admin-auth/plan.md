# Phase 11 — Admin Auth: Implementation Plan

Groups 1–10 touch only `apps/api`. Groups 11–19 touch only `apps/web`.
Groups 20–21 describe (do not perform) the `docker-compose*.yml`/
`Dockerfile`/`docs/deployment.md` edits implementation will need — this
plan-writing pass never edits files outside `specs/`.

## Group 0 — Branch

Branched `2026-10-03-phase-11-admin-auth` off `master` (Phase 10 already
merged).

## Group 1 — `apps/api/Cargo.toml`: new dependencies

```toml
argon2 = "0.6"
tower-sessions = "0.15"
tower-sessions-sqlx-store = { version = "0.15", features = ["postgres"] }

[dev-dependencies]
rpassword = "7"
```

Notes (checked against crates.io at planning time, re-verify at
implementation time):
- `argon2 = "0.6"` defaults already include `password-hash` and
  `getrandom` — `argon2::password_hash::{PasswordHash, PasswordHasher,
  PasswordVerifier, SaltString, rand_core::OsRng}` are usable with no
  extra feature flags.
- `tower-sessions-sqlx-store = "0.15"` pins `sqlx ^0.8.0` internally —
  matches this repo's existing `sqlx = "0.8"`. Its `postgres` feature
  flag just forwards to `sqlx/postgres`.
- `rpassword` is a **dev-dependency only if `create_admin` lives under
  `[[bin]]`/`src/bin/` as part of the same package** — in practice `cargo`
  still needs it as a normal (non-dev) dependency since `create_admin` is
  a release binary users run in production, not a test. Put it under
  `[dependencies]`, not `[dev-dependencies]` — correcting the sketch
  above; implementation should double check this lands in the right
  section.

## Group 2 — `apps/api/src/error.rs`: new variants

Add to the existing enum (Database/Validation/ItemsUnavailable unchanged):

```rust
pub enum AppError {
    Database(sqlx::Error),
    Validation(Vec<FieldError>),
    ItemsUnavailable(Vec<UnavailableItem>),
    Unauthorized,
    InvalidCredentials,
    Session(tower_sessions::session::Error),
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(err: tower_sessions::session::Error) -> Self {
        Self::Session(err)
    }
}
```

New `IntoResponse` match arms:

```rust
AppError::Unauthorized => {
    tracing::warn!("rejected request with no valid admin session");
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "unauthorized" }))).into_response()
}
AppError::InvalidCredentials => {
    tracing::warn!("admin login failed");
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid_credentials" }))).into_response()
}
AppError::Session(err) => {
    tracing::error!(error = ?err, "session store error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_server_error" })),
    )
        .into_response()
}
```

`Unauthorized` (missing/invalid session — used by `require_admin` and
`me`) is kept distinct from `InvalidCredentials` (bad login attempt) even
though both return `401` — the frontend can tell "please log in" apart
from "that email/password was wrong" by the `error` code without the
status code alone conflating them.

## Group 3 — `apps/api/src/lib.rs`: wire the session layer

Depends on: Groups 1, 2.

```rust
pub mod error;
pub mod routes;

use axum::Router;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, SessionManagerLayer,
};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::Level;

pub async fn build_app(pool: PgPool, cookie_secure: bool) -> Result<Router, sqlx::Error> {
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cookie_secure)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    Ok(Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health))
        .layer(session_layer)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .latency_unit(LatencyUnit::Millis),
                        ),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
        .with_state(pool))
}
```

`build_app` becomes `async` and fallible — see requirement.md Decision 3
for why this is preferred over an `.expect()`/panic inside it.

## Group 4 — `apps/api/src/main.rs`: await the now-fallible `build_app`

Depends on: Group 3.

```rust
let cookie_secure = std::env::var("COOKIE_SECURE")
    .map(|v| v == "true")
    .unwrap_or(false);

let app = match api::build_app(pool, cookie_secure).await {
    Ok(app) => app,
    Err(error) => {
        tracing::error!(%error, "failed to build application");
        std::process::exit(1);
    }
};
```

Same controlled-exit style every other startup failure in this file
already uses (`DATABASE_URL` missing, pool connect failure, migration
failure, listener bind failure) — no new panic path introduced.

## Group 5 — `apps/api/src/routes/admin/mod.rs` (new)

Depends on: Groups 2, 3.

```rust
pub mod auth;
pub mod middleware;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn admin_router() -> Router<PgPool> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route_layer(from_fn(middleware::require_admin));

    Router::new()
        .route("/login", post(auth::login))
        .merge(protected)
}
```

`route_layer` applies `require_admin` only to the routes already added to
`protected` before `.merge()` — `/login` stays unauthenticated.

## Group 6 — `apps/api/src/routes/admin/auth.rs` (new)

Depends on: Group 5.

```rust
use crate::error::AppError;
use crate::routes::admin::SESSION_USER_ID_KEY;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_sessions::Session;

// A real argon2id PHC-format hash generated once against an arbitrary
// fixed password (NOT a real account's password) — verified against on an
// unknown email so "no such user" and "wrong password" take a similar
// amount of time (Decision 6). Generate with, e.g.:
//   Argon2::default().hash_password(b"placeholder", &SaltString::generate(&mut OsRng))
// and hardcode the resulting string here; do not hand-write this constant.
const DUMMY_PASSWORD_HASH: &str = "<generate at implementation time — see above>";

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: String,
}

struct UserRow {
    id: i64,
    name: String,
    email: String,
    password_hash: String,
    role: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    session: Session,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AdminUserResponse>, AppError> {
    if payload.email.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::InvalidCredentials);
    }

    let user = sqlx::query_as!(
        UserRow,
        "SELECT id, name, email, password_hash, role FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&pool)
    .await?;

    let hash_to_check = user
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_PASSWORD_HASH);

    let parsed_hash =
        PasswordHash::new(hash_to_check).map_err(|_| AppError::InvalidCredentials)?;
    let verified = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok();

    let Some(user) = user.filter(|_| verified) else {
        return Err(AppError::InvalidCredentials);
    };

    session.cycle_id().await?; // regenerate session id on login — fixation hardening
    session.insert(SESSION_USER_ID_KEY, user.id).await?;

    Ok(Json(AdminUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
    }))
}

pub async fn me(
    State(pool): State<PgPool>,
    session: Session,
) -> Result<Json<AdminUserResponse>, AppError> {
    let user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    let user_id = user_id.ok_or(AppError::Unauthorized)?;

    let user = sqlx::query_as!(
        UserRow,
        "SELECT id, name, email, password_hash, role FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(Json(AdminUserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
    }))
}

pub async fn logout(session: Session) -> Result<StatusCode, AppError> {
    session.delete().await?;
    Ok(StatusCode::NO_CONTENT)
}
```

Note: `user.filter(|_| verified)` is a terse way to say "only treat this as
a real match if the password also verified" — if the exact-struct-move
semantics of `Option<UserRow>::filter` are awkward in practice (e.g. `UserRow`
needing `Clone`), an equivalent `if !verified { return Err(...) }` followed
by `let Some(user) = user else { return Err(...) }` is functionally
identical — implementation picks whichever reads cleaner.

## Group 7 — `apps/api/src/routes/admin/middleware.rs` (new)

Depends on: Group 5.

```rust
use crate::error::AppError;
use crate::routes::admin::SESSION_USER_ID_KEY;
use axum::{extract::Request, middleware::Next, response::Response};
use tower_sessions::Session;

pub async fn require_admin(
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    if user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(request).await)
}
```

## Group 8 — `apps/api/src/routes/mod.rs`: wire `/admin`

Depends on: Groups 5, 6, 7.

```rust
pub mod admin;
pub mod categories;
pub mod health;
pub mod menu_items;
pub mod orders;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn api_router() -> Router<PgPool> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
        .nest("/admin", admin::admin_router())
}
```

## Group 9 — `apps/api/src/bin/create_admin.rs` (new)

Depends on: Group 1 (needs `argon2`, `rpassword`, and the existing `sqlx`
dependency). This is a second binary target — Cargo auto-discovers any
`src/bin/*.rs` file as its own executable (`cargo run --bin create_admin`)
alongside the package's main `api` binary, no `Cargo.toml` `[[bin]]` entry
needed.

```rust
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use sqlx::postgres::PgPoolOptions;

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let Some(email) = arg_value("--email") else {
        eprintln!("usage: create_admin --email <email> [--name <name>]");
        std::process::exit(1);
    };
    let name = arg_value("--name").unwrap_or_else(|| "Admin".to_string());

    let password =
        rpassword::prompt_password("Password: ").expect("failed to read password");
    let confirm =
        rpassword::prompt_password("Confirm password: ").expect("failed to read password");
    if password != confirm {
        eprintln!("passwords did not match");
        std::process::exit(1);
    }
    if password.is_empty() {
        eprintln!("password must not be empty");
        std::process::exit(1);
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("failed to hash password")
        .to_string();

    sqlx::query!(
        r#"
        INSERT INTO users (name, email, password_hash, role)
        VALUES ($1, $2, $3, 'admin')
        ON CONFLICT (email) DO UPDATE
            SET password_hash = EXCLUDED.password_hash,
                name = EXCLUDED.name,
                updated_at = now()
        "#,
        name,
        email,
        password_hash,
    )
    .execute(&pool)
    .await
    .expect("failed to upsert admin user");

    println!("admin user '{email}' created/updated");
}
```

A standalone CLI tool panicking with `.expect()` on failure is fine here —
unlike `main.rs`/`lib.rs`, this isn't a long-running server the Phase 8
panic hook/controlled-exit convention was written for; it's a short-lived
operator command where a loud panic message is the right failure mode.

Usage:
- Local dev: `cd apps/api && DATABASE_URL=... cargo run --bin create_admin
  -- --email owner@zainstreat.com --name "Zain"`, prompted for the password
  twice.
- Production: see Group 21 (`docs/deployment.md` addition) — run inside the
  already-running `api` container via `docker compose exec`, not built/run
  from a developer machine against the prod database.

## Group 10 — Tests

Depends on: Groups 1–9.

### 10.1 — `apps/api/tests/common.rs` (new)

Extracts the `get`/`post` `oneshot` helpers `menu_browsing.rs` and
`order_submission.rs` each currently duplicate, and adds cookie-aware
variants this phase's login→me→logout flow needs:

```rust
use api::build_app;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn send(pool: PgPool, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let app = build_app(pool, false)
        .await
        .expect("failed to build app in test");
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, body)
}

pub async fn get(pool: PgPool, uri: &str) -> (StatusCode, Value) {
    let (status, _headers, body) =
        send(pool, Request::builder().uri(uri).body(Body::empty()).unwrap()).await;
    (status, body)
}

pub async fn post(pool: PgPool, uri: &str, body: Value) -> (StatusCode, Value) {
    let (status, _headers, resp_body) = send(
        pool,
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;
    (status, resp_body)
}

/// For auth flows: returns the response body plus any `Set-Cookie` value so
/// a follow-up request can carry the session forward.
pub async fn post_with_cookie(
    pool: PgPool,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, headers, resp_body) =
        send(pool, builder.body(Body::from(body.to_string())).unwrap()).await;
    let set_cookie = headers
        .get("set-cookie")
        .map(|v| v.to_str().unwrap().to_string());
    (status, resp_body, set_cookie)
}

pub async fn get_with_cookie(pool: PgPool, uri: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, body) = send(pool, builder.body(Body::empty()).unwrap()).await;
    (status, body)
}

/// A `Set-Cookie` value looks like `id=...; HttpOnly; SameSite=Lax; ...` —
/// only the `id=...` part before the first `;` is sent back as `Cookie`.
pub fn session_cookie(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap()
}
```

Refactor `menu_browsing.rs` and `order_submission.rs` to `mod common;` and
call `common::get(...)`/`common::post(...)` instead of their own
now-deleted local copies. No test *behavior* changes, pure de-duplication.

### 10.2 — `apps/api/tests/admin_auth.rs` (new)

```rust
mod common;

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use axum::http::StatusCode;
use common::{get_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

async fn insert_admin(pool: &PgPool, email: &str, password: &str) {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query!(
        "INSERT INTO users (name, email, password_hash, role) VALUES ($1, $2, $3, 'admin')",
        "Test Admin",
        email,
        hash,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn login_succeeds_and_me_reflects_session(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct horse battery staple").await;

    let (status, body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email"], "owner@example.com");
    let cookie = session_cookie(&set_cookie.expect("login should set a session cookie")).to_string();

    let (me_status, me_body) = get_with_cookie(pool, "/api/admin/me", Some(&cookie)).await;
    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me_body["email"], "owner@example.com");
}

#[sqlx::test]
async fn me_without_session_is_unauthorized(pool: PgPool) {
    let (status, body) = get_with_cookie(pool, "/api/admin/me", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn login_rejects_wrong_password(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct-password").await;

    let (status, body, set_cookie) = post_with_cookie(
        pool,
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "wrong-password" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
    assert!(set_cookie.is_none());
}

#[sqlx::test]
async fn login_rejects_unknown_email(pool: PgPool) {
    let (status, body, set_cookie) = post_with_cookie(
        pool,
        "/api/admin/login",
        json!({ "email": "nobody@example.com", "password": "anything" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
    assert!(set_cookie.is_none());
}

#[sqlx::test]
async fn logout_invalidates_session(pool: PgPool) {
    insert_admin(&pool, "owner@example.com", "correct horse battery staple").await;

    let (_status, _body, set_cookie) = post_with_cookie(
        pool.clone(),
        "/api/admin/login",
        json!({ "email": "owner@example.com", "password": "correct horse battery staple" }),
        None,
    )
    .await;
    let cookie = session_cookie(&set_cookie.unwrap()).to_string();

    let (logout_status, _body, _set_cookie) =
        post_with_cookie(pool.clone(), "/api/admin/logout", json!({}), Some(&cookie)).await;
    assert_eq!(logout_status, StatusCode::NO_CONTENT);

    let (me_status, _me_body) = get_with_cookie(pool, "/api/admin/me", Some(&cookie)).await;
    assert_eq!(me_status, StatusCode::UNAUTHORIZED);
}
```

### 10.3 — Offline query cache

With the dev stack up and `DATABASE_URL` exported, from `apps/api`:
```bash
cargo sqlx prepare
```
Commit the updated `apps/api/.sqlx/` directory — it now also covers
`create_admin.rs`'s insert and `admin/auth.rs`'s two new queries.

## Group 11 — `apps/web/proxy.ts`: exclude `/admin`

```ts
export const config = {
  matcher: ["/((?!api|admin|_next|_vercel|.*\\..*).*)"],
};
```

## Group 12 — `apps/web/lib/admin-auth.ts` (new)

Depends on: nothing frontend-side; depends on Group 6 existing for
`/api/admin/me` to be meaningful.

```ts
import { cookies } from "next/headers";

const API_BASE_URL = process.env.API_BASE_URL;

export interface AdminUser {
  id: number;
  name: string;
  email: string;
  role: string;
}

export async function getCurrentAdmin(): Promise<AdminUser | null> {
  if (!API_BASE_URL) return null;

  const cookieHeader = (await cookies()).toString();
  if (!cookieHeader) return null;

  const response = await fetch(`${API_BASE_URL}/api/admin/me`, {
    headers: { Cookie: cookieHeader },
    cache: "no-store",
  });

  if (!response.ok) return null;
  return (await response.json()) as AdminUser;
}
```

Same server-to-server pattern `lib/api.ts` already uses — `API_BASE_URL`
is container-internal, never exposed to the browser.

## Group 13 — Route Handler proxies: `apps/web/app/api/admin/{login,logout}/route.ts` (new)

Depends on: Group 6. Unlike Phase 10's `app/api/orders/route.ts`, both of
these must forward cookies in both directions.

**`apps/web/app/api/admin/login/route.ts`:**
```ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();
  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });

  const responseBody = await apiResponse.text();
  const response = new NextResponse(responseBody, {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });

  for (const cookie of apiResponse.headers.getSetCookie()) {
    response.headers.append("Set-Cookie", cookie);
  }

  return response;
}
```

**`apps/web/app/api/admin/logout/route.ts`:**
```ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/logout`, {
    method: "POST",
    headers: { Cookie: request.headers.get("cookie") ?? "" },
  });

  const response = new NextResponse(null, { status: apiResponse.status });

  for (const cookie of apiResponse.headers.getSetCookie()) {
    response.headers.append("Set-Cookie", cookie);
  }

  return response;
}
```

`Headers.getSetCookie()` is a standard `fetch`/`undici` API available in
the Node runtime Next.js 16 Route Handlers run on by default — confirm
this at implementation time if the project ever pins a `runtime = "edge"`
on either route (it currently doesn't anywhere in the codebase).

`GET /api/admin/me` needs no browser-facing proxy route — only
`lib/admin-auth.ts` (Group 12) calls it, server-to-server.

## Group 14 — `apps/web/app/admin/layout.tsx` (new)

Depends on: nothing. A second, independent root layout — see requirement.md
Decision 8 for why this is needed (no shared `app/layout.tsx` exists above
`app/[locale]/` today).

```tsx
import type { Metadata } from "next";
import "../globals.css";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
```

## Group 15 — `apps/web/components/admin-login-form.tsx` (new)

Depends on: Group 13. `"use client"`.

```tsx
"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";

export function AdminLoginForm() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);

    if (!email.trim() || !password) {
      setError("Email and password are required.");
      return;
    }

    setSubmitting(true);
    const response = await fetch("/api/admin/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    setSubmitting(false);

    if (response.ok) {
      router.push("/admin");
      router.refresh();
      return;
    }

    setError("Invalid email or password.");
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="mx-auto flex max-w-[400px] flex-col gap-5 py-16">
      <h1 className="m-0 text-2xl font-semibold">Admin Login</h1>
      <div>
        <label htmlFor="admin-email" className="mb-1.5 block text-sm font-semibold">Email</label>
        <input
          id="admin-email"
          type="email"
          className="field"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="username"
        />
      </div>
      <div>
        <label htmlFor="admin-password" className="mb-1.5 block text-sm font-semibold">Password</label>
        <input
          id="admin-password"
          type="password"
          className="field"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
        />
      </div>
      {error && <Notice>{error}</Notice>}
      <Button type="submit" disabled={submitting}>
        {submitting ? "Logging in…" : "Log In"}
      </Button>
    </form>
  );
}
```

Plain English copy, no `next-intl` — admin stays English-only
(requirement.md Decision 8). `Button` (not `ButtonLink`) is reused as-is
since it's a plain `<button>` with no dependency on `@/i18n/navigation`'s
locale-aware `Link` — only `ButtonLink` carries that dependency, and
nothing in admin should use it.

## Group 16 — `apps/web/app/admin/login/page.tsx` (new)

Depends on: Group 15.

```tsx
import type { Metadata } from "next";
import { AdminLoginForm } from "@/components/admin-login-form";

export const metadata: Metadata = {
  title: "Admin Login | Zain's Treat n More",
};

export default function AdminLoginPage() {
  return (
    <main className="container">
      <AdminLoginForm />
    </main>
  );
}
```

## Group 17 — `apps/web/components/admin-logout-button.tsx` (new)

Depends on: Group 13. `"use client"`.

```tsx
"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";

export function AdminLogoutButton() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);

  async function handleLogout() {
    setLoading(true);
    await fetch("/api/admin/logout", { method: "POST" });
    router.push("/admin/login");
    router.refresh();
  }

  return (
    <Button variant="secondary" onClick={handleLogout} disabled={loading}>
      {loading ? "Logging out…" : "Log Out"}
    </Button>
  );
}
```

## Group 18 — Protected admin area: `apps/web/app/admin/(protected)/{layout,page}.tsx` (new)

Depends on: Groups 12, 17.

**`apps/web/app/admin/(protected)/layout.tsx`:**
```tsx
import { redirect } from "next/navigation";
import { getCurrentAdmin } from "@/lib/admin-auth";

export default async function ProtectedAdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const admin = await getCurrentAdmin();
  if (!admin) {
    redirect("/admin/login");
  }
  return <>{children}</>;
}
```

**`apps/web/app/admin/(protected)/page.tsx`** — deliberately bare; Phase 12
replaces this with the real dashboard shell:
```tsx
import type { Metadata } from "next";
import { getCurrentAdmin } from "@/lib/admin-auth";
import { AdminLogoutButton } from "@/components/admin-logout-button";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default async function AdminHomePage() {
  const admin = await getCurrentAdmin();

  return (
    <main className="container flex flex-col items-start gap-4 py-16">
      <h1 className="m-0 text-2xl font-semibold">Welcome, {admin?.name}</h1>
      <p className="m-0 text-text-muted">Logged in as {admin?.email}.</p>
      <AdminLogoutButton />
    </main>
  );
}
```

The `(protected)` route group adds no URL segment — this page is still
reachable at exactly `/admin`, and `/admin/login` (Group 16) sits outside
the group, unauthenticated.

## Group 19 — No translation files touched

Admin stays English-only (requirement.md Decision 8) — no new keys in
`apps/web/messages/{en,nl}.json` for this phase.

## Group 20 — `docker-compose.yml` / `docker-compose.prod.yml`: `COOKIE_SECURE`

Describes the edit for whoever implements this phase (not performed by
this plan-writing pass — `docker-compose*.yml` is outside `specs/`):

- `docker-compose.yml`'s `api` service `environment:` block gains
  `COOKIE_SECURE: ${COOKIE_SECURE:-false}`, alongside the existing `PORT`/
  `LOG_FORMAT` entries.
- `docker-compose.prod.yml`'s `api` service `environment:` block gains
  `COOKIE_SECURE: ${COOKIE_SECURE:-true}`.

Same per-file-default convention already used for `LOG_FORMAT` (`pretty`
dev / `json` prod).

## Group 21 — `apps/api/Dockerfile` + `docs/deployment.md`: ship and document `create_admin`

Also described, not performed, here:

- `apps/api/Dockerfile`'s final stage gains a second `COPY --from=build`
  line so `create_admin` ships in the same image as `api`:
  ```dockerfile
  COPY --from=build /app/target/release/create_admin /usr/local/bin/create_admin
  ```
- `docs/deployment.md` gains a new "Creating the first admin account"
  section: SSH in as the **admin** user (not `deploy`), then
  ```bash
  docker compose -f docker-compose.yml -f docker-compose.prod.yml \
    exec api create_admin --email owner@zainstreat.com --name "Zain"
  ```
  prompted interactively for the password twice. Explicitly note this is a
  manual, one-time-per-account step, never run through `deploy.yml`/CI
  (consistent with the `deploy` user's sudo-less, docker-group-only scope
  from Phase 2).

## Verification

See `validation.md`.
