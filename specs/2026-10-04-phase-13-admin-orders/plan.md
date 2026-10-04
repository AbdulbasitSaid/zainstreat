# Phase 13 — Admin: Orders: Implementation Plan

Groups 1–5 touch only `apps/api`. Groups 6–17 touch only `apps/web`. No
`docker-compose*.yml`, `Dockerfile`, `Caddyfile`, `.github/**` or
`docs/**` changes are needed this phase — no new env var, no new binary,
no new service, no new operational step. No database migration either
(requirement.md Decision 15).

## Group 0 — Branch

Branched `2026-10-04-phase-13-admin-orders` off `master` (Phase 12 already
merged).

## Group 1 — `apps/api/src/routes/admin/orders.rs` (new)

No new Cargo dependencies — `axum`, `chrono`, `rust_decimal`, `serde`,
`sqlx` and `tracing` are all already in `apps/api/Cargo.toml`.

```rust
use crate::error::{AppError, FieldError};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

/// Mirrors the CHECK constraint on `orders.status` in
/// `migrations/20261002162443_initial_schema.up.sql`. Adding a status means
/// editing both — see requirement.md open risk 7.
pub const ORDER_STATUSES: [&str; 6] = [
    "new",
    "confirmed",
    "preparing",
    "ready",
    "completed",
    "cancelled",
];

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;
const RECENT_ORDER_COUNT: i64 = 5;

fn is_valid_status(value: &str) -> bool {
    ORDER_STATUSES.contains(&value)
}

#[derive(Debug, Serialize)]
pub struct OrderSummaryResponse {
    pub id: i64,
    pub customer_name: String,
    pub delivery_type: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderSummaryResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_orders(
    State(pool): State<PgPool>,
    Query(params): Query<ListOrdersQuery>,
) -> Result<Json<OrderListResponse>, AppError> {
    let mut errors = Vec::new();

    if let Some(status) = &params.status
        && !is_valid_status(status)
    {
        errors.push(FieldError { field: "status".into(), message: "invalid".into() });
    }

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        errors.push(FieldError { field: "limit".into(), message: "out_of_range".into() });
    }

    let offset = params.offset.unwrap_or(0);
    if offset < 0 {
        errors.push(FieldError { field: "offset".into(), message: "out_of_range".into() });
    }

    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM orders WHERE ($1::TEXT IS NULL OR status = $1)"#,
        params.status.as_deref()
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let orders = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>"
        FROM orders
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY created_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
        params.status.as_deref(),
        limit,
        offset
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(OrderListResponse { orders, total, limit, offset }))
}
```

`ORDER BY created_at DESC, id DESC` — `id DESC` is the tie-breaker, without
which two orders placed in the same transaction could swap places between
page loads and make pagination non-deterministic.

`COUNT(*)` comes back as `Option<i64>` from `query_scalar!` (sqlx can't
prove a bare aggregate is non-null), hence the `.unwrap_or(0)`.

Detail endpoint:

```rust
#[derive(Debug, Serialize)]
pub struct AdminOrderItemResponse {
    pub id: i64,
    pub item_name: String,
    pub option_label: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    pub quantity: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AdminOrderResponse {
    pub id: i64,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: String,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub delivery_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<AdminOrderItemResponse>,
}

pub async fn get_order(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminOrderResponse>, AppError> {
    // `fetch_one` yields sqlx::Error::RowNotFound for an unknown id, which
    // error.rs already maps to 404 {"error":"not_found"} — no new variant.
    let order = sqlx::query!(
        r#"
        SELECT id, customer_name, customer_email, customer_phone, delivery_type,
               delivery_address, notes, subtotal, delivery_fee, total, status,
               created_at as "created_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>"
        FROM orders
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&pool)
    .await?;

    let items = sqlx::query_as!(
        AdminOrderItemResponse,
        r#"
        SELECT id, item_name, option_label, unit_price, quantity, subtotal
        FROM order_items
        WHERE order_id = $1
        ORDER BY id
        "#,
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(AdminOrderResponse {
        id: order.id,
        customer_name: order.customer_name,
        customer_email: order.customer_email,
        customer_phone: order.customer_phone,
        delivery_type: order.delivery_type,
        delivery_address: order.delivery_address,
        notes: order.notes,
        subtotal: order.subtotal,
        delivery_fee: order.delivery_fee,
        total: order.total,
        status: order.status,
        created_at: order.created_at,
        updated_at: order.updated_at,
        items,
    }))
}
```

Status update:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub status: String,
}

