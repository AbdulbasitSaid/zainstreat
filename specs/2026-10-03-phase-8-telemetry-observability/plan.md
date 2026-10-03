# Phase 8 — Telemetry & Observability: Implementation Plan

Groups 1–4 touch `apps/api`; Group 5 touches `Caddyfile`/Compose files;
Group 6 touches `apps/web`; Group 7 touches docs; Group 8 touches
`specs/roadmap.md` and other live phase-number cross-references.

## Group 0 — Branch

Branched `2026-10-03-phase-8-telemetry-observability` off `master`
(Phase 7 already merged; this phase touches no file unique to any
unmerged branch).

## Group 1 — `apps/api/Cargo.toml`

Added `tracing = "0.1"`, `tracing-subscriber = { version = "0.3",
features = ["env-filter", "json"] }`, `tower-http = { version = "0.6",
features = ["trace", "request-id"] }`, and moved `tower` from
dev-only to a normal dependency (`ServiceBuilder` is needed in
`lib.rs`). Verified against the pinned lockfile versions already in use
(`axum 0.8.9`, `tower 0.5.3`, `http 1.5.0`) — `tower-http 0.6.11`
resolved cleanly, pulling in `uuid 1.27.0` transitively via the
`request-id` feature (no direct `uuid` dependency needed).

## Group 2 — `apps/api/src/main.rs`

- `init_tracing()`: builds an `EnvFilter` from `RUST_LOG` (default
  `"info"`), chooses JSON (default) or pretty output via `LOG_FORMAT`,
  and installs a `std::panic::set_hook` that logs any panic via
  `tracing::error!` before the default unwind behavior.
- Every previous `.expect()`/`.unwrap()` (`DATABASE_URL` read, Postgres
  connect, migrations, `TcpListener::bind`, `axum::serve`) replaced with
  an explicit match/if-let that logs a structured `tracing::error!` line
  and calls `std::process::exit(1)` on failure — same fatal behavior,
  now with a diagnostic trail instead of raw panic text.
- `println!("api listening on 0.0.0.0:{port}")` → `tracing::info!(port,
  "api listening")`.

## Group 3 — `apps/api/src/lib.rs`

`build_app` now layers `SetRequestIdLayer::x_request_id(MakeRequestUuid)`
→ `TraceLayer::new_for_http()` (span + on-response both at `INFO`,
latency in milliseconds) → `PropagateRequestIdLayer::x_request_id()`
through a single `tower::ServiceBuilder`, so ordering is correct
(request ID assigned before the trace span is created, then propagated
onto the response). `/health` is not exempted from logging.

## Group 4 — `apps/api/src/error.rs`

`AppError::into_response` now matches on the `sqlx::Error` variant:
`RowNotFound` → 404, `PoolTimedOut`/`PoolClosed`/`Io(_)` → 503,
`Database` with code `23505`/`23503` → 409, code `23514` → 400,
everything else → 500. Logs via `tracing::error!` (5xx) or
`tracing::warn!` (4xx) instead of the old `eprintln!`; because this runs
inside the request's tracing span, the log line automatically carries
method/path/request-id context with no extra plumbing.

## Group 5 — Infra: Caddyfile, docker-compose.yml, docker-compose.prod.yml, .env.example

- `Caddyfile`: global `log { format json }` default, plus an explicit
  `log { format json }` in each of the three existing site blocks, plus
  a new `logs.{$DOMAIN}` block (`basic_auth` gated, `reverse_proxy
  dozzle:8080`). Validated with `caddy validate` (ships in the
  `caddy:2-alpine` image) — syntactically valid.
- `docker-compose.yml` (dev): `api` service gets `RUST_LOG:
  ${RUST_LOG:-info,api=debug,tower_http=debug,sqlx=warn}` and
  `LOG_FORMAT: ${LOG_FORMAT:-pretty}`.
- `docker-compose.prod.yml`: added an `x-logging` YAML anchor
  (`json-file`, `max-size: 10m`, `max-file: 3`) merged (`<<: *default-logging`)
  into every service; added `restart: unless-stopped` to every service;
  added a `dozzle` service (`amir20/dozzle:latest`, read-only
  `docker.sock` mount, no published port); added `RUST_LOG`/`LOG_FORMAT`
  to `api`'s `environment:` block; `caddy`'s `depends_on` now includes
  `dozzle`. Verified with `docker compose -f docker-compose.yml -f
  docker-compose.prod.yml config` — merges cleanly, every service shows
  the logging/restart keys, `dozzle` present.
- `.env.example`: added `RUST_LOG`, `LOG_FORMAT`,
  `DOZZLE_BASIC_AUTH_USER`, `DOZZLE_BASIC_AUTH_HASH` (with a comment
  pointing at `caddy hash-password` to generate the latter).

## Group 6 — `apps/web/app/global-error.tsx`

New file, directly under `app/` (not `app/[locale]/`, since Next's root
error boundary defines its own `<html>`/`<body>` and there is no root
`app/layout.tsx` to fall back to). `"use client"`, imports
`./globals.css`, `console.error(error)`s and renders a minimal generic
notice plus a "Try again" button wired to `reset()`. No next-intl (no
locale context exists at the true root), no SDK.

## Group 7 — Docs

`docs/deployment.md`: new "Viewing production logs" section (direct SSH
`docker compose logs` path; Dozzle at `https://logs.<domain>`, Basic
Auth-gated, one-time setup noted; `RUST_LOG`/`LOG_FORMAT` tunable live via
`.env` + `docker compose ... restart api`). Also fixed a pre-existing
off-by-one in the adjacent "Backups" section (`Phase 16` → `Phase 17`,
see Group 8).

## Group 8 — Roadmap renumbering + live cross-references

`specs/roadmap.md`: inserted "Phase 8 — Telemetry & Observability";
renumbered old Phase 8 (Cart) through old Phase 16 (Production Hardening
& Final Rollout) up to Phase 9–17. Updated in-roadmap cross-references:
Phase 2's two stale self-references (previously read "Phase 15" for both
the Dockerfile-hardening note and the snapshot-restore note — those
actually describe old Phase 16's content, now correctly "Phase 17";
this was itself a pre-existing off-by-one bug, missed by Phase 7's own
renumbering pass, fixed here), Phase 3's "(Phase 10/11)" → "(Phase
11/12)", Phase 5's "Phase 13's job" → "Phase 14's job", old Phase 13's
"Phase-15 email" → "Phase-16 email".

Also updated forward-looking references in current (non-historical)
docs, following the same precedent Phase 7 itself set: `specs/tech-stack.md`
("restore is verified in Phase 16" → "Phase 17"), `docs/deployment.md`
("Phase 16 verifies an actual restore" → "Phase 17"), and
`apps/web/next.config.ts`'s comment ("added alongside this entry in
Phase 12" → "Phase 13", the new Admin: Menu & Categories number).
Historical `specs/<date>-phase-N-*/` folders (1–7) left untouched, per
the same precedent.

## Verification

See `validation.md`.
