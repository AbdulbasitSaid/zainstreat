# Local development

1. `cp .env.example .env`
2. `docker compose up`
3. Check:
   - Web: http://localhost:3000
   - API health: http://localhost:8080/health
   - MinIO console: http://localhost:9001 (login with `MINIO_ROOT_USER` /
     `MINIO_ROOT_PASSWORD` from `.env`)
4. `docker compose down` when done.

## Database

- Migrations run automatically on API container startup — no manual step
  for a normal `docker compose up`.
- To add a new migration: `cd apps/api && sqlx migrate add -r <name>`.
- To regenerate the offline query cache after changing a query:
  `cd apps/api && cargo sqlx prepare` (requires `DATABASE_URL` exported
  and the dev stack running — see the note above `.env.example`'s
  Postgres block), then commit `.sqlx/`.
- To load sample data: `docker compose exec -T postgres psql -U
  $POSTGRES_USER -d $POSTGRES_DB < apps/api/seed.sql` (or the equivalent
  run locally against `localhost:$POSTGRES_PORT`).

## Running API tests

`cargo test` from `apps/api` requires `DATABASE_URL` exported to a
reachable Postgres with `CREATEDB` privilege (the dev stack's `postgres`
service works as-is). Each test gets its own throwaway database via
`#[sqlx::test]`, migrated automatically. Run `cargo sqlx prepare` again
after adding or changing any query, including in `tests/`.

## Web → API data fetching

`apps/web`'s server components call `apps/api` over HTTP using
`API_BASE_URL` (set in `docker-compose.yml`'s `web` service, e.g.
`http://api:8080`) — server-only, not exposed to the browser. Both dev
and prod Compose files share the API's service name (`api`), so this
resolves via Docker's internal DNS in both environments with no override
needed.

## Internationalization (Dutch/English)

The public site is served under locale-prefixed routes (`/en`, `/nl`),
powered by `next-intl`. Conventions for adding UI copy in future phases:

- Translation strings live in `apps/web/messages/en.json` and
  `apps/web/messages/nl.json`, keyed by `PageName.keyName` (e.g.
  `HomePage.heroHeadline`). Add a key to both files together — never ship
  an English-only key. `SiteHeader`, `SiteFooter`, and one `PageName`
  namespace per route (e.g. `AboutPage`, `ServicesPage`, `ContactPage`,
  `MenuPage`) are the established message-key groups going forward
  (Phase 5).
- `apps/web/components/locale-toggle.tsx` now renders inside
  `site-header.tsx` (Phase 5), closing the forward-reference this doc
  previously left open. Don't add a second toggle elsewhere.
- The admin dashboard and the Rust API (`apps/api`) are out of scope for
  translation — they stay English-only.