pub async fn update_order_status(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateOrderStatusRequest>,
) -> Result<Json<OrderSummaryResponse>, AppError> {
    if !is_valid_status(&payload.status) {
        return Err(AppError::Validation(vec![FieldError {
            field: "status".into(),
            message: "invalid".into(),
        }]));
    }

    // `updated_at = now()` is explicit on purpose: the column's DEFAULT only
    // fires on INSERT and there is no BEFORE UPDATE trigger anywhere in
    // migrations/ (requirement.md Decision 9).
    let order = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        UPDATE orders
        SET status = $2, updated_at = now()
        WHERE id = $1
        RETURNING id, customer_name, delivery_type, total, status,
                  created_at as "created_at: DateTime<Utc>"
        "#,
        id,
        payload.status
    )
    .fetch_one(&pool)
    .await?;

    tracing::info!(order_id = id, status = %order.status, "admin updated order status");

    Ok(Json(order))
}
```

`Json` must stay the last extractor in the handler signature — `State`,
then `Path`, then `Json`.

Dashboard summary:

```rust
#[derive(Debug, Serialize)]
pub struct OrdersSummaryResponse {
    pub counts: HashMap<String, i64>,
    pub today: i64,
    pub recent: Vec<OrderSummaryResponse>,
}

pub async fn orders_summary(
    State(pool): State<PgPool>,
) -> Result<Json<OrdersSummaryResponse>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT status, COUNT(*) as "count!" FROM orders GROUP BY status"#
    )
    .fetch_all(&pool)
    .await?;

    // Seed all six at zero so the dashboard renders every tile even before a
    // single order of that status exists.
    let mut counts: HashMap<String, i64> =
        ORDER_STATUSES.iter().map(|s| ((*s).to_string(), 0)).collect();
    for row in rows {
        counts.insert(row.status, row.count);
    }

    // "Today" means today in Europe/Amsterdam, not UTC — the business's wall
    // clock, matching what the frontend renders (requirement.md Decision 12).
    let today = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM orders
        WHERE created_at >= date_trunc('day', now() AT TIME ZONE 'Europe/Amsterdam')
                            AT TIME ZONE 'Europe/Amsterdam'
        "#
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let recent = sqlx::query_as!(
        OrderSummaryResponse,
        r#"
        SELECT id, customer_name, delivery_type, total, status,
               created_at as "created_at: DateTime<Utc>"
        FROM orders
        ORDER BY created_at DESC, id DESC
        LIMIT $1
        "#,
        RECENT_ORDER_COUNT
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(OrdersSummaryResponse { counts, today, recent }))
}
```

The `as "count!"` suffix tells sqlx the aggregate is non-null, giving a
plain `i64` instead of `Option<i64>`.

## Group 2 — `apps/api/src/routes/admin/mod.rs`: register the four routes

Depends on: Group 1.

```rust
pub mod auth;
pub mod middleware;
pub mod orders;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use axum::{
    middleware::from_fn,
    routing::{get, patch, post},
    Router,
};
use sqlx::PgPool;

pub fn admin_router() -> Router<PgPool> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/orders", get(orders::list_orders))
        .route("/orders/summary", get(orders::orders_summary))
        .route("/orders/{id}", get(orders::get_order))
        .route("/orders/{id}/status", patch(orders::update_order_status))
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
```

Path-parameter syntax is `{id}`, not `:id` — this repo is on `axum = "0.8"`
(`apps/api/Cargo.toml`), where the old colon syntax panics at startup.
`/orders/summary` is a static segment and wins over `/orders/{id}` in
axum's matcher regardless of registration order.

Nothing in `apps/api/src/routes/mod.rs`, `lib.rs`, `main.rs` or `error.rs`
changes — the `.nest("/admin", admin::admin_router())` line and every
`AppError` variant this phase needs already exist.

## Group 3 — Offline query cache

Depends on: Groups 1, 2. With the dev stack up and `DATABASE_URL`
exported, from `apps/api`:

```bash
cargo sqlx prepare
```

Commit the updated `apps/api/.sqlx/`. Without this the Docker build
(`SQLX_OFFLINE=true`, no network to Postgres — see `tech-stack.md`'s
"Offline query checking for CI") fails on the six new queries.

## Group 4 — `apps/api/tests/common.rs`: add `patch_with_cookie`

Depends on: nothing. Follows the existing `post_with_cookie` shape exactly.

```rust
#[allow(dead_code)]
pub async fn patch_with_cookie(
    pool: PgPool,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let (status, _headers, resp_body) =
        send(pool, builder.body(Body::from(body.to_string())).unwrap()).await;
    (status, resp_body)
}
```

## Group 5 — `apps/api/tests/admin_orders.rs` (new)

Depends on: Groups 1, 2, 4.

Helpers: re-use `common::{get_with_cookie, patch_with_cookie,
post_with_cookie, session_cookie}`. `insert_admin` is currently private to
`admin_auth.rs` — move it into `common.rs` (behind `#[allow(dead_code)]`,
same as the other helpers) and have both test files call it rather than
copying it a second time. `argon2` is a normal dependency of the `api`
package, so test code can use it without a `Cargo.toml` change.

