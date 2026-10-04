# Phase 13 — Admin: Orders: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 13 — the
original three bullets, expanded by this phase's planning to record the
decisions below:

> - Dashboard shell (nav: Dashboard, Orders, Menu, Categories, Settings).
> - Order list (ID, customer, total, status) and order detail view (items,
>   quantities, totals, delivery/pickup info, notes).
> - Update order status.

This is the phase `mission.md`'s business goal 6 ("Provide a simple
order-management system") has been waiting for: orders have been
submittable since Phase 10 and visible to nobody since Phase 10.

## Context

Phase 10 made `POST /api/orders` real — `orders` and `order_items` rows
have been accumulating in Postgres with no way to read them back except
`psql`. Phase 11 then built the authentication seam this phase plugs into,
and said so explicitly in its own `requirement.md` Decision 5:

> Phases 12/13's admin CRUD endpoints (orders, menu, categories) nest into
> this same protected group later and inherit the middleware for free —
> this phase builds the group specifically so that's a route addition, not
> a new auth wire-up, next time.

So the pieces already on disk that this phase consumes rather than builds:

- `apps/api/src/routes/admin/mod.rs` — `admin_router()` with a `protected`
  sub-router carrying `.route_layer(from_fn(middleware::require_admin))`.
  New admin endpoints are added to `protected` and are authenticated with
  no further wiring.
- `apps/api/src/error.rs` — `AppError::{Database, Validation,
  ItemsUnavailable, Unauthorized, InvalidCredentials, Session}` with the
  `{"error": "..."}` JSON envelope and `sqlx::Error::RowNotFound → 404
  not_found` mapping this phase's "unknown order id" case gets for free.
- `apps/web/app/admin/(protected)/layout.tsx` — already `force-dynamic`,
  already calls `getCurrentAdmin()` and `redirect("/admin/login")`. This
  phase turns it into the dashboard shell; the auth gate itself is
  untouched.
- `apps/web/app/admin/(protected)/page.tsx` — the bare "Welcome, `{name}`"
  placeholder Phase 11 described as "deliberately bare; [the next phase]
  replaces this with the real dashboard shell." This phase replaces it.
- `apps/web/app/api/admin/{login,logout}/route.ts` — the same-origin Route
  Handler proxy pattern (forwarding `Cookie` and `Set-Cookie` in both
  directions) this phase's one write operation copies.

The data model needs nothing new. `orders` already has
`customer_name/email/phone`, `delivery_type` (`'pickup' | 'delivery'`),
`delivery_address`, `notes`, `subtotal`, `delivery_fee`, `total`,
`status` (CHECK-constrained to `'new' | 'confirmed' | 'preparing' |
'ready' | 'completed' | 'cancelled'`, default `'new'`), `created_at`,
`updated_at`, plus `idx_orders_status`. `order_items` already has
`item_name`, `option_label`, `unit_price`, `quantity`, `subtotal` and
`idx_order_items_order_id`. **No migration is written this phase.**

Branches off `master` (Phase 12 is already merged; this phase touches no
file unique to any other unmerged branch).

## Decisions

1. **The dashboard shell is the existing `(protected)/layout.tsx`, not a
   new nesting level.** That layout already runs the auth gate and is
   already `force-dynamic`; adding the sidebar/main chrome there means one
   server round-trip for `getCurrentAdmin()` serves both the gate and the
   nav's "signed in as" line, and every page added under `(protected)` in
   Phase 14 inherits the shell automatically. `/admin/login` stays outside
   the route group and keeps its current bare, chrome-less treatment.

2. **All five nav items render; Menu, Categories and Settings render
   disabled.** Decided with the user (over rendering only what exists, and
   over linking all five to "coming soon" stub pages). They are
   `<span aria-disabled="true">`, not `<a>` — not focusable, not
   navigable, visually muted with a "Soon" marker. Reasons: the roadmap
   bullet is met literally; Phase 14 turns two of them on by changing a
   flag in one array rather than restructuring the nav; and an admin never
   clicks into a dead end. Settings has no roadmap phase at all and stays
   disabled past Phase 14 until one exists.

