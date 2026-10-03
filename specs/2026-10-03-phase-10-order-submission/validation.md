# Phase 10 — Order Submission: Validation

## Pass/fail checklist

- [ ] `cd apps/api && cargo test` — all new `tests/order_submission.rs` cases
      pass (happy path flat-priced, happy path priced-option, missing
      delivery address, empty cart, non-positive quantity, unavailable
      item with transaction rollback confirmed, soft-deleted item,
      mismatched price-option selection).
- [ ] `cd apps/api && cargo sqlx prepare --check` (or equivalent) — the
      committed `.sqlx/` cache matches the new queries; no drift.
- [ ] `cd apps/web && pnpm lint` — clean, no `@typescript-eslint/no-explicit-any`
      violations on any new/changed file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean, no implicit/explicit `any`.
- [ ] `cd apps/web && pnpm build` — succeeds (`/order` appears in the
      build's route list for both locales, e.g. `/en/order`, `/nl/order`;
      `app/api/orders/route.ts` appears as a server function, not a static
      route).
- [ ] `grep -c '"Order"' apps/web/messages/en.json apps/web/messages/nl.json`
      — both files define the `Order` namespace; spot-check the key sets
      match between the two files (same keys, no file missing one); confirm
      `proceedToOrderComingSoon` was removed from both `Cart` namespaces.
- [ ] Live dev stack (`docker compose up`), manual smoke test against
      `http://localhost:3000`:
  - [ ] From an empty cart, visiting `/en/order` directly shows the
        empty-cart `Notice` with a working link back to `/en/menu` — no
        form is rendered.
  - [ ] Add a flat-priced item and a priced-option item to the cart, click
        "Proceed to Order" on `/cart` — lands on `/en/order` showing the
        customer-details form (not a disabled button anymore).
  - [ ] Submitting the details form with empty required fields shows
        inline field errors and does not advance to the review step.
  - [ ] Entering an invalid-shaped email (e.g. `foo`) shows the
        email-specific error; fixing it clears the error on next submit.
  - [ ] Selecting "Delivery" reveals the address field and makes it
        required; selecting "Pickup" hides it and lifts the requirement.
  - [ ] Submitting a valid form advances to the review step: shows the
        entered customer details (with a working "Edit Details" link back
        to the form, values preserved), the exact cart lines with
        name/option/qty/unit price, and a total matching the cart's
        subtotal (no delivery fee added, per Decision 9).
  - [ ] Clicking "Place Order" on the review step: button shows a
        "Placing Order…" busy state, then advances to the success step
        showing an order number, the same line items, the total, and a
        working "Back to Menu" link.
  - [ ] After success, re-open `/cart` — it's empty (confirms `clearCart()`
        ran) and the header cart badge is gone.
  - [ ] Hard-refresh the browser while still on the success step (full
        reload, not client nav) — the confirmation still renders (confirms
        the `sessionStorage` recovery path works, since cart state alone
        wouldn't survive a reload).
  - [ ] Trigger the `409` path: with an item in the cart, use `psql`
        (`docker compose exec -T postgres psql -U $POSTGRES_USER -d
        $POSTGRES_DB`) to set that `menu_items` row's `is_available` to
        `false`, then click "Place Order" on the review step — review
        stays on-screen with a `Notice` listing the now-unavailable item by
        name, and nothing new appears in `orders`/`order_items` (spot-check
        via `psql`). Restore `is_available` afterward.
  - [ ] `psql` spot-check after a successful order: `orders` has exactly
        one new row with the right `customer_name`/`delivery_type`/
        `subtotal`/`total`; `order_items` rows reference the right
        `menu_item_id`/`price_option_id` and carry the *current* DB price,
        not a stale one (test this by changing a menu item's price in
        `psql` between adding it to cart and submitting — the order should
        reflect the new price, confirming Decision 3).
  - [ ] Toggle locale `/en` ↔ `/nl` at each wizard step — all new strings
        (labels, errors, review, success) switch language correctly.
  - [ ] Keyboard-only pass through the details form and review step: every
        input/radio/button is reachable via Tab in a sensible order, has a
        visible focus state and an associated label.
  - [ ] Directly `curl -X POST http://localhost:8080/api/orders` (bypassing
        `apps/web` entirely) with an empty JSON body — confirms the Rust API
        itself validates and returns `400`/`409` appropriately, independent
        of any client-side checks (defense-in-depth per Decision 10).
- [ ] `/validate` skill run (full lint + build + live HTTP smoke sweep) —
      confirm no regression on existing routes (`/`, `/en/menu`, `/en/cart`,
      `/api/categories`, `/api/menu-items`, `/health`).
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

A customer can go from a populated cart on `/cart` to a confirmed order
without leaving the site: entering their details, reviewing exactly what
they're ordering and for how much, and submitting it. `POST /api/orders`
re-validates and re-prices every line against the live menu inside a single
transaction — an order is either fully persisted with the current price
and option data, or (if any line is unavailable) rejected outright with
nothing written and a clear reason per line. The confirmation step shows
the order number and survives a same-page reload via `sessionStorage`
without exposing a public order-lookup endpoint. No `apps/api` write path
existed before this phase; after it, exactly one does, reachable only
through `apps/web`'s own same-origin proxy route — no new dependency, no
CORS, and no `any` anywhere in the new code.
