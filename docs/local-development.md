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

## Internationalization (Dutch/English)

The public site is served under locale-prefixed routes (`/en`, `/nl`),
powered by `next-intl`. Conventions for adding UI copy in future phases:

- Translation strings live in `apps/web/messages/en.json` and
  `apps/web/messages/nl.json`, keyed by `PageName.keyName` (e.g.
  `HomePage.title`). Add a key to both files together — never ship an
  English-only key.
- `apps/web/components/locale-toggle.tsx` is a placeholder control
  currently rendered on the hello page; once a real header/nav exists
  (Phase 5), relocate it there instead of adding a second toggle.
- The admin dashboard and the Rust API (`apps/api`) are out of scope for
  translation — they stay English-only.