```rust
mod common;

use axum::http::StatusCode;
use common::{get_with_cookie, insert_admin, patch_with_cookie, post_with_cookie, session_cookie};
use serde_json::json;
use sqlx::PgPool;

/// `order_items.menu_item_id` is a nullable FK, so an order can be seeded
/// without any categories/menu_items rows.
async fn insert_order(pool: &PgPool, customer: &str, status: &str, total: &str) -> i64 {
    let total: rust_decimal::Decimal = total.parse().unwrap();
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             notes, subtotal, total, status)
        VALUES ($1, 'c@example.com', '+31612345678', 'pickup', 'no onions', $2, $2, $3)
        RETURNING id
        "#,
        customer,
        total,
        status,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO order_items
            (order_id, item_name, option_label, unit_price, quantity, subtotal)
        VALUES ($1, 'Jollof Rice', '2 L', $2, 1, $2)
        "#,
        id,
        total,
    )
    .execute(pool)
    .await
    .unwrap();

    id
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
```

Cases:

| Test | Asserts |
|---|---|
| `orders_list_requires_a_session` | `GET /api/admin/orders` with no cookie → `401 {"error":"unauthorized"}` |
| `order_detail_requires_a_session` | `GET /api/admin/orders/1` with no cookie → `401` |
| `status_update_requires_a_session` | `PATCH /api/admin/orders/1/status` with no cookie → `401`, and the row's status is unchanged in the DB |
| `orders_list_returns_newest_first_with_envelope` | three seeded orders → `200`, `body["total"] == 3`, `body["limit"] == 25`, `body["offset"] == 0`, `body["orders"][0]["id"]` is the newest, `total` serialized as a string (`"42.00"`, per `rust_decimal::serde::str`) |
| `orders_list_filters_by_status` | two `new` + one `ready` → `?status=ready` returns exactly one order and `total == 1` |
| `orders_list_paginates` | three orders, `?limit=2&offset=2` → one order returned, `total == 3`, `limit == 2`, `offset == 2` |
| `orders_list_rejects_an_unknown_status` | `?status=banana` → `400 {"error":"validation_error"}` with `fields[0].field == "status"` |
| `orders_list_rejects_an_out_of_range_limit` | `?limit=0` and `?limit=101` → `400`, `fields[0].field == "limit"` |
| `order_detail_returns_items` | → `200`, `body["items"][0]["item_name"] == "Jollof Rice"`, `body["items"][0]["option_label"] == "2 L"`, `body["notes"] == "no onions"`, `body["customer_email"]` present (not in the list shape) |
| `order_detail_404s_for_an_unknown_id` | `GET /api/admin/orders/999999` → `404 {"error":"not_found"}` |
| `status_update_changes_status_and_bumps_updated_at` | read `updated_at` before, `PATCH {"status":"preparing"}` → `200`, `body["status"] == "preparing"`, re-read the row: status is `preparing` and `updated_at > created_at` |
| `status_update_allows_moving_backward` | `ready` → `PATCH {"status":"new"}` → `200` (requirement.md Decision 4 — explicitly not a `409`) |
| `status_update_rejects_an_unknown_status` | `PATCH {"status":"banana"}` → `400 validation_error`, `fields[0].field == "status"`, row unchanged |
| `status_update_404s_for_an_unknown_id` | `PATCH /api/admin/orders/999999/status` → `404 not_found` |
| `summary_counts_every_status_and_recent_orders` | orders across three statuses → `200`, `counts` has all six keys with the uncounted three at `0`, `today == <seeded count>` (every `#[sqlx::test]` row is inserted now, so all are "today"), `recent` has at most 5 entries, newest first |

Each test takes `pool: PgPool` from `#[sqlx::test]`, which gives a fresh
migrated database per test — no cross-test interference on the `orders`
table.

## Group 6 — `apps/web/lib/fonts.ts` (new) + both root layouts

Depends on: nothing. See requirement.md Decision 16 — without this every
admin heading renders in the browser's default `serif`.

**`apps/web/lib/fonts.ts`:**
```ts
import { Fraunces, Work_Sans } from "next/font/google";

export const displayFont = Fraunces({
  subsets: ["latin"],
  variable: "--font-display",
  weight: ["600", "700"],
});

export const bodyFont = Work_Sans({
  subsets: ["latin"],
  variable: "--font-body",
});
```

