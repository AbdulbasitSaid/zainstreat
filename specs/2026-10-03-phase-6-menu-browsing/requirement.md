# Phase 6 — Menu Browsing (Read-Only): Requirements

## Source

`specs/roadmap.md`, Phase 6:

> - API: list categories, list available menu items (with category filter).
> - Menu page: category filtering, item cards (name, description, price,
>   image), empty-category state, unavailable-item state.

## Context

Phases 1–5 are merged to `master`. Phase 4 (Data Model) already created the
`categories` and `menu_items` tables this phase queries — no migration work
is needed here. Phase 5 (Public Static Pages) built the header/footer/brand
system/shared components (`Button`, `PageHero`, `Notice`, `ImageSlot`, etc.)
and its own `validation.md` explicitly says it's "ready for Phase 6
(Menu Browsing) to add the homepage's 'Featured Menu' section and the
`/menu` route on top of this layout" — Phase 5 deliberately shipped with no
homepage menu section since no real menu data existed yet.

`apps/api` today is a single `src/main.rs` with one route (`GET /health`,
DB-aware since Phase 4). There are no route modules, DTOs, error-handling
convention, or tests anywhere in the API. **This phase is the first to add
real `sqlx::query_as!`-backed routes and the first test suite in the repo.**

`apps/web` today has zero data-fetching anywhere — no `fetch()` calls, no
API client, no env var for reaching `apps/api`. **This phase is the first
time the web app calls the API at all.**

The schema this phase reads (from
`apps/api/migrations/20261002162443_initial_schema.up.sql`):

```sql
categories(id BIGSERIAL, name TEXT, description TEXT,
           created_at, updated_at, deleted_at)

menu_items(id BIGSERIAL, category_id BIGINT FK -> categories,
           name TEXT, description TEXT, price NUMERIC(10,2),
           image_url TEXT, is_available BOOLEAN DEFAULT true,
           is_featured BOOLEAN DEFAULT false,
           created_at, updated_at, deleted_at)
```

`apps/api/seed.sql` (dev-only) seeds 3 categories / 6 items with
`image_url` pointing at `placehold.co` placeholders — real photo
upload/storage via MinIO is Phase 11's job, not this phase's. There is also
no admin/CMS yet to create menu data; `seed.sql` is the only data source
until Phase 11.

## Decisions

Four genuine open questions were identified during planning and put to the
user directly.

1. **API shape.** The roadmap bullet could be read as either two separate
   list endpoints or one combined/nested endpoint. **Decision: two separate
   REST endpoints** — `GET /api/categories` and `GET /api/menu-items`
   (optional `?category_id=` filter) — matching the roadmap's literal
   wording and keeping both the sqlx queries and the response shapes
   simple.

2. **Homepage scope.** Phase 5's validation.md flagged the homepage
   "Featured Menu" section as ready to be picked up here, but the Phase 6
   roadmap bullet only mentions the menu page. **Decision: include the
   homepage Featured Menu section (`is_featured` items) in this phase**,
   alongside the new `/menu` route, so the roadmap's "Browse" step is
   complete in one phase rather than leaving a half-finished homepage.

3. **Tests.** No test infrastructure exists anywhere in the repo (no API
   tests, no web tests, no test step in CI). **Decision: add a minimal API
   integration test suite for the two new endpoints** (the repo's first),
   using sqlx's `#[sqlx::test]` (ephemeral, auto-migrated per-test
   database). No frontend tests are added — there is no existing frontend
   test precedent and the UI is simple, server-rendered markup.

4. **CI trigger scope.** `.github/workflows/deploy.yml` triggers only on
   `push: branches: [master]` — there is no `pull_request` trigger anywhere
   in the repo, even though work lands via PRs. **Decision: add a
   `pull_request` trigger** so the new test job also runs on PRs and shows
   pass/fail before merge, while `build-and-push`/`deploy` stay gated to
   `push` events only (PRs never build/push images or deploy).

