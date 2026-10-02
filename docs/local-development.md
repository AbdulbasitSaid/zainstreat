# Local development

1. `cp .env.example .env`
2. `docker compose up`
3. Check:
   - Web: http://localhost:3000
   - API health: http://localhost:8080/health
   - MinIO console: http://localhost:9001 (login with `MINIO_ROOT_USER` /
     `MINIO_ROOT_PASSWORD` from `.env`)
4. `docker compose down` when done.

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
