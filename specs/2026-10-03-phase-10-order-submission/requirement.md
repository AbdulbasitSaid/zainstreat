# Phase 10 — Order Submission: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 10 (unchanged by
this phase):

> - Customer details form (name, phone, email, pickup/delivery, notes).
> - Order review step.
> - API: submit order — persists `orders` + `order_items`, capturing item
>   name/price at the time of the order (not a live reference to
>   `menu_items`).
> - Order confirmation page showing the order number and next steps.

## Context

Phase 9 (Cart) built a fully client-side cart (`localStorage`-persisted,
React Context + `useReducer`) with a disabled "Proceed to Order" placeholder
button on `/cart`, explicitly left for this phase to wire up (Phase 9's
`requirement.md` Open risk 1). Phase 9 also explicitly flagged (Open risk 2)
that the submission endpoint "must treat the client's cart payload as
untrusted and re-price/re-validate server-side" — this phase is where that
promise is kept.

This is also the **first write endpoint** `apps/api` has ever exposed —
every prior phase (`/api/categories`, `/api/menu-items`) is read-only — and
the **first time `apps/web` needs a client-side (browser) network call** to
reach `apps/api` at all; every fetch so far (`lib/api.ts`) runs inside a
Server Component using the server-only `API_BASE_URL` env var.

Branches directly off `master` (Phase 9 is already merged; this phase
touches no file unique to any unmerged branch).

## Decisions

1. **Flow: a single `/order` page with client-side wizard steps**
   (`details` → `review` → `success`), not three separate routes. Decided
   with the user. Matches Phase 9's "page, not drawer" precedent and avoids
   building cross-route state passing (query params/`sessionStorage` just to
   get from step to step) for a flow that's inherently one continuous task.
   `sessionStorage` is still used (Decision 5), but only to survive a hard
   refresh while already on the `success` step — not to pass state between
   routes.

2. **Form validation: plain React state + a hand-written `validate()`
   function, no new dependency.** Decided with the user. Same
   minimal-dependency posture Phase 9 used to reject `zustand`/`jotai` for
   the cart; a 6-field form doesn't justify being the first phase to
   introduce `react-hook-form`/`zod`. Reuses the existing `.field` CSS class
   and label pattern already sitting unused in the Phase 5 static
   contact-form shell (`app/[locale]/contact/page.tsx`).

3. **Server-side submission always re-fetches and re-prices from the
   current `menu_items`/`menu_item_price_options` rows — the client's cart
   `unitPrice` is never trusted or persisted.** This satisfies `mission.md`'s
   "price as it was at the time of the order" rule by treating "the time of
   the order" as the moment the API inserts the row, not the moment the
   customer added something to a possibly-stale `localStorage` cart. There
   is therefore no separate "price changed" error case — a price drift
   between add-to-cart and submission is silently resolved by using the
   current price. The **only** rejection case is an item that is
   unavailable, soft-deleted, or whose `(menu_item_id, price_option_id)`
   pairing no longer makes sense (e.g. the item became flat-priced after the
   customer's cart line was taken from its old `price_options` form, or vice
   versa) — see Decision 6.

4. **Reject the whole order, return which lines failed and why, nothing is
   persisted.** Decided with the user (over silently dropping bad lines, and
   over trusting the client outright). `POST /api/orders` validates every
   line inside the same transaction that would otherwise insert the order;
   if any line is invalid, the transaction is rolled back and a single `409
   items_unavailable` response lists every offending line with a `reason` —
   the frontend surfaces this on the review step rather than silently
   mutating what the customer agreed to.