**`apps/web/app/[locale]/layout.tsx`:** delete the two local
`Fraunces(...)`/`Work_Sans(...)` calls and the `next/font/google` import,
replacing them with `import { bodyFont, displayFont } from "@/lib/fonts";`.
Everything downstream (`${displayFont.variable} ${bodyFont.variable}` on
`<html>`) is unchanged.

**`apps/web/app/admin/layout.tsx`:**
```tsx
import type { Metadata } from "next";
import { bodyFont, displayFont } from "@/lib/fonts";
import "../globals.css";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body className="bg-background">{children}</body>
    </html>
  );
}
```

`bg-background` (white) rather than `globals.css`'s cream `body`
default — the dashboard is a dense data surface, not a marketing page.

## Group 7 — `apps/web/lib/admin-api.ts` (new)

Depends on: nothing. Server-only — it calls `cookies()` from
`next/headers`, so it must never be imported into a `"use client"` module.

```ts
import { cookies } from "next/headers";

const API_BASE_URL = process.env.API_BASE_URL;

/**
 * Server-to-server fetch against the Rust API with the browser's session
 * cookie forwarded. Same posture as lib/api.ts's apiFetch — API_BASE_URL is
 * container-internal and never reaches the browser.
 */
export async function adminApiFetch(path: string): Promise<Response> {
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  return fetch(`${API_BASE_URL}${path}`, {
    headers: { Cookie: (await cookies()).toString() },
    cache: "no-store",
  });
}

export async function adminApiJson<T>(path: string): Promise<T> {
  const response = await adminApiFetch(path);

  if (!response.ok) {
    throw new Error(`Admin API request to ${path} failed with status ${response.status}`);
  }

  return (await response.json()) as T;
}
```

## Group 8 — `apps/web/lib/admin-auth.ts`: refactor onto Group 7

Depends on: Group 7. Public behaviour unchanged — still `null` on a
missing `API_BASE_URL`, a missing cookie, or a non-`200`, which is what
`(protected)/layout.tsx`'s redirect depends on.

```ts
import { adminApiFetch } from "@/lib/admin-api";

export interface AdminUser {
  id: number;
  name: string;
  email: string;
  role: string;
}

export async function getCurrentAdmin(): Promise<AdminUser | null> {
  try {
    const response = await adminApiFetch("/api/admin/me");
    if (!response.ok) return null;
    return (await response.json()) as AdminUser;
  } catch {
    return null;
  }
}
```

One behavioural nuance worth knowing: the old version short-circuited to
`null` without a network call when there was no cookie at all. Now an
empty `Cookie` header is sent and the API answers `401`. One extra
Docker-internal round-trip on the logged-out path; not worth keeping a
special case for.

## Group 9 — `apps/web/lib/format.ts`: date helpers

Depends on: nothing. Appended next to the existing `formatPrice` family.

```ts
// Explicit timeZone is load-bearing: the server container runs in UTC and
// the browser in the visitor's zone, so an implicit zone would produce a
// hydration mismatch and a wrong "placed today". en-GB (not formatPrice's
// nl-NL) because the admin surface is English-only.
const DATE_TIME_FORMAT = new Intl.DateTimeFormat("en-GB", {
  timeZone: "Europe/Amsterdam",
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

export function formatDateTime(iso: string): string {
  return DATE_TIME_FORMAT.format(new Date(iso));
}
```

## Group 10 — `apps/web/lib/admin-orders.ts` (new)

Depends on: Group 7.

```ts
import { adminApiJson } from "@/lib/admin-api";

export const ORDER_STATUSES = [
  "new",
  "confirmed",
  "preparing",
  "ready",
  "completed",
  "cancelled",
] as const;

export type OrderStatus = (typeof ORDER_STATUSES)[number];

export function isOrderStatus(value: string): value is OrderStatus {
  return (ORDER_STATUSES as readonly string[]).includes(value);
}

export const ORDERS_PAGE_SIZE = 25;

export interface OrderSummary {
  id: number;
  customer_name: string;
  delivery_type: "pickup" | "delivery";
  total: string;
  status: OrderStatus;
  created_at: string;
}

export interface AdminOrderItem {
  id: number;
  item_name: string;
  option_label: string | null;
  unit_price: string;
  quantity: number;
  subtotal: string;
}

export interface AdminOrder {
  id: number;
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: "pickup" | "delivery";
  delivery_address: string | null;
  notes: string | null;
  subtotal: string;
  delivery_fee: string;
  total: string;
  status: OrderStatus;
  created_at: string;
  updated_at: string;
  items: AdminOrderItem[];
}

export interface AdminOrderList {
  orders: OrderSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface AdminOrdersSummary {
  counts: Record<OrderStatus, number>;
  today: number;
  recent: OrderSummary[];
}

export function getAdminOrders({
  status,
  page = 1,
}: { status?: OrderStatus; page?: number } = {}): Promise<AdminOrderList> {
  const params = new URLSearchParams({
    limit: String(ORDERS_PAGE_SIZE),
    offset: String((page - 1) * ORDERS_PAGE_SIZE),
  });
  if (status) params.set("status", status);
  return adminApiJson<AdminOrderList>(`/api/admin/orders?${params.toString()}`);
}

export function getAdminOrder(id: number): Promise<AdminOrder> {
  return adminApiJson<AdminOrder>(`/api/admin/orders/${id}`);
}

export function getAdminOrdersSummary(): Promise<AdminOrdersSummary> {
  return adminApiJson<AdminOrdersSummary>("/api/admin/orders/summary");
}
```