3. **`/admin` is a stats overview, not a welcome message or a redirect to
   `/admin/orders`.** Decided with the user. It renders: a count per
   status (all six, including zeroes), a count of orders placed today, and
   the five most recent orders as links into their detail pages. This
   gives the nav's "Dashboard" entry a reason to exist and answers "what
   needs my attention right now" without a click — the single most common
   question the owner opens this dashboard to ask.

4. **Order status may be changed from any value to any other — no
   enforced forward-only workflow.** Decided with the user (over
   `new → confirmed → preparing → ready → completed` with backward jumps
   rejected `409`, and over a forward-default-plus-confirmed-override
   hybrid). Reason: one person runs this kitchen, and a mis-tap they can
   undo themselves beats a mis-tap that needs a developer and a `psql`
   session. The database's existing CHECK constraint remains the only
   restriction on what a status can be; see open risk 5.

5. **Four new endpoints, all nested into Phase 11's existing `protected`
   sub-router** in `apps/api/src/routes/admin/mod.rs`, so all four are
   behind `require_admin` with no new auth code:
   - `GET /api/admin/orders?status=&limit=&offset=` — list
   - `GET /api/admin/orders/summary` — dashboard counts + recent
   - `GET /api/admin/orders/{id}` — detail with items
   - `PATCH /api/admin/orders/{id}/status` — update status

   `/orders/summary` is a static path segment, so axum's router matches it
   ahead of the `/orders/{id}` parameterized route regardless of
   registration order — but both are registered in the same `Router` and
   the static one is written first for readability.

6. **The list endpoint returns a pagination envelope
   (`{ orders, total, limit, offset }`), not a bare array** — unlike the
   public `GET /api/categories` and `GET /api/menu-items`, which return
   bare arrays. The client cannot render "Page 1 of 3" or disable a "Next"
   button without knowing `total`, and an `X-Total-Count` header would
   mean threading a non-JSON channel through the Next.js server-component
   fetch for one number. `GET /api/admin/orders/{id}` and
   `GET /api/admin/orders/summary` return plain objects. **This envelope
   is the convention Phase 14's admin list endpoints should follow.**

7. **Pagination is `limit`/`offset`, default `limit=25`, maximum
   `limit=100`.** A `limit` or `offset` that is negative, or a `limit`
   above 100, is a `400 validation_error` naming the field — not silently
   clamped, so a caller never silently gets a different page than it asked
   for. Chosen over keyset/cursor pagination: offsets let the UI render a
   real "Page N of M" pager from `total`, which a cursor cannot, and the
   row counts here will not reach the depth where `OFFSET` scanning costs
   anything. See open risk 3.

8. **Statuses are validated as a plain `String` against one shared
   `ORDER_STATUSES` constant, producing `AppError::Validation`** — not
   modelled as a `#[derive(Deserialize)]` enum like `orders.rs`'s
   `DeliveryType`. A bad enum variant makes `axum`'s `Json`/`Query`
   extractor reject the request itself with its own `422` plain-text body,
   bypassing `error.rs` entirely, so the frontend would have to parse two
   unrelated error shapes. Explicit validation keeps every failure in this
   phase inside the established `{"error": "validation_error", "fields":
   [...]}` envelope. The same constant backs both the `?status=` filter
   and the `PATCH` body, and mirrors the database CHECK constraint — see
   open risk 7.

9. **The status update sets `updated_at = now()` explicitly in the
   `UPDATE` statement.** `orders.updated_at` has a `DEFAULT now()` but
   there is no `BEFORE UPDATE` trigger anywhere in
   `apps/api/migrations/` — the default only fires on `INSERT`. Without
   the explicit assignment `updated_at` would silently freeze at creation
   time, making the detail view's "last updated" line a lie. Adding a
   trigger migration instead was considered and rejected: one `UPDATE`
   statement in the whole codebase touches this table, and the repo has no
   trigger precedent to follow.

