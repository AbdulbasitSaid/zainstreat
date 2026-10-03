# Phase 11 — Admin Auth: Validation

## Pass/fail checklist

- [ ] `cd apps/api && cargo test` — all cases pass: `admin_auth.rs` (login
      success + `/me` reflects session, missing-session `/me` is `401
      unauthorized`, wrong password is `401 invalid_credentials`, unknown
      email is `401 invalid_credentials`, logout invalidates the session so
      a subsequent `/me` is `401` again) and the refactored
      `menu_browsing.rs`/`order_submission.rs` (now using
      `tests/common.rs`) still pass unchanged.
- [ ] `cd apps/api && cargo sqlx prepare --check` (or equivalent) — the
      committed `.sqlx/` cache matches the new `admin/auth.rs` and
      `create_admin.rs` queries; no drift.
- [ ] `cd apps/api && cargo build --release --bin create_admin` succeeds —
      confirms it's a real, independently buildable binary target.
- [ ] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new/changed
      file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean, no implicit/explicit `any`.
- [ ] `cd apps/web && pnpm build` — succeeds; `/admin` and `/admin/login`
      appear in the build's route list **without** a locale prefix (not
      `/en/admin`/`/nl/admin`); `app/api/admin/login/route.ts` and
      `app/api/admin/logout/route.ts` appear as server functions.
- [ ] Live dev stack (`docker compose up`), manual smoke test:
  - [ ] Create a local admin account: from `apps/api`, with `DATABASE_URL`
        pointing at the dev Postgres (or via `docker compose exec api
        create_admin --email test@example.com --name "Test Admin"` once
        the image includes the binary), enter a password twice. Confirm a
        row exists via `psql`: `SELECT email, role FROM users;`.
  - [ ] Visiting `/admin` directly while logged out redirects to
        `/admin/login` (no flash of the protected page's content first).
  - [ ] Submitting the login form with an empty email or password shows
        the required-fields error and makes no network request.
  - [ ] Submitting the login form with the wrong password shows "Invalid
        email or password." and stays on `/admin/login` — confirm via
        `psql`/logs that the error doesn't reveal whether the email itself
        was valid.
  - [ ] Submitting the login form with the correct credentials redirects
        to `/admin`, showing "Welcome, `<name>`" and the account's email.
  - [ ] Hard-refresh `/admin` after a successful login — still shows the
        welcome page (confirms the session cookie persisted, not just
        client-side router state).
  - [ ] Click "Log Out" — redirects to `/admin/login`; visiting `/admin`
        again afterward redirects back to `/admin/login` (confirms the
        session was actually invalidated server-side, not just forgotten
        client-side).
  - [ ] Open dev tools → Application → Cookies against `http://
        localhost:3000` after logging in: the session cookie has
        `HttpOnly` set and **not** `Secure` (dev's `COOKIE_SECURE=false`
        default, since this is plain `http://`).
  - [ ] `curl -i -X POST http://localhost:8080/api/admin/login -H
        'Content-Type: application/json' -d '{"email":"nobody@x.com",
        "password":"x"}'` — bypassing `apps/web` entirely, confirms the
        Rust API itself returns `401 {"error":"invalid_credentials"}`
        independent of any client-side checks.
  - [ ] `curl -i http://localhost:8080/api/admin/me` (no cookie) — `401
        {"error":"unauthorized"}`.
  - [ ] Keyboard-only pass through the login form: both inputs and the
        submit button are reachable via Tab in order, have a visible focus
        state and an associated label.
- [ ] Code review: confirm `docker-compose.yml`/`docker-compose.prod.yml`
      carry the new `COOKIE_SECURE` env var with the documented per-file
      defaults, `apps/api/Dockerfile` copies `create_admin` into the final
      image, and `docs/deployment.md` documents the manual admin-creation
      step (Group 21 of `plan.md`) — these are implementation-time edits
      this plan only describes.
- [ ] `/validate` skill run (full lint + build + live HTTP smoke sweep) —
      confirm no regression on existing routes (`/`, `/en/menu`,
      `/en/cart`, `/en/order`, `/api/categories`, `/api/menu-items`,
      `/api/orders`, `/health`).
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

An administrator can log in at `/admin/login` with an email/password
created via the `create_admin` CLI, land on a minimal authenticated `/admin`
page, and log out — all backed by a real Postgres-persisted session
(`tower-sessions` + argon2 verification in `apps/api`), not a client-side
illusion of being logged in. Visiting `/admin` without a valid session
always redirects to the login page, and every protected API endpoint
(`GET /api/admin/me`, `POST /api/admin/logout`, and whatever Phases 12/13
nest alongside them later) rejects requests with no valid session
independent of the frontend, via the same `require_admin` middleware. Login
failures never reveal whether the problem was the email or the password.
No dashboard functionality exists yet — that's Phase 12 — but the
authentication seam it will build behind is fully wired and tested end to
end.
