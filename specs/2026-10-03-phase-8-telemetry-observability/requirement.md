# Phase 8 — Telemetry & Observability: Requirements

## Source

Not an original roadmap phase — inserted after production returned its
first 502, in response to discovering there was no way to see why.
`specs/roadmap.md`, Phase 8 (as written by this phase):

> Not an original roadmap phase — inserted after the site's first
> production 502 revealed there was no way to see why. Self-hosted only
> (no external SaaS), consistent with the single-VPS hosting decision in
> `tech-stack.md`.
>
> - `apps/api`: structured logging (`tracing` + `tracing-subscriber`),
>   request-level logging with latency and a request ID
>   (`tower_http::trace::TraceLayer` + `request_id`), and `sqlx::Error`
>   variants mapped to distinct HTTP status codes (404/409/500/503)
>   instead of everything collapsing to a bare 500.
> - Replace the `eprintln!`/panicking `.expect()`/`.unwrap()` calls in
>   `main.rs`/`error.rs` with structured `tracing::error!` logging before a
>   controlled exit, plus a panic hook.
> - Caddy: JSON access/error logs on every site block, so a 502 (upstream
>   unreachable) is timestamped and correlatable with the API's own logs.
> - Dozzle (self-hosted, browser-based log viewer) added to
>   `docker-compose.prod.yml`, reverse-proxied through Caddy and gated by
>   HTTP Basic Auth (it can read every container's logs, so it isn't
>   exposed directly).
> - `restart: unless-stopped` and bounded `json-file` log retention
>   (`max-size`/`max-file`) added to every prod Compose service — a
>   crashed `api` container currently stays down until someone manually
>   SSHes in, which directly compounds a 502.
> - `docs/deployment.md`: new "Viewing production logs" section.

## Context

Production returned a 502. Investigating it surfaced the actual problem:
`apps/api` had zero structured logging (`eprintln!` only, with the
literal comment `// no tracing crate yet` at the old `error.rs:18`),
startup failures panicked via `.expect()`/`.unwrap()` with no context,
and there was no request-level logging at all. Caddy — the reverse proxy
that actually emits a 502 when `api` is unreachable — had no `log`
directive configured either, so even the proxy-side evidence of the
incident wasn't captured anywhere durable.

Separately, `docker-compose.prod.yml` had no `restart:` policy on any
service. If `api` panics (e.g. from one of those `.expect()`/`.unwrap()`
calls), the container simply stays dead — Caddy keeps 502ing until
someone manually SSHes into the Lightsail box and runs `docker compose up
-d` again. This directly compounds the reported bug: even once the cause
is visible, nothing brings the service back automatically.