10. **Reads are server-to-server from Server Components; the one write
    goes through a same-origin Route Handler proxy.** Direct continuation
    of Phase 11 Decision 10 and Phase 10 Decision 7 — the browser never
    talks to `apps/api` directly. The list, detail and summary pages fetch
    against `API_BASE_URL` with the incoming `Cookie` header forwarded;
    the status change POSTs to a new
    `apps/web/app/api/admin/orders/[id]/status/route.ts` which forwards
    `Cookie` to the API and the status code/body back. **No Server Actions
    are introduced** — nothing in the codebase uses `"use server"` today,
    and introducing a second write mechanism alongside the established
    proxy pattern for one `PATCH` is not worth the inconsistency.

11. **A shared `apps/web/lib/admin-api.ts` owns cookie-forwarding admin
    fetches, and `lib/admin-auth.ts`'s `getCurrentAdmin()` is refactored
    onto it.** `getCurrentAdmin()` is currently the only place that reads
    `cookies()` and forwards them to `API_BASE_URL`; this phase adds three
    more callers, and four copies of that six-line block is exactly the
    duplication Phase 11 Decision 12 refactored `tests/common.rs` to
    avoid. `getCurrentAdmin()`'s external behaviour is unchanged (still
    returns `null` rather than throwing on a missing `API_BASE_URL`, a
    missing cookie, or a non-`200`), because the `(protected)` layout's
    redirect depends on exactly that.

12. **Dates render via `Intl.DateTimeFormat("en-GB", { timeZone:
    "Europe/Amsterdam" })`**, added as `formatDateTime`/`formatDate` in
    the existing `apps/web/lib/format.ts` next to `formatPrice`. The
    explicit `timeZone` is load-bearing: without it a server component
    renders in the container's UTC and a client re-render would use the
    visitor's zone, producing a hydration mismatch and a wrong "placed
    today". `en-GB` rather than `formatPrice`'s `nl-NL` because the admin
    surface is English-only (Decision 14) — the business, and therefore
    the wall clock that matters, is still Dutch.

13. **Admin pages use `next/link`, never `@/i18n/navigation`'s `Link` or
    the `ButtonLink` component.** `ButtonLink` imports the locale-aware
    `Link`, which would rewrite admin hrefs into `/en/admin/...` — the
    exact thing Phase 11 Decision 8 took `/admin` outside the locale
    scheme to avoid. Plain `Button` (a bare `<button>`) is still reusable
    as-is, as are `Notice`, `buttonClasses` and the `.field`/`.container`
    classes in `globals.css`.

14. **Admin stays English-only** — no new keys in
    `apps/web/messages/{en,nl}.json`, consistent with `tech-stack.md`'s
    i18n scope ("the admin dashboard and the Rust API stay English-only")
    and Phase 11 Decision 8.

15. **No database migration.** `orders` and `order_items` already carry
    every field this phase reads or writes, and `idx_orders_status`
    already covers the list endpoint's filter. See open risk 4 for the
    one index this phase deliberately does *not* add.

16. **The brand fonts are extracted to `apps/web/lib/fonts.ts` and applied
    in both root layouts.** Found during research, not anticipated:
    `--font-display` (Fraunces) and `--font-body` (Work Sans) are loaded
    by `next/font/google` inside `app/[locale]/layout.tsx` and applied to
    *that* layout's `<html>`. `app/admin/layout.tsx` is an independent
    second root layout (Phase 11 Decision 8) that imports `globals.css`
    but never loads those fonts — so `globals.css`'s
    `h1..h6 { font-family: var(--font-display), serif }` resolves to the
    bare `serif` fallback and every admin heading currently renders in
    Times New Roman. Phase 11's single-paragraph login page made this
    invisible; a full dashboard makes it obvious. The two
    `next/font/google` loader calls move to a shared module and both root
    layouts apply the same `variable` class names — no duplicated loader
    config, no second font download (Next.js dedupes by loader call site).

