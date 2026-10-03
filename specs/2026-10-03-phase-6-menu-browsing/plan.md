# Phase 6 — Menu Browsing (Read-Only): Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed
and sanity-checked before moving to the next. Backend groups (1–8) touch
`apps/api` and `.github/workflows/deploy.yml`; frontend groups (9–18) touch
`apps/web`.

## Group 0 — Branch

0.1. Create branch `2026-10-03-phase-6-menu-browsing` off `master` (done).

---

## Backend (`apps/api`)

## Group 1 — Cargo dependencies

Depends on: Group 0.

1.1. `apps/api/Cargo.toml` — add the `rust_decimal` feature to the existing
     `sqlx` dependency (maps Postgres `NUMERIC` ↔ `rust_decimal::Decimal`
     at compile-check time) and add the `rust_decimal` crate with its
     `serde-with-str` feature (forces `price` to serialize as a JSON
     **string**, e.g. `"6000.00"`, never a float — avoids reintroducing
     rounding risk one hop after Phase 4 picked `NUMERIC(10,2)` to avoid
     exactly that):

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "macros", "migrate", "chrono", "rust_decimal"] }
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1", features = ["serde-with-str"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

`tower`/`http-body-util` are dev-only, needed to drive the `Router` through
`tower::ServiceExt::oneshot` in integration tests (Group 6).

## Group 2 — Split into a lib + routes (first route module in the repo)

Depends on: Group 1.

Today `apps/api` is bin-only (`src/main.rs` only), so an integration test
under `apps/api/tests/` can't import the app-building code. Standard fix:
thin binary + library crate split.

2.1. New `apps/api/src/lib.rs`:

```rust
pub mod error;
pub mod routes;

use axum::Router;
use sqlx::PgPool;

pub fn build_app(pool: PgPool) -> Router {
    Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health))
        .with_state(pool)
}
```

2.2. New `apps/api/src/error.rs` — error-handling convention for the two
     new endpoints (distinct from `/health`'s own pattern, where a DB
     failure *is* the signal being probed — a query failure elsewhere is
     just a generic server error):

```rust
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

pub struct AppError(sqlx::Error);

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self { Self(err) }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("database error: {:?}", self.0); // no tracing crate yet
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_server_error" }))).into_response()
    }
}
```

2.3. New `apps/api/src/routes/mod.rs`:

```rust
pub mod categories;
pub mod health;
pub mod menu_items;

use axum::{routing::get, Router};
use sqlx::PgPool;

pub fn api_router() -> Router<PgPool> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
}
```

2.4. New `apps/api/src/routes/health.rs` — move the existing `/health`
     handler out of `main.rs` verbatim (no behavior change).

2.5. `apps/api/src/main.rs` shrinks to startup/wiring only: connect the
     pool, run `sqlx::migrate!("./migrations")`, call
     `api::build_app(pool)`, `axum::serve`. (`use api::build_app` works
     because Cargo auto-links a binary to its package's own `src/lib.rs`
     under the package name `api`.)

## Group 3 — `GET /api/categories`

Depends on: Group 2.

3.1. New `apps/api/src/routes/categories.rs`:

```rust
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_categories(State(pool): State<PgPool>) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let categories = sqlx::query_as!(
        CategoryResponse,
        r#"SELECT id, name, description FROM categories WHERE deleted_at IS NULL ORDER BY id"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(categories))
}
```

`created_at`/`updated_at`/`deleted_at` are deliberately not exposed —
nothing downstream needs them; the contract stays additive-only going
forward.

## Group 4 — `GET /api/menu-items`

Depends on: Group 2 (sequenced after Group 3 for consistency, not a hard
dependency).

4.1. New `apps/api/src/routes/menu_items.rs`:

```rust
use axum::{extract::{Query, State}, Json};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MenuItemResponse {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub is_featured: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListMenuItemsQuery {
    pub category_id: Option<i64>,
}

/// Returns all non-deleted items, including unavailable ones — see
/// requirement.md Decision 5. Does NOT filter on is_available.
pub async fn list_menu_items(
    State(pool): State<PgPool>,
    Query(params): Query<ListMenuItemsQuery>,
) -> Result<Json<Vec<MenuItemResponse>>, AppError> {
    let items = sqlx::query_as!(
        MenuItemResponse,
        r#"
        SELECT id, category_id, name, description, price, image_url,
               is_available, is_featured
        FROM menu_items
        WHERE deleted_at IS NULL
          AND ($1::BIGINT IS NULL OR category_id = $1)
        ORDER BY id
        "#,
        params.category_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(items))
}
```