Prices stay `string` end to end (the API serializes `Decimal` via
`rust_decimal::serde::str`) and go through the existing `formatPrice` —
same contract `lib/api.ts`'s `MenuItem.price` already uses. No `any`
anywhere; `isOrderStatus` is the type guard that narrows an untrusted
`searchParams` value.

## Group 11 — `apps/web/components/admin-nav.tsx` (new)

Depends on: nothing. `"use client"` — it needs `usePathname` for the
active state.

```tsx
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

// `enabled: false` renders a muted, non-focusable span instead of a link —
// requirement.md Decision 2. Phase 14 flips Menu and Categories to true.
const NAV_ITEMS = [
  { label: "Dashboard", href: "/admin", enabled: true },
  { label: "Orders", href: "/admin/orders", enabled: true },
  { label: "Menu", href: "/admin/menu", enabled: false },
  { label: "Categories", href: "/admin/categories", enabled: false },
  { label: "Settings", href: "/admin/settings", enabled: false },
] as const;

const ITEM_CLASSES =
  "flex items-center justify-between rounded-xl px-4 py-2.5 text-sm font-semibold";

function isActive(pathname: string, href: string) {
  return href === "/admin" ? pathname === "/admin" : pathname.startsWith(href);
}

export function AdminNav() {
  const pathname = usePathname();

  return (
    <nav aria-label="Admin sections" className="flex flex-col gap-1">
      {NAV_ITEMS.map((item) =>
        item.enabled ? (
          <Link
            key={item.href}
            href={item.href}
            aria-current={isActive(pathname, item.href) ? "page" : undefined}
            className={`${ITEM_CLASSES} transition-colors duration-150 ease-out-expo focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              isActive(pathname, item.href)
                ? "bg-primary text-white"
                : "text-text hover:bg-background-soft"
            }`}
          >
            {item.label}
          </Link>
        ) : (
          <span key={item.href} aria-disabled="true" className={`${ITEM_CLASSES} text-text-muted opacity-50`}>
            {item.label}
            <span className="text-[0.65rem] font-semibold uppercase tracking-wider">Soon</span>
          </span>
        ),
      )}
    </nav>
  );
}
```

`next/link`, **not** `@/i18n/navigation`'s `Link` (requirement.md
Decision 13) — the latter would rewrite these to `/en/admin/...`.

## Group 12 — `apps/web/components/admin-status-badge.tsx` (new)

Depends on: Group 10. No `"use client"` — purely presentational.

```tsx
import type { OrderStatus } from "@/lib/admin-orders";

const STATUS_CLASSES: Record<OrderStatus, string> = {
  new: "bg-accent-light text-primary-dark",
  confirmed: "bg-background-soft text-primary-dark",
  preparing: "bg-cream text-text",
  ready: "bg-whatsapp/15 text-whatsapp-dark",
  completed: "bg-card text-text-muted",
  cancelled: "bg-text/10 text-text-muted",
};

export function AdminStatusBadge({ status }: { status: OrderStatus }) {
  return (
    <span
      className={`inline-block rounded-full px-3 py-1 text-xs font-semibold capitalize ${STATUS_CLASSES[status]}`}
    >
      {status}
    </span>
  );
}
```

A `Record<OrderStatus, string>` (not a partial map with a fallback) so
adding a seventh status to `ORDER_STATUSES` becomes a compile error here
rather than an unstyled badge at runtime.

## Group 13 — `apps/web/app/admin/(protected)/layout.tsx`: the shell

Depends on: Groups 8, 11. The auth gate and `force-dynamic` are unchanged;
only the returned markup grows.