17. **Backend gets integration tests** (`apps/api/tests/admin_orders.rs`),
    with a `patch_with_cookie` helper added to the shared
    `apps/api/tests/common.rs`. **The frontend continues to have none** —
    validated manually via `/validate`'s live smoke test, same precedent
    as every prior phase.

## Out of scope

- Admin CRUD for menu items and categories (add/edit/archive/availability
  toggle) — `roadmap.md` Phase 14. This phase only renders those two nav
  entries, disabled.
- A Settings page of any kind — no roadmap phase owns one yet; the nav
  entry stays disabled past Phase 14.
- Editing an order's items, quantities, customer details, totals or
  delivery address, and deleting orders. Status is the only mutable field
  — anything else would put `mission.md`'s "historical orders must retain
  the item name and price as they were at the time of purchase" rule at
  risk for no MVP benefit.
- Notifying the customer when their status changes (email or otherwise) —
  `roadmap.md` Phase 17 owns wiring `lettre` in.
- Searching orders by customer name/phone/email, date-range filtering, and
  CSV/print export. The status filter is the only filter this phase ships.
- A status-change audit trail (who changed what, when, from what) —
  open risk 1; multi-staff accounts are Beyond MVP per `tech-stack.md`.
- Auto-refresh/polling/websockets on the order list — an admin reloads the
  page.
- Rate limiting on admin endpoints — still the open item Phase 11 risk 1
  left for `roadmap.md` Phase 17.
- Kitchen tickets / printable order slips.

## Open risks flagged during planning

1. **Status changes leave no audit trail.** `updated_at` records *when* an
   order last changed, not *who* changed it or *what from*. Harmless while
   exactly one admin account exists (Phase 11's `create_admin` CLI is the
   only way to make one), but the moment staff accounts arrive — Beyond
   MVP, per `tech-stack.md` — "who marked this cancelled?" becomes
   unanswerable without a new table.
2. **No optimistic-concurrency check on `PATCH .../status`.** Two tabs, or
   two people, changing the same order concurrently is a silent
   last-write-wins. A `version`/`If-Unmodified-Since` guard would fix it;
   not worth the complexity at one-admin scale.
3. **`limit`/`offset` pagination drifts under concurrent inserts** — a new
   order arriving while the admin is on page 2 shifts every later row down
   by one, so one order can be seen twice or skipped. Orders arrive at a
   rate of a handful a day; keyset pagination is the fix if this ever
   actually bites.
4. **The dashboard summary scans `orders` on every page load** — a
   `GROUP BY status` plus a `created_at >= today` count, with no index on
   `created_at`. Nothing at MVP row counts, but the deliberate choice not
   to add `idx_orders_created_at` is recorded here so the reason is
   "measured as unnecessary", not "forgotten".
5. **Free-form status changes (Decision 4) have no confirmation step** — a
   mis-tap can move an order backward or mark it `cancelled` silently. The
   user chose this explicitly over a guarded workflow; the mitigation is
   that the same control can immediately put it back.
6. **The dashboard and list pages are `force-dynamic` and uncached**, each
   making 1–3 API calls per request (`/me` for the shell, plus the page's
   own). Negligible over Docker-internal networking on one box, but it is
   per-navigation, not amortized.
7. **The `ORDER_STATUSES` constant duplicates the database's CHECK
   constraint** (Decision 8) — adding a seventh status means editing both
   a migration and a Rust constant, and forgetting the Rust side means the
   new status is unreachable through the API while being perfectly legal
   in the database. A `status` lookup table would remove the duplication
   at the cost of a join on every read; not worth it for six values that
   have not changed since Phase 4.
