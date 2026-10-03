# Phase 4 — Data Model: Requirements

## Source

`specs/roadmap.md`, Phase 4:

> - Postgres migrations for `users`, `categories`, `menu_items`, `orders`,
>   `order_items` (per `tech-stack.md` / README §31).
> - Soft-delete columns (`deleted_at`) on `categories` and `menu_items`.
> - Seed script with a few sample categories/menu items for local development.

## Context

Phases 1 (scaffolding), 2 (AWS deployment), and 3 (Dutch/English i18n) are
merged to `master`. `postgres` already runs in `docker-compose.yml`
(healthy, env-driven), but nothing has ever connected to it — `apps/api`
is still just a `/health` route with no dependencies beyond `axum`,
`tokio`, `serde`. `tech-stack.md` already pins `sqlx` as the
compile-time-checked query layer, but no migration, no schema, and no
`DATABASE_URL` exist yet.

This phase has to land before Phase 6 (Menu Browsing), which is the first
phase to actually query `categories`/`menu_items`, and before Phase 8
(Order Submission), which writes `orders`/`order_items`. Phase 5 (Public
Static Pages) has no DB dependency, so strictly it could go first, but the
roadmap orders Data Model directly after i18n so the schema — and the
sqlx/CI wiring it requires — exists before any phase needs to write a
real query against it.

## Decisions

1. **Branch:** `2026-10-02-phase-4-data-model` off `master`.
2. **Schema:** exactly the five tables and columns in README §31 —
   `users`, `categories`, `menu_items`, `orders`, `order_items` — no
   additional tables this phase.
3. **Primary keys:** `BIGSERIAL`, not UUID. README §18/§27 display order
   numbers as short human-readable codes (`Order #ZT-000123`,
   `#ZT-001`/`#ZT-002` in the admin order list) — a sequential integer
   supports that directly; UUIDs would need a second sequence just for
   display. All five tables use the same convention for consistency.
4. **Soft deletes:** `deleted_at TIMESTAMPTZ NULL`, only on `categories`
   and `menu_items`, per the roadmap bullet and the archive-not-delete
   non-negotiable rule in `mission.md`. `users`, `orders`, `order_items`
   are not soft-deleted — out of scope for any phase on the current
   roadmap.
5. **`users.role`:** `TEXT NOT NULL DEFAULT 'admin'`, no CHECK constraint,
   no roles table. README §26 explicitly defers "role-based permissions"
   and "multiple staff accounts" to Future — a single implicit role is
   all the MVP needs; adding real RBAC later is a column/table addition,
   not a redesign.
6. **Order status:** `status TEXT NOT NULL DEFAULT 'new'` with a `CHECK
   (status IN ('new','confirmed','preparing','ready','completed','cancelled'))`
   constraint, per README §19's internal status list. Chosen over a native
   Postgres `ENUM` type — a `CHECK` constraint can be redefined with a
   plain `ALTER TABLE ... DROP CONSTRAINT / ADD CONSTRAINT` migration;
   widening a Postgres `ENUM` has transactional restrictions that don't
   fit this project's lightweight migration style.
7. **Delivery type:** `delivery_type TEXT NOT NULL CHECK (delivery_type IN
   ('pickup','delivery'))`, per the Pickup/Delivery choice in README §17.
8. **Money columns:** `NUMERIC(10,2)` for `menu_items.price`,
   `orders.subtotal`/`delivery_fee`/`total`, and
   `order_items.unit_price`/`subtotal` — avoids floating-point rounding
   error for currency and is the conventional choice for a plain
   relational schema (no ORM decimal-type gymnastics to work around).
9. **No stored `order_number` column.** The `ZT-000123` display format in
   README §18/§27 is computed at render time from `orders.id` (e.g.
   `format!("ZT-{:06}", id)`) when Phase 8 (Order Submission) and Phase 10
   (Admin: Orders) build their UI — not stored. Keeps this phase's schema
   exactly what README §31 lists, with no speculative column.
10. **`updated_at` is application-maintained, not trigger-maintained.** No
    `BEFORE UPDATE` triggers. Whatever phase first issues `UPDATE`
    queries against these tables (Phase 9 argon2/sessions touching
    `users`, Phase 11 admin menu management touching `menu_items`/
    `categories`, etc.) is responsible for setting `updated_at = now()`
    explicitly in that query. Keeps migrations plain SQL with no
    procedural logic, matching the "no ORM magic" / straightforward
    relational model already decided in `tech-stack.md`.
11. **Foreign keys:** `menu_items.category_id → categories.id ON DELETE
    RESTRICT` (a category with menu items can't be hard-deleted — it must
    be archived via `deleted_at` instead, matching decision 4).
    `order_items.order_id → orders.id ON DELETE CASCADE` (order items have
    no independent existence). `order_items.menu_item_id → menu_items.id
    ON DELETE SET NULL`, nullable — `order_items` already denormalizes
    `item_name`/`unit_price` for historical accuracy (the non-negotiable
    rule in `mission.md`), so the live FK is a convenience link, not the
    source of truth, and must not block or cascade if a menu item row is
    ever removed.