```tsx
import { redirect } from "next/navigation";
import { getCurrentAdmin } from "@/lib/admin-auth";
import { AdminNav } from "@/components/admin-nav";
import { AdminLogoutButton } from "@/components/admin-logout-button";

export const dynamic = "force-dynamic";

export default async function ProtectedAdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const admin = await getCurrentAdmin();
  if (!admin) {
    redirect("/admin/login");
  }

  return (
    <div className="min-h-dvh md:grid md:grid-cols-[15rem_1fr]">
      <aside className="flex flex-col gap-6 border-b border-text/10 bg-background-soft/40 p-5 md:min-h-dvh md:border-b-0 md:border-r">
        <p className="m-0 text-base font-semibold text-primary">Zain&apos;s Admin</p>
        <AdminNav />
        <div className="flex flex-col items-start gap-3 pt-4 md:mt-auto">
          <p className="m-0 text-xs text-text-muted">Signed in as {admin.name}</p>
          <AdminLogoutButton />
        </div>
      </aside>
      <main className="p-5 md:p-10">{children}</main>
    </div>
  );
}
```

Note the sidebar stacks above the content below `md` — no hamburger, no
GSAP: the public `SiteHeader`'s mobile nav machinery is deliberately not
reused here (five items, three of them disabled, do not need a drawer).

`main` carries `globals.css`'s `@layer base` rules for `main h1`
(`clamp(2.25rem, 4.5vw, 3.25rem)`) and `main section { padding-block:
var(--spacing-section) }`. Admin pages should therefore use plain `<div>`
wrappers rather than `<section>` elements, or inherit marketing-page
section rhythm that is far too airy for a data table.

## Group 14 — `apps/web/app/admin/(protected)/page.tsx`: the dashboard

Depends on: Groups 9, 10, 12.

```tsx
import Link from "next/link";
import type { Metadata } from "next";
import { getAdminOrdersSummary, ORDER_STATUSES } from "@/lib/admin-orders";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Dashboard | Zain's Admin",
};

export default async function AdminDashboardPage() {
  const summary = await getAdminOrdersSummary();

  return (
    <div className="flex flex-col gap-10">
      <h1 className="m-0">Dashboard</h1>

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div className="rounded-card bg-background-soft p-5">
          <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">
            Placed today
          </p>
          <p className="m-0 text-3xl font-semibold text-primary">{summary.today}</p>
        </div>
        {ORDER_STATUSES.map((status) => (
          <Link
            key={status}
            href={`/admin/orders?status=${status}`}
            className="rounded-card bg-card p-5 transition-transform duration-150 ease-out-expo hover:-translate-y-0.5 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
          >
            {/* `capitalize`, not `uppercase` — both set text-transform, so
                combining them silently drops one. */}
            <p className="m-0 text-xs font-semibold capitalize tracking-wider text-text-muted">
              {status}
            </p>
            <p className="m-0 text-3xl font-semibold">{summary.counts[status]}</p>
          </Link>
        ))}
      </div>

      <div className="flex flex-col gap-4">
        <div className="flex items-baseline justify-between">
          <h2 className="m-0 text-xl font-semibold">Recent orders</h2>
          <Link href="/admin/orders" className="text-sm font-semibold text-primary underline">
            View all
          </Link>
        </div>

        {summary.recent.length === 0 ? (
          <p className="m-0 text-text-muted">No orders yet.</p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {summary.recent.map((order) => (
              <li key={order.id}>
                <Link
                  href={`/admin/orders/${order.id}`}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-card border border-text/10 px-5 py-4 hover:bg-background-soft focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                >
                  <span className="font-semibold">#{order.id}</span>
                  <span>{order.customer_name}</span>
                  <span className="text-text-muted">{formatDateTime(order.created_at)}</span>
                  <span className="font-semibold">{formatPrice(order.total)}</span>
                  <AdminStatusBadge status={order.status} />
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
```

Each status tile links into the order list pre-filtered to that status —
the summary's whole point is "what needs attention", so a count that is
not clickable would be a dead end.

## Group 15 — `apps/web/app/admin/(protected)/orders/page.tsx`: the list

Depends on: Groups 9, 10, 12.