5. **Confirmation data comes back directly in the `POST /api/orders`
   response body; no `GET /api/orders/:id` endpoint exists.** Decided with
   the user. There's no customer auth/session yet (Phase 11 is Admin Auth
   only) to gate a lookup endpoint, and `orders.id` is a sequential
   `BIGSERIAL` — a public-by-ID lookup would let anyone enumerate other
   customers' names/phone numbers/orders. The frontend holds the created
   order in component state and mirrors it into
   `sessionStorage["zainstreat:last-order:v1"]` purely so a hard refresh
   while still on `/order`'s `success` step doesn't lose the confirmation
   (component state alone doesn't survive a full reload). Closing the tab or
   navigating away loses it — acceptable with no customer accounts yet.

6. **`order_items` gains nullable `price_option_id` (FK to
   `menu_item_price_options`, `ON DELETE SET NULL`) and `option_label`
   columns**, mirroring the existing nullable `menu_item_id` FK pattern
   introduced in the initial schema. Needed so a priced-option line (e.g.
   "Egusi Soup — 2 L") records *which* option was ordered, not just a
   denormalized name string — Phase 12 (Admin: Orders) will want to display
   this distinctly, and the API's own revalidation logic needs
   `price_option_id` to re-look-up the specific option's current price.

7. **`apps/web`'s browser never talks to `apps/api` directly — a new
   Next.js Route Handler (`app/api/orders/route.ts`) proxies the browser's
   `POST` server-side to the existing internal `API_BASE_URL`.** Decided
   with the user, over using the `api.{$DOMAIN}` Caddy vhost Phase 2 already
   provisioned (confirmed present in the repo's `Caddyfile` but unused by
   any code to date). Reasons: this is the first public write endpoint
   ever exposed by this project, and keeping `apps/api` exactly as exposed
   as it is today (reachable only from `web`, over the internal Docker
   network) needs no new `tower_http` CORS dependency, no `NEXT_PUBLIC_*`
   env var baked into the browser bundle, and no Caddyfile change — all of
   which Option B (direct browser → `api.{$DOMAIN}`) would require getting
   right on day one for a write path. The pre-provisioned `api.{$DOMAIN}`
   vhost remains unused after this phase; revisiting it is left for a
   future phase that has a concrete need for direct browser↔API traffic.

8. **`AppError` (`apps/api/src/error.rs`) becomes an enum** —
   `Database(sqlx::Error)` (today's only variant, renamed/wrapped, behavior
   unchanged), `Validation(Vec<FieldError>)` (400), and
   `ItemsUnavailable(Vec<UnavailableItem>)` (409). First phase needing a
   structured non-database error shape; existing routes
   (`categories.rs`/`menu_items.rs`/`health.rs`) are unaffected since
   `From<sqlx::Error> for AppError` still exists and their `?`-based call
   sites don't change.

9. **`delivery_fee` is always `0`.** Delivery fee calculation is listed
   under `roadmap.md`'s "Beyond MVP" section; `delivery_address` is
   required (client- and server-side) only when `delivery_type ==
   "delivery"`, consistent with the roadmap's "pickup/delivery" bullet.

10. **Minimal field validation only — no sanitization/hardening pass.**
    Required-field checks, a basic email-shape regex, and `quantity > 0`
    — both client-side (fast feedback) and server-side (defense in depth,
    since the client is never trusted for the actual order data per
    Decision 3). `roadmap.md` Phase 16 ("Input validation/sanitization
    hardening on all forms and API endpoints") explicitly owns going
    further than this.

11. **The order insert + every `order_items` insert happen inside one
    `sqlx` transaction**, committed only after every line has been
    validated and priced — a partial order (e.g. 2 of 3 lines inserted, then
    a failure) must never be visible to an admin later.

12. **Backend gets integration tests (`apps/api/tests/order_submission.rs`),
    consistent with `tests/menu_browsing.rs`'s existing `#[sqlx::test]`
    precedent. The frontend continues to have none** (no test runner
    installed in `apps/web`, same precedent Phase 9 left in place) —
    validated manually via `/validate`'s live smoke test instead.

## Out of scope

- Sending an order-confirmation email — `lettre`/transactional email wiring
  is `tech-stack.md`'s P1 item, scheduled for `roadmap.md` Phase 16. The
  "confirmation page" in this phase is an on-screen success step only.
- A `GET /api/orders/:id` (or any) order-lookup endpoint — Decision 5.
- Delivery fee calculation, discount codes, payments — all already listed
  under `roadmap.md`'s "Beyond MVP" section.
- Rate-limiting/CAPTCHA on `POST /api/orders` — flagged as an open risk
  below, not solved here.
- Deep input sanitization/hardening — `roadmap.md` Phase 16 (Decision 10).
- Re-enabling or wiring up the `api.{$DOMAIN}` Caddy vhost for direct
  browser traffic — Decision 7.
- Any `apps/admin`-facing order view — `roadmap.md` Phase 12.

## Open risks flagged during planning

1. **No rate-limiting or CAPTCHA on `POST /api/orders`.** A public,
   unauthenticated write endpoint with no throttling could be used to spam
   fake orders. Acceptable for MVP launch at this traffic scale; worth
   revisiting in Phase 16 (polish/non-functional requirements) or a
   dedicated hardening follow-up if abuse is observed in production logs
   (Phase 8's Dozzle/structured logging now makes that observable).
2. **Small TOCTOU window on availability checks.** The order handler reads
   `menu_items`/`menu_item_price_options` and later inserts within the same
   transaction, but doesn't take row locks (`SELECT ... FOR UPDATE`) — an
   admin marking an item unavailable in the few milliseconds between this
   phase's read and its insert is a race this phase doesn't close. No
   admin-side menu editing exists yet (that's Phase 13), so the window has
   no real way to be hit today; flagged for whoever builds Phase 13 to
   reconsider if it becomes a real concern.
3. **`sessionStorage`-held confirmation is best-effort only** (Decision 5)
   — survives a reload of `/order` itself, nothing else. A customer who
   closes the tab right after ordering has no way to see their order number
   again until Phase 16 wires up confirmation emails.
4. **The pre-provisioned `api.{$DOMAIN}` Caddy vhost stays unused** after
   this phase (Decision 7) — not a regression, but worth a mental note that
   it was provisioned ahead of an actual consumer that still doesn't exist.
