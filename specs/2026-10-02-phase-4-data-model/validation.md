# Phase 4 — Data Model: Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

- [ ] `docker compose up` starts all four services; `docker compose ps`
      shows `postgres`, `minio`, and `api` as `healthy`.
- [ ] `docker compose logs api` shows the migration running successfully
      on startup, with no panic/`expect` failure.
- [ ] `docker compose exec postgres psql -U $POSTGRES_USER -d $POSTGRES_DB
      -c '\dt'` lists `users`, `categories`, `menu_items`, `orders`,
      `order_items`, and sqlx's own `_sqlx_migrations` bookkeeping table.
- [ ] `docker compose restart api` succeeds a second time without error —
      re-running an already-applied migration is a no-op, not a failure.
- [ ] `curl -i http://localhost:8080/health` returns `200` +
      `{"status":"ok"}` while Postgres is up.
- [ ] Stopping Postgres (`docker compose stop postgres`) and then curling
      `/health` returns a non-`200` status (`503`) rather than the API
      crashing or hanging — confirms the DB-aware health check actually
      checks something. Restart Postgres afterward
      (`docker compose start postgres`).
- [ ] `apps/api/seed.sql` applied against the dev database inserts the
      sample categories/menu items; `SELECT count(*) FROM categories;` and
      `SELECT count(*) FROM menu_items;` return non-zero counts.
- [ ] `SELECT count(*) FROM menu_items WHERE image_url IS NULL;` returns
      `0` — every seeded menu item has a placeholder photo, not a blank.
- [ ] `apps/api/.sqlx/` exists, is committed (not gitignored), and is
      non-empty.
- [ ] `cd apps/api && SQLX_OFFLINE=true cargo build --release` succeeds
      with no network access to Postgres (simulates the Docker build
      environment) — confirms the offline cache actually works, not just
      that it exists.
- [ ] `docker compose build api` (or a full `docker build ./apps/api`)
      succeeds — the production Dockerfile's `cargo build --release`
      stage completes without reaching out to a live database.
- [ ] `apps/api/Cargo.lock` is updated and committed with the new `sqlx`/
      `chrono` dependencies.
- [ ] No menu/order/admin HTTP routes exist beyond `/health` — confirms
      scope stayed to schema + the one health-check enhancement, not
      Phase 6/8/9-11 business logic.
- [ ] `git diff master --stat` touches only `apps/api/**`,
      `docker-compose.yml`, `docs/local-development.md`,
      `specs/tech-stack.md`, and the new
      `specs/2026-10-02-phase-4-data-model/` folder — nothing in
      `apps/web/**`.

## Definition of done

Phase 4 is complete when every box above is checked: the schema from
README §31 exists in Postgres via a reversible sqlx migration that runs
automatically and idempotently on API startup, the offline query cache
lets the production Docker build succeed with no live database, the seed
script populates sample data for local development, and `/health` proves
real DB connectivity rather than just that the process is running. Ready
for Phase 5 (Public Static Pages, no DB dependency) and Phase 6 (Menu
Browsing, the first phase to query this schema) to proceed.