```tsx
import Link from "next/link";
import type { Metadata } from "next";
import {
  getAdminOrders,
  isOrderStatus,
  ORDERS_PAGE_SIZE,
  ORDER_STATUSES,
} from "@/lib/admin-orders";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Orders | Zain's Admin",
};

function buildHref(status: string | undefined, page: number) {
  const params = new URLSearchParams();
  if (status) params.set("status", status);
  if (page > 1) params.set("page", String(page));
  const query = params.toString();
  return query ? `/admin/orders?${query}` : "/admin/orders";
}

export default async function AdminOrdersPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const { status: rawStatus, page: rawPage } = await searchParams;

  // Anything not a real status is treated as "All" rather than 400ing the
  // page — a hand-typed URL shouldn't break the dashboard.
  const status = rawStatus && isOrderStatus(rawStatus) ? rawStatus : undefined;
  const parsedPage = Number.parseInt(rawPage ?? "1", 10);
  const page = Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1;

  const { orders, total } = await getAdminOrders({ status, page });
  const pageCount = Math.max(1, Math.ceil(total / ORDERS_PAGE_SIZE));

  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Orders</h1>

      <div className="flex flex-wrap gap-2">
        {[undefined, ...ORDER_STATUSES].map((value) => (
          <Link
            key={value ?? "all"}
            href={buildHref(value, 1)}
            aria-current={status === value ? "page" : undefined}
            className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider transition-colors duration-150 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              status === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value ?? "All"}
          </Link>
        ))}
      </div>

      {orders.length === 0 ? (
        <p className="m-0 text-text-muted">
          {status ? `No ${status} orders.` : "No orders yet."}
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
                <th scope="col" className="py-3 pr-4 font-semibold">Order</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Customer</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Type</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Total</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Status</th>
                <th scope="col" className="py-3 font-semibold">Placed</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((order) => (
                <tr key={order.id} className="border-b border-text/10 hover:bg-background-soft">
                  <td className="py-3 pr-4">
                    <Link
                      href={`/admin/orders/${order.id}`}
                      className="font-semibold text-primary underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                    >
                      #{order.id}
                    </Link>
                  </td>
                  <td className="py-3 pr-4">{order.customer_name}</td>
                  <td className="py-3 pr-4 capitalize">{order.delivery_type}</td>
                  <td className="py-3 pr-4 font-semibold">{formatPrice(order.total)}</td>
                  <td className="py-3 pr-4"><AdminStatusBadge status={order.status} /></td>
                  <td className="py-3 text-text-muted">{formatDateTime(order.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {pageCount > 1 && (
        <nav aria-label="Pagination" className="flex items-center justify-center gap-6 text-sm">
          {page > 1 ? (
            <Link href={buildHref(status, page - 1)} className="font-semibold text-primary underline">
              ‹ Previous
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">‹ Previous</span>
          )}
          <span className="text-text-muted">Page {page} of {pageCount}</span>
          {page < pageCount ? (
            <Link href={buildHref(status, page + 1)} className="font-semibold text-primary underline">
              Next ›
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">Next ›</span>
          )}
        </nav>
      )}
    </div>
  );
}
```

`searchParams` is a `Promise` in Next.js 16 — it must be awaited, same as
`params` already is in `app/[locale]/layout.tsx`.

## Group 16 — `apps/web/app/api/admin/orders/[id]/status/route.ts` (new)

Depends on: Groups 1, 2. The one browser-initiated write this phase has;
mirrors `app/api/admin/logout/route.ts`'s cookie forwarding
(requirement.md Decision 10). No `Set-Cookie` loop is needed — the API
doesn't set one on this route — but forwarding the inbound `Cookie` is
mandatory or `require_admin` rejects every call.

```ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const { id } = await params;
  const body = await request.text();

  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/orders/${id}/status`, {
    method: "PATCH",
    headers: {
      "Content-Type": "application/json",
      Cookie: request.headers.get("cookie") ?? "",
    },
    body,
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
```

`id` is forwarded as an opaque path segment; the Rust side's
`Path<i64>` extractor is what rejects a non-numeric one.

## Group 17 — `apps/web/components/admin-order-status-select.tsx` (new)

Depends on: Groups 10, 16. `"use client"`.

```tsx
"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { ORDER_STATUSES, type OrderStatus } from "@/lib/admin-orders";
import { Notice } from "@/components/notice";

export function AdminOrderStatusSelect({
  orderId,
  status,
}: {
  orderId: number;
  status: OrderStatus;
}) {
  const router = useRouter();
  const [value, setValue] = useState<OrderStatus>(status);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleChange(next: OrderStatus) {
    const previous = value;
    setValue(next);
    setError(null);
    setSaving(true);

    const response = await fetch(`/api/admin/orders/${orderId}/status`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: next }),
    });

    setSaving(false);

    if (!response.ok) {
      setValue(previous);
      setError("Could not update the status. Please try again.");
      return;
    }

    router.refresh();
  }

  return (
    <div className="flex flex-col gap-2">
      <label htmlFor="order-status" className="text-xs font-semibold uppercase tracking-wider text-text-muted">
        Status
      </label>
      <select
        id="order-status"
        className="field max-w-[16rem] capitalize"
        value={value}
        disabled={saving}
        onChange={(event) => handleChange(event.target.value as OrderStatus)}
      >
        {ORDER_STATUSES.map((option) => (
          <option key={option} value={option} className="capitalize">
            {option}
          </option>
        ))}
      </select>
      {error && <Notice>{error}</Notice>}
    </div>
  );
}
```

The `as OrderStatus` cast on `event.target.value` is sound because the
`<option>` values are exactly `ORDER_STATUSES`; it is a cast, not an
`any`, so the no-`any` rule is satisfied. `router.refresh()` re-runs the
server component so the badge, `updated_at` line and any other derived
copy stay in step with the select.

Reverting `value` to `previous` on failure matters: without it the select
would keep displaying a status the server rejected.

## Group 18 — `apps/web/app/admin/(protected)/orders/[id]/page.tsx` (new)

Depends on: Groups 9, 10, 12, 17.

```tsx
import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminOrder } from "@/lib/admin-orders";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { AdminOrderStatusSelect } from "@/components/admin-order-status-select";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Order | Zain's Admin",
};

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">{label}</p>
      <p className="m-0">{value}</p>
    </div>
  );
}