`($1::BIGINT IS NULL OR category_id = $1)` is the standard sqlx
optional-filter idiom for `Option<i64>`; the explicit cast avoids Postgres
failing to infer a type for an untyped `NULL` parameter.

## Group 5 — Regenerate the sqlx offline cache (do not skip)

Depends on: Groups 3, 4.

5.1. With the dev stack's Postgres reachable and `DATABASE_URL` exported:

```bash
cd apps/api
cargo sqlx prepare
```

5.2. Commit the new `apps/api/.sqlx/query-*.json` files. Required because
     `apps/api/Dockerfile`'s build stage sets `SQLX_OFFLINE=true` with no
     DB access — a stale/missing cache breaks the Docker build in CI
     (flagged as a known risk in Phase 4's own requirement.md).

5.3. Sanity check: `SQLX_OFFLINE=true cargo check` succeeds locally using
     only the committed cache.

5.4. **Re-run this again after Group 6** — writing tests adds more
     `query!`/`query_as!`/`query_scalar!` calls that also need cache
     entries.

## Group 6 — Integration tests (first in the repo)

Depends on: Groups 2–5.

6.1. New `apps/api/tests/menu_browsing.rs`, using `#[sqlx::test]` (gives
     each test function a fresh, auto-migrated, throwaway Postgres
     database via `DATABASE_URL` — requires `CREATEDB` on the connecting
     role, true both for the dev stack's bootstrap user and a fresh CI
     service container) and `tower::ServiceExt::oneshot` against
     `api::build_app(pool)`.

6.2. Tests to write:
   - `categories` endpoint excludes soft-deleted rows.
   - `menu-items` endpoint, unfiltered, returns all non-deleted items
     across categories (a soft-deleted item is excluded).
   - `?category_id=` filters correctly to one category.
   - An unavailable (but non-deleted) item **is present** in the response
     with `is_available: false` — this test locks in Decision 5 and will
     fail loudly if someone later "fixes" the query to filter
     `is_available = true`.

## Group 7 — CI workflow changes

Depends on: Group 6. `.github/workflows/deploy.yml` currently triggers
only on `push: branches: [master]` and has no test step at all.

7.1. Add a `pull_request` trigger alongside the existing `push` trigger,
     and a new `test-api` job gating the existing jobs, which gain
     `if: github.event_name == 'push'` so they never run on a PR event:

```yaml
on:
  push:
    branches: [master]
  pull_request:
    branches: [master]

jobs:
  test-api:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_USER: zainstreat_test
          POSTGRES_PASSWORD: zainstreat_test
          POSTGRES_DB: zainstreat_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U zainstreat_test -d zainstreat_test"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 5
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Run API tests
        working-directory: apps/api
        env:
          DATABASE_URL: postgres://zainstreat_test:zainstreat_test@localhost:5432/zainstreat_test
          SQLX_OFFLINE: "true"
        run: cargo test

  build-and-push:
    needs: test-api
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    # ...existing steps unchanged...

  deploy:
    needs: build-and-push
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    # ...existing steps unchanged...
```

`test-api` runs on both PRs and pushes to master, so PRs show pass/fail
before merge. `SQLX_OFFLINE: "true"` during `cargo test`'s compile step
also makes this job double as a cache-freshness check for Group 5 — a
stale `.sqlx/` cache fails here with a clear compile error before
`build-and-push` would waste time on a Docker build failing the same way.

## Group 8 — Docs

Depends on: Group 7.

8.1. `docs/local-development.md` — add a short "Running API tests" note:
     `cargo test` from `apps/api` requires `DATABASE_URL` exported to a
     reachable Postgres with `CREATEDB` privilege (the dev stack's
     `postgres` service works as-is); each test gets its own throwaway
     database via `#[sqlx::test]`, migrated automatically. Also note: "run
     `cargo sqlx prepare` again after adding/changing any query, including
     in `tests/`."

---

## Frontend (`apps/web`)

## Group 9 — API base URL wiring

Depends on: Group 0 (independent of backend groups — can run in parallel
once the API contract above is fixed).

9.1. `docker-compose.yml`'s `web` service currently has no `environment:`
     block at all. Add:

```yaml
  web:
    image: node:22-bookworm-slim
    working_dir: /app
    volumes:
      - ./apps/web:/app
      - web_node_modules:/app/node_modules
    command: sh -c "corepack enable && pnpm install && pnpm dev"
    environment:
      API_BASE_URL: http://api:${API_PORT:-8080}
    ports:
      - "${WEB_PORT:-3000}:3000"
    depends_on:
      api:
        condition: service_healthy
```

   Server-only — **not** `NEXT_PUBLIC_API_BASE_URL` — since nothing fetches
   from the browser in this phase (no client component calls the API).
   `docker-compose.prod.yml` never overrides `web`'s `environment`, and
   both containers share one Compose network with the same service name
   `api` in dev and prod, so this line works unchanged in production — no
   GitHub Actions secret, no Dockerfile build-arg needed (pure runtime env
   var, not baked into the image).

9.2. `docs/local-development.md` — document `API_BASE_URL` and that it
     resolves via Docker's internal DNS in both environments.

## Group 10 — Typed API client

Depends on: Group 9.

10.1. New `apps/web/lib/api.ts`:

```typescript
export interface Category {
  id: number;
  name: string;
  description: string | null;
}

export interface MenuItem {
  id: number;
  category_id: number;
  name: string;
  description: string | null;
  price: string;
  image_url: string | null;
  is_available: boolean;
  is_featured: boolean;
}

const API_BASE_URL = process.env.API_BASE_URL;

async function apiFetch<T>(path: string): Promise<T> {
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  const response = await fetch(`${API_BASE_URL}${path}`, { cache: "no-store" });

  if (!response.ok) {
    throw new Error(`API request to ${path} failed with status ${response.status}`);
  }

  return (await response.json()) as T;
}

export function getCategories(): Promise<Category[]> {
  return apiFetch<Category[]>("/api/categories");
}

export function getMenuItems(categoryId?: number): Promise<MenuItem[]> {
  const query = categoryId !== undefined ? `?category_id=${categoryId}` : "";
  return apiFetch<MenuItem[]>(`/api/menu-items${query}`);
}
```

`price: string` — matches the backend's `rust_decimal::serde::str` choice
(Group 1.1), no defensive union type needed. `cache: "no-store"` satisfies
Decision 6 (always fresh, no ISR).

10.2. New `apps/web/lib/format.ts`:

```typescript
export function parsePrice(price: string): number {
  return Number.parseFloat(price);
}

export function formatPrice(price: string): string {
  const value = parsePrice(price);
  return `₦${value.toLocaleString("en-NG", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}
```

Formatted in the `en-NG` locale regardless of UI locale — Naira is a
business-identity fact, not a translatable string. Kept separate from
`api.ts` since price formatting will be needed again by Phase 7 (cart) and
Phase 8 (order review).

## Group 11 — `next.config.ts` image host

Depends on: Group 0 (independent of Groups 9–10).

11.1. `apps/web/next.config.ts` — add `images.remotePatterns` for the dev
      seed data's placeholder host:

```typescript
const nextConfig: NextConfig = {
  output: "standalone",
  agentRules: false,
  images: {
    remotePatterns: [
      { protocol: "https", hostname: "placehold.co" },
    ],
  },
};
```

Leave a comment noting the production/MinIO host gets added alongside this
entry in Phase 11 — don't remove it then.

Note: Chin Chin, Efo Soup, Egusi Soup, and Small Chops now seed real photos
from `apps/web/public/images/menu/` via root-relative `image_url` paths
(e.g. `/images/menu/egusi-soup.jpeg`) — these are same-origin and need no
`remotePatterns` entry. Only the remaining `placehold.co`-based seed items
need the entry above.

## Group 12 — New components

Depends on: Groups 10, 11.

12.1. `apps/web/components/menu-item-card.tsx` — server component. Renders
      `next/image` when `image_url` is set (the first dynamic/remote image
      in the codebase, vs. `Logo`'s static local import), falling back to
      the existing `ImageSlot` placeholder when null. Shows an
      "unavailable" badge and reduced opacity (`opacity-60 grayscale-[30%]`)
      when `!is_available`. Uses `formatPrice` from Group 10.2.

12.2. `apps/web/components/category-filter.tsx` — server component, pure
      `Link`-based pills driven by `searchParams` (no client state), an
      "All" pill plus one per category, `aria-current="page"` on the
      active one.

12.3. No new `EmptyState` component — reuse `components/notice.tsx` (the
      existing soft informational banner, already used for an analogous
      "nothing here yet" case in Phase 5's `ContactPage.formComingSoon`)
      for the empty-category message.

## Group 13 — `/menu` route

Depends on: Groups 10, 12.

13.1. New `apps/web/app/[locale]/menu/page.tsx`:
   - `export const dynamic = "force-dynamic";` and the first per-page
     `generateMetadata` in the codebase (`getTranslations` from
     `next-intl/server`, since `generateMetadata` can't use the
     `useTranslations` hook).
   - Reads `category` from `searchParams` (Decision 7), parses to a
     `number | undefined`.
   - Fetches `getCategories()` and `getMenuItems(selectedCategoryId)`.
   - Groups items by `category_id` and renders one `<section>` per
     category (or just the selected one), each with its `MenuItemCard`
     grid or, if empty, the `Notice` empty-state message
     (`MenuPage.emptyCategory`).
   - Renders `CategoryFilter` above the category sections.

## Group 14 — Homepage "Featured Menu" section

Depends on: Groups 10, 12.

14.1. `apps/web/app/[locale]/page.tsx` becomes `async`. Fetch
      `getMenuItems()` once, filter to `is_featured && is_available`, cap
      at ~6 items, render via `MenuItemCard` in a new section.

14.2. **Placement:** after the existing services-overview section and
      before the "Why Choose Us"/trust section — closest practical match
      to README's suggested ordering without undoing Phase 5's already-
      shipped section merge.

14.3. **Zero-featured-items degradation: hide the section entirely**, no
      empty-state message — this is a promotional homepage block, not a
      page the visitor opened specifically to browse (unlike `/menu`,
      where category-level emptiness is meaningful information worth
      surfacing).

14.4. Include a "View Full Menu" `ButtonLink` to `/menu`.

## Group 15 — Nav discoverability (required)

Depends on: Group 13.

15.1. Add a `menu` entry to `SiteHeader`'s `NAV_ITEMS`
      (`apps/web/components/site-header.tsx:11-16`):

```typescript
const NAV_ITEMS = [
  { href: "/", key: "home" },
  { href: "/about", key: "about" },
  { href: "/services", key: "services" },
  { href: "/menu", key: "menu" },
  { href: "/contact", key: "contact" },
] as const;
```

15.2. Add the matching link to `SiteFooter`'s link list. Without this,
      `/menu` is a real but undiscoverable route except via the homepage's
      "View Full Menu" button — not acceptable for a phase whose entire
      point is menu browsing.

## Group 16 — i18n message keys

Depends on: Groups 13–15. Added to both `apps/web/messages/en.json` and
`apps/web/messages/nl.json` in the same change, matching the existing flat
`PageName: { key: value }` convention:

| Namespace.Key | English | Dutch |
|---|---|---|
| `MenuPage.heading` | Our Menu | Ons Menu |
| `MenuPage.metaTitle` | Menu \| Zain's Treat n More | Menu \| Zain's Treat n More |
| `MenuPage.metaDescription` | Browse our full menu of freshly prepared halal meals, snacks, drinks, and more. | Bekijk ons volledige menu met vers bereide halal maaltijden, snacks, drankjes en meer. |
| `MenuPage.categoryNavLabel` | Menu categories | Menucategorieën |
| `MenuPage.allCategories` | All | Alles |
| `MenuPage.emptyCategory` | Nothing available in this category right now. | Op dit moment is er niets beschikbaar in deze categorie. |
| `MenuPage.unavailable` | Currently unavailable | Momenteel niet beschikbaar |
| `MenuPage.loadError` | We couldn't load the menu right now. Please try again shortly. | We konden het menu nu niet laden. Probeer het later opnieuw. |
| `HomePage.featuredMenuEyebrow` | Crowd Favorites | Publieksfavorieten |
| `HomePage.featuredMenuHeading` | Taste Our Featured Menu | Proef Ons Uitgelichte Menu |
| `HomePage.viewFullMenu` | View Full Menu | Bekijk Volledig Menu |
| `SiteHeader.menu` | Menu | Menu |
| `SiteFooter.menu` | Menu | Menu |

(Best-effort Dutch translation, per Phase 5's established precedent —
flagged for a later native-speaker review pass, not blocking this phase.)

## Group 17 — Error boundary

Depends on: Group 13.

17.1. New `apps/web/app/[locale]/menu/error.tsx` (`"use client"`), a
      minimal friendly fallback using `MenuPage.loadError` for when
      `apps/api`/Postgres is unreachable. First error boundary in the
      codebase — this is the first page whose successful render depends on
      a second service being up.

## Group 18 — Docs

Depends on: Groups 9–17.

18.1. `docs/local-development.md` — Group 9.2's note, plus `MenuPage` as
      an established i18n namespace.

18.2. `specs/tech-stack.md` — one sentence in the Frontend section noting
      the `API_BASE_URL` server-only env var convention and that
      `apps/web` now performs real data fetching (previously "zero
      data-fetching exists").

---

## Group 19 — Verification

See `validation.md` for the full pass/fail checklist.
