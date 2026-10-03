# Phase 8 — Telemetry & Observability: Validation

## Pass/fail checklist

- [x] `cd apps/api && SQLX_OFFLINE=true cargo check --all-targets` — clean.
- [x] `cd apps/api && SQLX_OFFLINE=true cargo clippy --all-targets` — clean, zero warnings.
- [x] `cd apps/api && cargo test` (against the local dev Postgres) — 8/8 pass, unchanged from Phase 7 (no query/schema changes this phase).
- [x] `cd apps/api && cargo build apps/api` (release, via `docker build apps/api`) — compiles cleanly in release mode too.
- [x] Structured request logging, live against the dev stack: `curl
      http://localhost:8080/api/categories` → `docker compose logs api`
      shows one JSON line (`finished processing request`, `status`,
      `latency`) inside a span carrying `method`/`uri`; the same
      `x-request-id` appears in both the span and the HTTP response
      header (`curl -i`).
- [x] DB-down → 503 mapping: `docker compose stop postgres`, hit
      `/api/categories` → `503 {"error":"service_unavailable"}` plus a
      `tracing::error!` "request failed" line (`sqlx::Error::Io` branch).
      `docker compose start postgres` → confirmed immediate recovery
      (`200` with the categories payload).
- [x] Panic hook: temporarily added a `/api/__panic_test` route that
      `panic!()`s (not committed — added, tested, fully reverted;
      confirmed via `git diff --stat` showing zero diff on
      `routes/mod.rs` afterward). Hitting it produced one structured
      `{"level":"ERROR",...,"message":"panicked",...}` line with the
      panic message and the request's span context. `/health` right
      after confirmed the server itself stayed up — Tokio isolates a
      handler panic to that request's task, it doesn't crash the
      process.
- [x] Fatal startup path: ran the compiled binary directly with
      `SQLX_OFFLINE=true DATABASE_URL=postgres://wrong:wrong@localhost:1/doesnotexist`
      → one structured `"failed to connect to Postgres"` error line,
      then a clean non-zero exit (no raw panic/backtrace).
- [x] `Caddyfile` syntax: `caddy validate` (run via the `caddy:2-alpine`
      image) → `Valid configuration`.
- [x] `docker compose -f docker-compose.yml -f docker-compose.prod.yml
      config` — merges cleanly: every service (`api`, `web`, `caddy`,
      `postgres`, `minio`, `dozzle`) shows `restart: unless-stopped` and
      the bounded `json-file` logging block; `api` carries
      `RUST_LOG`/`LOG_FORMAT`; `dozzle` present with the read-only
      `docker.sock` mount and no published port; `caddy`'s `depends_on`
      includes `dozzle`.
- [x] End-to-end against an isolated local stack (production API image
      built via `docker build apps/api`, plus `caddy`/`dozzle`, DOMAIN
      overridden to `localhost` to avoid any real ACME/DNS calls to the
      production domain):
  - `GET https://api.localhost/api/categories` through Caddy → `200`,
    `x-request-id` present, `via: 1.1 Caddy`.
  - `GET https://logs.localhost/` unauthenticated → `401` with
    `www-authenticate: Basic`; with credentials → `200`, Dozzle UI loads.
  - Stopped the `api` container, hit `https://api.localhost/api/categories`
    → `502`, and Caddy's own JSON access/error log recorded a timestamped
    `"status":502` line (`err_trace: reverseproxy.statusError`) while the
    `api` container's own log stream was silent for that window —
    confirms the cross-correlation story this phase was built for.
- [ ] **Restart-policy auto-recovery could not be demonstrated in this
      sandbox.** `docker kill`-ing the `api` container (and, as a sanity
      check, a bare `docker run --restart unless-stopped alpine sleep
      300`, killed and polled for 10s) never respawned —
      `RestartCount` stayed `0` throughout. `docker compose config`
      confirms the YAML itself is correct (`restart: unless-stopped` on
      every prod service, which is standard, well-documented Docker
      Engine behavior) — this looks like a sandbox-level restriction on
      the restart-policy supervisor, not a bug in this phase's config.
      **Needs a real spot-check once this deploys**: SSH into the
      Lightsail box after deploy, `docker kill` the `api` container, and
      confirm it comes back without manual intervention.
- [x] `apps/web`: `pnpm lint` and `npx tsc --noEmit` clean on all changed
      files; `pnpm build` succeeds.
- [x] `global-error.tsx` round-trip: temporarily added a deliberate
      `throw` in `app/[locale]/page.tsx` (not committed — reverted
      immediately after, confirmed via `git diff --stat` showing zero
      diff), `pnpm build && pnpm start`, requested the page → `500`
      with a matching error `digest` logged server-side
      (`⨯ Error: global-error.tsx verification throw ... digest:
      '3904598221'`), confirming Next's error pipeline correctly routes
      an uncaught render error through the new root boundary. (The
      boundary's own rendered markup only paints in a JS-executing
      browser — standard Next.js behavior for a `"use client"` error
      boundary — so a plain `curl` only shows the matched-digest error
      response, not the final rendered fallback.)
- [x] `grep -n "^## Phase" specs/roadmap.md` — phases run 1–17
      sequentially, no gaps or duplicates.
- [x] `/validate` skill run (full lint + build + live HTTP smoke sweep):
      `apps/web` — `pnpm lint` and `pnpm build` both clean; `apps/api` —
      `cargo check`, `cargo clippy --all-targets`, and `cargo build` all
      clean (zero warnings). Live dev stack: `GET /health` → `200
      {"status":"ok"}`; `GET /` → `307` to `/en`; `GET /en`/`GET /nl` →
      `200` with correct `<html lang>`; `GET /api/categories`/`GET
      /api/menu-items` → `200` (no regressions); `GET /en/menu` → `200`.
- [ ] CI green on PR (same open item every prior phase has left
      unchecked at spec-writing time).

## Definition of done

`apps/api` now has structured, correlatable logging (request ID,
method/path/status/latency, panic capture, DB-error status-code mapping)
instead of a bare `eprintln!`; Caddy logs every request as JSON,
including a timestamped 502 when the upstream is unreachable; Dozzle
gives a Basic-Auth-gated browser view across every container's logs; and
every prod Compose service now auto-restarts (per the YAML — see the one
unchecked item above re: sandbox verification) with bounded log
retention. `apps/web` gained a minimal root error boundary closing the
one remaining silent-failure gap. The next production 502 should be
fully diagnosable from `docker compose logs` or Dozzle alone, without
needing to reproduce it first.
