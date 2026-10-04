# Phase 13 — Admin: Orders: Validation

## Pass/fail checklist

### Backend

- [ ] `cd apps/api && cargo test` — all cases in the new
      `tests/admin_orders.rs` pass (auth gating on all three endpoints,
      newest-first list with the `{orders,total,limit,offset}` envelope,
      status filtering, pagination, rejected `status`/`limit` values,
      detail with items and `option_label`, `404` on an unknown id, a
      status update that bumps `updated_at`, a backward status move that
      is **allowed**, and a summary with all six status keys present),
      and `admin_auth.rs` / `menu_browsing.rs` / `order_submission.rs`
      still pass unchanged after `insert_admin` moves into
      `tests/common.rs`.
- [ ] `cd apps/api && cargo clippy --all-targets -- -D warnings` — clean.
- [ ] `cd apps/api && cargo sqlx prepare --check` — the committed
      `.sqlx/` cache covers the six new queries in
      `routes/admin/orders.rs`; no drift.
- [ ] `git diff --stat apps/api/migrations/` is empty — this phase adds
      no migration (requirement.md Decision 15).
- [ ] Confirm by reading `routes/admin/mod.rs` that all four new routes
      sit inside the `protected` sub-router, above
      `.route_layer(from_fn(middleware::require_admin))`, and that
      `/login` is still the only unauthenticated admin route.

### Frontend build

- [ ] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean.
- [ ] `cd apps/web && pnpm build` — succeeds; the route list shows
      `/admin`, `/admin/orders`, `/admin/orders/[id]` and
      `/api/admin/orders/[id]/status` **without** a locale prefix (not
      `/en/admin/...`).
- [ ] `grep -rn "i18n/navigation\|ButtonLink" apps/web/app/admin apps/web/components/admin-*`
      returns nothing — admin never uses the locale-aware `Link`
      (requirement.md Decision 13).
- [ ] `grep -rn "Fraunces\|Work_Sans" apps/web` matches only
      `apps/web/lib/fonts.ts` — the loaders were extracted, not copied.

### Live dev stack (`docker compose up`) — manual smoke test

Seed a few orders first by placing them through the public `/en/order`
flow (at least four, across two or more statuses, so pagination and
filtering have something to act on), or insert them directly via `psql`.

- [ ] **Fonts:** `/admin` headings render in Fraunces, not Times New
      Roman — the regression requirement.md Decision 16 exists to fix.
      Compare against `/en` to confirm they match.
- [ ] **Shell:** every admin page shows the sidebar with all five nav
      items. Dashboard and Orders are links; Menu, Categories and
      Settings are muted, carry a "Soon" marker, are **not** clickable,
      and are **skipped by Tab** (keyboard-only pass confirms focus goes
      Dashboard → Orders → Log Out).
- [ ] The nav item for the current page is visually highlighted and
      carries `aria-current="page"` (check in dev tools); navigating
      between Dashboard and Orders moves the highlight.
- [ ] Below `md` width the sidebar stacks above the content and nothing
      overflows horizontally.
- [ ] **Dashboard (`/admin`):** shows a "Placed today" tile plus all six
      status tiles including ones reading `0`; the counts match
      `SELECT status, count(*) FROM orders GROUP BY status;` in `psql`.
- [ ] Clicking a status tile lands on `/admin/orders?status=<that>` with
      that filter already active.
- [ ] "Recent orders" lists at most five, newest first, each linking to
      its detail page. With zero orders in the table the dashboard
      renders "No orders yet." rather than erroring.
- [ ] **Order list (`/admin/orders`):** the table shows ID, customer,
      type, total, status badge and placed-at; newest first. Totals
      render as `€x.xx` via `formatPrice`, dates as e.g. `4 Oct 2026,
      14:02`.
- [ ] Clicking a status filter chip narrows the table and resets to page
      1; "All" clears it. The active chip is highlighted.