This phase branches directly off `master` (Phase 7 is already merged, and
this phase doesn't touch any file only present on an unmerged branch).

## Decisions

1. **Self-hosted only, no external SaaS.** `tracing` + `tracing-subscriber`
   + `tower_http::trace::TraceLayer` for the API, JSON access logs for
   Caddy, and Dozzle (a lightweight, self-hosted, browser-based Docker log
   viewer) for convenient viewing — no Sentry, no Datadog, no
   Prometheus/Grafana. Consistent with `tech-stack.md`'s single-VPS
   hosting decision; no new external account or cost.

2. **Dozzle is access-gated through Caddy, not exposed on its own port.**
   Dozzle's read-only `docker.sock` mount lets it read **every**
   container's logs, including `postgres` and `minio` — reachable only
   via a new `logs.{$DOMAIN}` Caddy site block gated by HTTP Basic Auth.
   The Lightsail firewall only opens `22`/`80`/`443` anyway, so this is
   the only way to reach it regardless.

3. **`sqlx::Error` variants now map to distinct status codes** instead of
   every DB error collapsing to a bare 500: `RowNotFound` → 404,
   `PoolTimedOut`/`PoolClosed`/`Io` → 503 (the "DB is the actionable
   bottleneck" signal), unique/FK violations → 409, check violations →
   400, everything else → 500. This is what makes the logs/responses
   actually diagnostic instead of uniformly opaque.

4. **Startup failures stay fatal, but structured.** `DATABASE_URL`
   missing, Postgres connect failure, failed migrations, listener bind
   failure, and a fatal `axum::serve` error all still terminate the
   process (there's no meaningful degraded mode for any of them) — but
   each now logs one structured `tracing::error!` line before a clean
   `std::process::exit(1)`, instead of an unwind panic whose message can
   get truncated or interleaved with other container output. A
   `std::panic::set_hook` also captures any *unexpected* panic
   (e.g. inside a request handler) as a structured log line; Tokio
   isolates a handler panic to that request's task, so the server itself
   keeps running — confirmed live during validation (see
   `validation.md`).

5. **Restart policy: `unless-stopped`, prod only.** Chosen over
   `on-failure` because it also recovers the whole stack after a host
   reboot (Lightsail maintenance, OOM), while still respecting an
   intentional `docker compose stop`. Scoped to
   `docker-compose.prod.yml` only — dev containers run interactively via
   bind-mounted `cargo run`/`pnpm dev`, where an auto-restart loop on a
   broken local edit would just be noisy.

6. **One minimal `apps/web` addition: a root `global-error.tsx`.**
   `apps/web` had zero safety net for an unhandled render crash outside
   `/menu` (the only existing boundary was the route-local
   `app/[locale]/menu/error.tsx`) — the same "silent failure" class of
   problem this phase otherwise fixes on the backend. Added one file,
   `console.error`-only, no SDK, no client-to-server forwarding — kept
   tightly scoped since this phase's primary focus is backend/infra.

## Out of scope

- External error-tracking/APM SaaS (Sentry, Datadog) — explicitly
  decided against; self-hosted only for now.
- Prometheus/Grafana or any metrics (as opposed to logs) stack — out of
  scope for a 2 GB single-VPS MVP; revisit if/when traffic justifies it.
- Enforcing the restart policy in dev (`docker-compose.yml`) —
  deliberately prod-only.
- Graceful shutdown / signal handling in `apps/api` (`axum::serve`'s
  accept loop has no `with_graceful_shutdown`) — unrelated to the 502
  investigation, left for a future phase if needed.
- `web`'s own resilience to a transient `api` outage after its own
  startup — Compose's `depends_on: condition: service_healthy` is only
  evaluated at `up` time, not continuously, so `web` isn't itself
  restarted when `api` crashes and later auto-recovers. Pre-existing gap,
  noted but not fixed here.
- A full client-side error-tracking buildout (breadcrumbs, SDK,
  server-forwarding) for `apps/web` — only the minimal root boundary
  described above is in scope.

## Open risks flagged during planning

1. **Restart-policy auto-recovery could not be demonstrated live in the
   development sandbox used to build this phase** — even a bare
   `docker run --restart unless-stopped alpine sleep 300`, killed and
   polled for 10s, never respawned in that environment (`RestartCount`
   stayed `0`). `docker compose config` confirms the YAML is correct
   (every prod service carries `restart: unless-stopped`, which is
   standard, well-documented Docker Compose/Engine behavior), but the
   actual respawn-after-crash behavior should be spot-checked once this
   deploys to the real Lightsail box (`docker kill` the `api` container,
   confirm it comes back without manual intervention).
2. Dozzle's Basic Auth credential (`DOZZLE_BASIC_AUTH_HASH`) is a
   one-time manual step on the server, not automated by CI/CD — same
   posture as the server's real `.env`, but worth remembering it needs to
   be set before `logs.{$DOMAIN}` works post-deploy.
3. `/health`'s 5s Docker healthcheck interval means ~17k log lines/day
   from that endpoint alone once this ships — acceptable given the
   bounded `json-file` retention (`max-size: 10m`, `max-file: 3` per
   service) added in this same phase, but worth knowing why log volume
   jumps after this deploys.