12. **Migration tooling: sqlx's built-in migrator.** Plain numbered `.sql`
    files under `apps/api/migrations/`, embedded into the binary via
    `sqlx::migrate!()` and run automatically against the database on API
    startup, before the HTTP listener binds. Fits this project's
    single-VPS, no-manual-approval continuous deployment (Phase 2) — no
    separate CI/deploy migration step to add or maintain.
13. **sqlx offline mode for CI.** `sqlx::query!`/`query_as!` macros need a
    live DB connection (or a cached query catalog) at compile time.
    `apps/api/Dockerfile`'s `cargo build --release` stage has no network
    access to Postgres, so this phase adopts sqlx's offline mode: generate
    `.sqlx/` query metadata locally with `cargo sqlx prepare` (run against
    the dev stack's Postgres), commit that directory, and set
    `ENV SQLX_OFFLINE=true` in the Dockerfile's build stage. This is a
    hard requirement for Phase 2's existing Docker-build CI step to keep
    working once Phase 6+ adds real `sqlx::query!` calls.
14. **`DATABASE_URL`:** composed in `docker-compose.yml`'s `api` service
    from the existing `POSTGRES_USER`/`POSTGRES_PASSWORD`/`POSTGRES_DB`
    env vars (`postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@postgres:5432/$POSTGRES_DB`),
    read in `main.rs` via `std::env::var("DATABASE_URL")` — same pattern
    already used for `PORT`. No new secrets, no `dotenvy` crate.
15. **`/health` becomes DB-aware.** Once a pool exists, `/health` runs a
    trivial `SELECT 1` against it and returns `503` if that fails (still
    `200` + `{"status":"ok"}` when healthy) — a cheap, directly
    phase-relevant way to prove the API can actually reach Postgres,
    matching `roadmap.md`'s top-level principle that "each phase should
    produce something that can be run and checked on its own."
16. **Seed data:** a plain idempotent SQL file, `apps/api/seed.sql` — not
    a numbered migration, since it's fixture data, not schema. Run
    manually via `psql` for local development only; never applied in
    production. A handful of sample categories (e.g. Rice Dishes, Snacks,
    Drinks) and menu items (e.g. Jollof Rice, Fried Rice, Chicken Suya,
    Puff Puff, Meat Pie, Chin Chin — in the spirit of README's own
    "Jollof Rice = ₦6,000" example).

    **Update (2026-10-03):** superseded by Phase 7, which replaced this
    placeholder fixture content with the real client menu. With real menu
    data (rather than placeholder Naira examples) in `seed.sql`, it is now
    also run against production — manually once, then via the
    `.github/workflows/seed-production.yml` CI workflow going forward —
    since no admin CMS exists yet (Phase 13) to populate the live menu any
    other way. See `docs/deployment.md`'s "Seeding the production database"
    section.
17. **Seed `image_url` values use an external placeholder image service**
    (e.g. `https://placehold.co/...`), not real uploaded photos. Every
    seeded menu item gets a non-null `image_url` so Phase 6 (Menu
    Browsing) has something to render immediately, without this phase
    needing to add any asset files under `apps/web/public` — that would
    be out of scope here (Phase 4 touches `apps/api` and `specs/` only;
    see `validation.md`'s diff-scope check). Real photo upload/storage via
    MinIO is Phase 11's job (Admin: Menu & Categories); seed placeholders
    are a dev-only stand-in, not a storage decision.

## Out of scope (explicitly deferred)

- Any menu/order/admin API routes beyond the `/health` DB check in
  decision 15 — CRUD endpoints are Phases 6 (menu, read-only), 8 (order
  submission), 10–11 (admin menu/order management).
- Admin authentication, sessions, `argon2` — Phase 9.
- Real production data — `seed.sql` is dev-only, never run against the
  production database. **(Superseded 2026-10-03 — see decision 16's update
  above.)**
- The `ZT-000123` order-number display format — computed later (decision
  9), not stored now.
- Role-based access control / multiple staff accounts — explicitly Future
  scope per README §26, not MVP.
- Catering/contact enquiry storage — Phase 12 owns that; it isn't one of
  README §31's five tables and isn't added here.

## Open risks flagged during planning

- **Auto-migrate-on-boot risk:** running migrations automatically on every
  API container start means a bad migration in a later phase could crash
  the whole service on deploy with no automated rollback. Acceptable for
  a pre-launch MVP with no test suite and no manual approval gate (Phase
  2's own stated philosophy), but worth revisiting in Phase 15
  (Production Hardening) once real data and traffic exist.
- **`.sqlx/` cache maintenance:** every future query change requires
  re-running `cargo sqlx prepare` and committing the updated `.sqlx/`
  directory, or the Docker build will fail in CI with a stale-cache error.
  This is a new manual step for whoever implements Phase 6 (the first
  phase to add real `sqlx::query!` calls) — flagged here so it isn't a
  surprise there.
- **`sqlx-cli` is a local dev tool, not a project dependency** — it won't
  appear in `Cargo.toml`; whoever implements this phase needs
  `cargo install sqlx-cli` locally to run `cargo sqlx prepare` /
  `sqlx migrate add`.