- [ ] With more than 25 matching orders, the pager appears, "Next ›"
      advances to `?page=2`, "‹ Previous" returns, and the end-of-range
      control is rendered inert rather than as a broken link. (If there
      aren't 25 real orders, temporarily lower `ORDERS_PAGE_SIZE` to 2 to
      exercise this, then put it back.)
- [ ] A hand-typed `/admin/orders?status=banana` falls back to the
      unfiltered list rather than erroring, and `?page=0` /
      `?page=abc` fall back to page 1.
- [ ] **Order detail (`/admin/orders/<id>`):** shows every line item with
      its `option_label` where one exists, unit price, quantity and line
      subtotal; and the subtotal / delivery fee / total block matches the
      `orders` row in `psql`. Customer name, email, phone, type, address
      and notes all render, with `—` where a field is null.
- [ ] **Status update:** changing the select updates the badge and the
      "Last updated" timestamp after the refresh; `SELECT status,
      updated_at FROM orders WHERE id = <id>;` confirms both changed in
      the database (not just on screen).
- [ ] Moving a status **backward** (e.g. `ready` → `new`) succeeds —
      requirement.md Decision 4, explicitly not rejected.
- [ ] Hard-refresh the detail page after a status change — the new status
      persists.
- [ ] **Keyboard/a11y pass** on the detail page: the status `<select>` is
      reachable by Tab, has a visible focus ring, and is operable with
      arrow keys + Enter.

### Auth gating (the part the frontend must not be trusted for)

- [ ] Logged out, visiting `/admin/orders` and `/admin/orders/1` directly
      both redirect to `/admin/login` (no flash of dashboard content).
- [ ] `curl -i http://localhost:8080/api/admin/orders` (no cookie) →
      `401 {"error":"unauthorized"}`.
- [ ] `curl -i http://localhost:8080/api/admin/orders/summary` (no
      cookie) → `401`.
- [ ] `curl -i -X PATCH http://localhost:8080/api/admin/orders/1/status -H
      'Content-Type: application/json' -d '{"status":"cancelled"}'` (no
      cookie) → `401`, and `psql` confirms order 1's status is
      **unchanged**.
- [ ] `curl -i 'http://localhost:8080/api/admin/orders?status=banana'`
      with a valid session cookie → `400 {"error":"validation_error",
      "fields":[{"field":"status",...}]}` — the project's error envelope,
      not axum's raw `422` text (requirement.md Decision 8).
- [ ] `curl -i 'http://localhost:8080/api/admin/orders?limit=101'` with a
      valid cookie → `400`, `fields[0].field == "limit"`.
- [ ] `curl -i http://localhost:8080/api/admin/orders/999999` with a
      valid cookie → `404 {"error":"not_found"}`.

### Regression

- [ ] `/validate` skill run (full lint + build + live HTTP smoke sweep) —
      no regression on `/`, `/en/menu`, `/en/cart`, `/en/order`,
      `/api/categories`, `/api/menu-items`, `/api/orders`, `/health`, or
      the Phase 11 login/logout loop.
- [ ] Public site fonts and typography are unchanged after the
      `lib/fonts.ts` extraction — compare `/en` and `/nl` against
      `master` visually.
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

An administrator who logs in lands on a real dashboard, not a placeholder:
a shell whose sidebar lists all five roadmap sections — Dashboard and
Orders live, Menu, Categories and Settings visibly disabled until Phase 14
and beyond — around a landing page that answers "what needs my attention"
with per-status counts, a placed-today count and the five most recent
orders. From there they can page through every order ever placed, filtered
by status, open any one of them to see exactly what was ordered (item
names, option labels, quantities, unit prices and totals captured at
purchase time), who placed it and how they want it, and move it to any of
the six statuses — with the change written to Postgres, `updated_at`
bumped, and the new state surviving a hard refresh. Every one of the four
new endpoints is enforced server-side by Phase 11's `require_admin`
middleware and rejects an unauthenticated request independently of the
frontend. Nothing about an order other than its status can be changed from
this UI, so `mission.md`'s historical-accuracy rule is untouched; admin
menu and category management remain Phase 14's job.