Additional implementation decisions (not genuine forks — sensible,
reversible defaults, not put to the user, though two were independently
reached the same way by separate backend/frontend design passes, which is
a good signal they're correct):

5. **"Available" wire contract for `GET /api/menu-items`.** The roadmap
   says "list available menu items," which taken literally would mean
   filtering to `is_available = true` server-side. But the same roadmap
   entry also requires an "unavailable-item state" on the menu page, and
   Phase 7 requires blocking add-to-cart for unavailable items — neither is
   implementable if the API silently drops unavailable rows. **Resolution:
   `GET /api/menu-items` returns all non-soft-deleted items, including
   unavailable ones, with `is_available` exposed on each row.** "Available"
   in the roadmap bullet is read as "the orderable-in-principle set" (i.e.
   not soft-deleted), not literally `is_available = true` only. Filtering
   `deleted_at IS NULL` still applies to both endpoints — soft-deleted
   items/categories never appear in either response, available or not.

6. **Data freshness.** Server components fetch with `cache: "no-store"` /
   `force-dynamic` — always fresh on every request, no ISR/revalidate
   window — since there is no cache-invalidation mechanism yet and a
   future admin (Phase 11) will edit menu data live.

7. **Category filtering UI.** A URL query param (`/menu?category=<id>`)
   read via `searchParams` in the server component, rather than a path
   segment or client-side filtering of an already-fetched full list. This
   keeps every filtered view a real, crawlable, shareable URL with zero
   client JS, and exercises the API's own `category_id` filter directly.

8. **Response envelope.** Both new endpoints return a plain JSON array, not
   a `{ "data": [...] }` wrapper — there's no pagination or metadata need
   at this phase's scale (a handful of categories/items).

## Addendum — Real photos/copy for 4 seed items

The user supplied real photos and descriptions for four dishes (Chin Chin,
Efo Soup, Egusi Soup, Small Chops). `apps/api/seed.sql` was updated to use
them instead of generic `placehold.co` placeholders/copy: a new **Soups**
category (Efo Soup, Egusi Soup — neither fit Rice Dishes/Snacks/Drinks),
Small Chops added to Snacks, and Chin Chin's existing row updated in place.
The four JPEGs live under the new `apps/web/public/images/menu/` directory
and are referenced from `seed.sql` by root-relative path (e.g.
`/images/menu/chin-chin.jpeg`) — same mechanism as the remaining
`placehold.co` URLs, just same-origin instead of remote, so no
`next.config.ts` change was needed for these specific items. This is a
content update to the fixture data only; it does not pull Phase 11's
MinIO photo-upload feature forward — see "Out of scope" below, unchanged.
Egusi Soup, Efo Soup, and Small Chops are seeded with `is_featured = true`
(Chin Chin stays `false`, matching its prior state).

## Out of scope

- Menu/category **management** (create/edit/archive) — Phase 11 (Admin:
  Menu & Categories).
- Real photo upload/MinIO wiring for `image_url` — also Phase 11. This
  phase only renders whatever URL already exists in the database.
- Cart / add-to-cart interactivity — Phase 7. Unavailable items are shown
  visually disabled with no actionable button, since no cart exists yet.
- Any admin-facing UI.

## Open risks flagged during planning

1. Decision 5 (exposing unavailable items on the wire) is the single
   highest-impact interpretation call in this phase — it shapes the
   contract Phase 7's cart and Phase 11's admin build against. Worth a
   final sanity check at the start of implementation, even though the
   reasoning (the roadmap's own "unavailable-item state" and "cannot be
   added to cart" bullets) seems to leave no simpler correct reading.
2. No schema validation (e.g. `zod`) is introduced on the frontend's API
   response boundary — `response.json() as T` trusts the backend shape
   completely. Acceptable for a zero-new-dependency read-only phase; worth
   reconsidering once Phase 8 (order submission) raises the stakes of a
   mismatched payload.
3. `price` is serialized as a JSON **string** (via
   `rust_decimal::serde::str`) to avoid float-rounding risk for money,
   matching why Phase 4 chose `NUMERIC(10,2)` in the first place — the
   frontend's `MenuItem.price` type and `formatPrice` helper depend on this
   and will need updating if that serialization choice ever changes.
4. `ORDER BY id` is the simplest stable ordering for both endpoints, since
   no `display_order` column exists on `categories`/`menu_items`. If a
   later phase wants manual reordering, that's a schema addition, not a
   redesign of these endpoints.
5. Without adding `/menu` to the header nav and footer (not mentioned in
   the roadmap bullet but included here as part of Group 15 of `plan.md`),
   the new route would be a real but undiscoverable page — treated as
   necessarily in-scope rather than a follow-up.
