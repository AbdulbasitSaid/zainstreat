# Phase 11 — Admin Auth: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 11 (unchanged by
this phase):

> - Login page.
> - API: argon2 password verification, session creation via
>   `tower-sessions` (Postgres-backed), logout.
> - Middleware protecting all `/admin` routes and admin API endpoints.

`specs/tech-stack.md`'s "Admin Authentication (Rust-native)" section already
pins the crate choices (`argon2`, `tower-sessions` + a Postgres-backed
store) and the reasoning (auth lives entirely in the Rust service, no
Node-based auth library). This phase is where that pin becomes real code.

## Context

Every prior phase's write path (`POST /api/orders`, Phase 10) is public and
unauthenticated. This is the first phase that needs a concept of "who is
making this request" at all. `users` already has the columns this needs —
it was created in the very first migration
(`20261002162443_initial_schema.up.sql`):

```sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

No migration since has touched it, and nothing has ever populated it — there
is no seed script, CLI, or admin UI that creates a `users` row today. This
phase has to solve that gap as well as login/logout/middleware, or the
login page would have no account to ever authenticate against.

`apps/api/seed.sql` (the existing dev/prod menu-data seed) is explicitly a
`TRUNCATE ... CASCADE`-and-replace script for `categories`/`menu_items`/
`menu_item_price_options` (`docs/deployment.md`'s "Seeding the production
database" section) — it must never touch `users`, or every re-seed would
wipe admin accounts.

Branches off `master` (Phase 10 is already merged; this phase touches no
file unique to any other unmerged branch).

## Decisions

1. **First admin account creation: a CLI subcommand, `apps/api/src/bin/
   create_admin.rs`.** Decided with the user (over a committed
   `seed-admin.sql` with a hardcoded hash, and over env-var auto-bootstrap
   on startup). Reasons: a hash committed to git (even for a
   change-immediately password) sits in history forever and is awkward to
   rotate; env-var bootstrap threads a real password through compose
   secrets and only ever fires once. The CLI is reusable as-is when Phase
   12+ needs to add a second staff account, and it's a small, ordinary Rust
   binary — no new framework. It's a manual, one-time-per-account
   operational step, run via SSH as the **admin** user (not the sudo-less
   `deploy` CI user — see `docs/deployment.md`'s existing
   manual/break-glass access distinction from Phase 2), never through CI.

2. **Session expiry: a rolling 7-day inactivity window** —
   `tower_sessions::Expiry::OnInactivity(Duration::days(7))`. Decided with
   the user (over a fixed 7-day absolute expiry, and over a short 30-minute
   session). Fits how an owner/small staff actually use a dashboard —
   on and off through the week, not constantly — without the security
   cost of a session that never expires from use alone.

3. **Session store: `tower-sessions-sqlx-store`'s `PostgresStore`, self-
   migrating via its own `.migrate()` call at API startup** — not a
   committed `apps/api/migrations/*.sql` file. This is a deliberate, scoped
   exception to `tech-stack.md`'s "plain numbered `.sql` files, embedded via
   `sqlx::migrate!()`" convention: the session table's schema is owned and
   versioned by the `tower-sessions-sqlx-store` crate itself, not by this
   project, so hand-writing a migration for it would just be duplicating
   (and risking drift from) what the crate already does correctly. `
   apps/api/lib.rs::build_app` becomes fallible (`Result<Router, sqlx::
   Error>`) so this startup step fails the same controlled way
   `sqlx::migrate!()` already does in `main.rs` — no new `.expect()`/panic
   path, consistent with Phase 8's "replace panicking `.expect()`/`.unwrap()`
   with structured logging + controlled exit" fix.

4. **No `AppState` struct introduced.** `tower_sessions::SessionManagerLayer`
   is a plain `tower` `Layer` wrapping the whole `Router`, and its
   `Session` extractor reads from request extensions the layer sets — it
   needs nothing from router `State`. Every existing handler's
   `State<PgPool>` extraction in `categories.rs`/`menu_items.rs`/
   `orders.rs`/`health.rs` is untouched.

5. **New route group `/api/admin/*`**, nested inside the existing
   `api_router()`: `POST /api/admin/login` (public), and
   `GET /api/admin/me` + `POST /api/admin/logout` behind a `require_admin`
   middleware (`axum::middleware::from_fn`) applied via `.route_layer()` to
   just that protected sub-router. Phases 12/13's admin CRUD endpoints
   (orders, menu, categories) nest into this same protected group later and
   inherit the middleware for free — this phase builds the group
   specifically so that's a route addition, not a new auth wire-up, next
   time.

6. **Generic, timing-equalized login failure.** Both "no such email" and
   "wrong password" return the same `401 invalid_credentials` response,
   never revealing which was the problem. When the email isn't found, the
   handler still runs `Argon2::verify_password` against a fixed dummy
   argon2id hash (a real PHC-format hash generated once against an
   arbitrary password, hardcoded as a constant) instead of short-circuiting
   immediately — keeping the "no such user" and "wrong password" code paths
   roughly the same shape/cost, rather than letting a fast early-return
   leak which emails have accounts via response timing.

7. **Cookie attributes: `HttpOnly` (`tower-sessions` default), `SameSite=
   Lax`, and `Secure` toggled by a new `COOKIE_SECURE` env var** (default
   `false` in `docker-compose.yml`, default `true` in
   `docker-compose.prod.yml`) — the same per-compose-file-default
   convention Phase 8 already established for `LOG_FORMAT` (`pretty` in
   dev, `json` in prod). `Secure` must be off in dev (plain `http://
   localhost`) and on in prod (Caddy terminates real HTTPS).

8. **`apps/web`'s admin surface lives outside the locale-prefix scheme**:
   `apps/web/app/admin/**`, a sibling top-level segment to `app/[locale]/`
   — consistent with `tech-stack.md`'s existing "the admin dashboard ...
   stay[s] English-only" decision (Phase 3). Since there is no shared
   `app/layout.tsx` above `app/[locale]/` today (it's already a root layout
   defining its own `<html>`/`<body>`), `app/admin/layout.tsx` is a second,
   independent root layout (Next.js's "multiple root layouts" pattern) —
   its own minimal `<html lang="en">`/`<body>`, no `next-intl`/
   `CartProvider`/site header-footer. `apps/web/proxy.ts`'s next-intl
   middleware matcher is updated to also exclude `/admin`, or it would try
   to locale-redirect `/admin/login` → `/en/admin/login`.

9. **Auth-gating on the Next.js side is a Server Component layout, not
   global middleware.** `app/admin/(protected)/layout.tsx` calls
   `GET /api/admin/me` directly against `API_BASE_URL` (server-to-server,
   same pattern `lib/api.ts` already uses for menu/category reads),
   forwarding the incoming request's `Cookie` header, and `redirect()`s to
   `/admin/login` on anything other than `200`. The `(protected)` route
   group covers exactly one real page this phase: a bare placeholder
   landing page (`/admin` — "Welcome, `{name}`" + a logout button). The
   actual dashboard shell/nav is explicitly Phase 12's job (`roadmap.md`);
   this phase deliberately builds no more admin UI than proves the
   login/logout/middleware loop works end to end, the same minimal-stub
   precedent Phase 9 left for Phase 10 to build on.

10. **Login and logout submissions proxy through same-origin Next.js Route
    Handlers** (`app/api/admin/login/route.ts`,
    `app/api/admin/logout/route.ts`), mirroring Phase 10 Decision 7's
    "browser never talks to `apps/api` directly" rule. Unlike Phase 10's
    `app/api/orders/route.ts` — which this phase's research confirmed
    forwards neither `Set-Cookie` nor `Cookie` — these two new proxies
    explicitly forward `Set-Cookie` (API response → browser) and `Cookie`
    (browser request → API call), since dropping either would silently
    break the whole session. `GET /api/admin/me` does **not** need its own
    proxy route — it's only ever called server-to-server from the Next.js
    layout (Decision 9), never from the browser.

11. **Minimal field validation only** — both the login form and the API
    handler require non-empty `email`/`password` and nothing more (no
    email-shape regex; a malformed email simply won't match any `users`
    row and falls into the generic `invalid_credentials` case). Consistent
    with Phase 10 Decision 10's precedent; `roadmap.md` Phase 16 owns
    deeper hardening.

12. **`apps/api/tests/common.rs` introduced**, extracting the `get`/`post`
    `oneshot` helpers `menu_browsing.rs` and `order_submission.rs` each
    currently duplicate, plus new cookie-aware variants
    (`post_with_cookie`/`get_with_cookie`) this phase's login→me→logout
    flow needs. The two existing test files are refactored to use the
    shared helper instead of their own copies. Phase 10's plan.md
    explicitly flagged this exact duplication as "fine for now, follow
    whichever reads cleaner once both exist" — a third near-identical copy
    for this phase is the point where sharing one stops being premature.

13. **Backend gets integration tests** (`apps/api/tests/admin_auth.rs`).
    **The frontend continues to have none** — validated manually via
    `/validate`'s live smoke test instead, same precedent as every prior
    phase.

## Out of scope

- Dashboard shell/nav (`Dashboard`, `Orders`, `Menu`, `Categories`,
  `Settings`) — `roadmap.md` Phase 12.
- Any admin-facing CRUD (orders, menu items, categories) — Phases 12/13.
- Multiple staff accounts, granular roles beyond the single `role` column's
  `'admin'` default, 2FA, password reset — `tech-stack.md`'s "Future items
  from README §26 ... extend this same session model later; they are not
  MVP scope."
- Rate-limiting/lockout/CAPTCHA on login attempts — open risk below.
- Re-wiring the pre-provisioned `api.{$DOMAIN}` Caddy vhost for direct
  browser traffic — still unused, per Phase 10 Decision 7 / open risk 4;
  admin login/logout proxy through `web` exactly like `orders` does.

## Open risks flagged during planning

1. **No rate-limiting or lockout on `POST /api/admin/login`.** A public,
   unauthenticated endpoint with no throttling is brute-forceable. Same
   posture Phase 10 already accepted for `POST /api/orders`; worth
   revisiting in Phase 16 or a dedicated hardening follow-up if abuse shows
   up in Phase 8's logs.
2. **The dummy-hash timing mitigation (Decision 6) is best-effort, not a
   rigorous constant-time guarantee** — network jitter alone likely
   dominates any remaining timing signal. Acceptable for MVP; a real
   user-enumeration concern would need more than this.
3. **The `create_admin` CLI has no built-in safeguard against being pointed
   at the wrong database** — nothing stops someone from running it against
   a production `DATABASE_URL` from a laptop by accident, or vice versa.
   Relies entirely on operator discipline (the same posture
   `docs/deployment.md`'s existing manual/break-glass runbook already
   assumes for admin SSH access). Worth a confirmation prompt later if a
   near-miss ever happens.
4. **`tower-sessions`'s self-migrated session table sits outside this
   project's own migration history** (Decision 3) — if that crate is ever
   replaced, cleaning up its table is a manual follow-up, not a `down.sql`.