export default async function AdminOrderDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const orderId = Number.parseInt(id, 10);
  if (!Number.isInteger(orderId) || orderId < 1) {
    notFound();
  }

  const order = await getAdminOrder(orderId);

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/orders" className="text-sm font-semibold text-primary underline">
        ‹ Back to orders
      </Link>

      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Order #{order.id}</h1>
        <AdminStatusBadge status={order.status} />
      </div>

      <AdminOrderStatusSelect orderId={order.id} status={order.status} />

      <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="Customer" value={order.customer_name} />
        <Field label="Email" value={order.customer_email} />
        <Field label="Phone" value={order.customer_phone} />
        <Field label="Type" value={order.delivery_type} />
        <Field label="Address" value={order.delivery_address ?? "—"} />
        <Field label="Placed" value={formatDateTime(order.created_at)} />
        <Field label="Last updated" value={formatDateTime(order.updated_at)} />
      </div>

      <div>
        <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">Notes</p>
        <p className="m-0 whitespace-pre-wrap">{order.notes?.trim() || "—"}</p>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Item</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Unit price</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Qty</th>
              <th scope="col" className="py-3 font-semibold">Subtotal</th>
            </tr>
          </thead>
          <tbody>
            {order.items.map((item) => (
              <tr key={item.id} className="border-b border-text/10">
                <td className="py-3 pr-4">
                  {item.item_name}
                  {item.option_label && (
                    <span className="text-text-muted"> · {item.option_label}</span>
                  )}
                </td>
                <td className="py-3 pr-4">{formatPrice(item.unit_price)}</td>
                <td className="py-3 pr-4">{item.quantity}</td>
                <td className="py-3 font-semibold">{formatPrice(item.subtotal)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <dl className="m-0 ml-auto flex w-full max-w-[18rem] flex-col gap-2 text-sm">
        <div className="flex justify-between">
          <dt className="text-text-muted">Subtotal</dt>
          <dd className="m-0">{formatPrice(order.subtotal)}</dd>
        </div>
        <div className="flex justify-between">
          <dt className="text-text-muted">Delivery fee</dt>
          <dd className="m-0">{formatPrice(order.delivery_fee)}</dd>
        </div>
        <div className="flex justify-between border-t border-text/15 pt-2 text-base font-semibold">
          <dt>Total</dt>
          <dd className="m-0">{formatPrice(order.total)}</dd>
        </div>
      </dl>
    </div>
  );
}
```

`getAdminOrder` throws on a `404` from the API (that's `adminApiJson`'s
contract), which surfaces as Next.js's error boundary rather than a 404
page. If that reads badly in practice, catch the throw and call
`notFound()` — implementation's call, but make it one of the two, not an
unhandled 500.

## Group 19 — Nothing else changes

- `apps/web/proxy.ts` — already excludes `/admin` and `/api` from the
  next-intl matcher (Phase 11 Group 11). Untouched.
- `apps/web/messages/{en,nl}.json` — admin is English-only
  (requirement.md Decision 14). Untouched.
- `apps/web/app/admin/login/page.tsx`, `components/admin-login-form.tsx`,
  `components/admin-logout-button.tsx` — unchanged (the logout button is
  reused as-is inside the new shell).
- No migration, no `docker-compose*.yml`/`Dockerfile`/`Caddyfile`/CI/docs
  changes.

## Verification

See `validation.md`.
