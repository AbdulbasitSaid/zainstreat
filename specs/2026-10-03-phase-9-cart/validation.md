# Phase 9 — Cart: Validation

## Pass/fail checklist

- [ ] `cd apps/web && pnpm lint` — clean, no `@typescript-eslint/no-explicit-any`
      violations on any new/changed file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean, no implicit/explicit `any`.
- [ ] `cd apps/web && pnpm build` — succeeds (`/cart` appears in the build's
      route list for both locales, e.g. `/en/cart`, `/nl/cart`).
- [ ] `grep -c '"Cart"' apps/web/messages/en.json apps/web/messages/nl.json`
      — both files define the `Cart` namespace; spot-check the key sets
      match between the two files (same keys, no file missing one).
- [ ] Live dev stack (`docker compose up`), manual smoke test against
      `http://localhost:3000` (or whatever port `web` publishes in dev):
  - [ ] `/en/menu`: a flat-priced item shows one "Add to Cart" button; an
        item with `price_options` (e.g. Egusi Soup) shows one "Add {option}"
        button per option, with the existing label/price display untouched
        above it.
  - [ ] An item with `is_available: false` shows no add-to-cart control at
        all (only the existing "Currently unavailable" badge).
  - [ ] Clicking "Add to Cart" on a flat-priced item, then clicking it again
        on the same item, produces **one** cart line with quantity 2 (not
        two separate lines) — confirms `lineKey` dedupe/increment logic.
  - [ ] Clicking "Add Egusi Soup — 2 L" then "Add Egusi Soup — 3 L" produces
        **two** distinct cart lines, each quantity 1 — confirms price-option
        lines are keyed independently from each other and from the flat case.
  - [ ] Header cart icon shows no badge when the cart is empty, and shows
        the correct total item count (sum of all line quantities, not
        distinct line count) after adding items.
  - [ ] `/en/cart` with an empty cart: shows the empty-state `Notice` plus a
        working "Continue Shopping" link back to `/en/menu`.
  - [ ] `/en/cart` with items: each line shows image/placeholder, name,
        option label (when present), unit price, a working quantity
        stepper, a per-line total, and a working "Remove" action; the page
        subtotal equals the sum of all line totals.
  - [ ] Quantity stepper's `−` button at quantity 1 removes the line
        entirely (reducer treats `quantity - 1 = 0` as removal).
  - [ ] "Clear Cart" empties all lines and the page falls back to the empty
        state.
  - [ ] "Proceed to Order" renders visibly disabled with a tooltip
        explaining it's coming soon — does not navigate anywhere or throw.
  - [ ] Reload the page (full browser refresh, not client nav) after adding
        items — cart contents survive (confirms `localStorage` persistence
        and the hydrate-effect/persist-effect ordering don't clobber stored
        data on mount).
  - [ ] Open DevTools → Application → Local Storage, confirm a
        `zainstreat:cart:v1` key holding the current cart JSON.
  - [ ] With DevTools "block all storage" (or a private window with storage
        disabled) — site still loads and cart add/remove still works
        in-memory for that session with no thrown error or broken UI
        (confirms the `try`/`catch` degrade path).
  - [ ] Toggle locale `/en` ↔ `/nl` while on `/cart` — all new strings
        (heading, empty state, buttons, aria-labels) switch language
        correctly, cart contents (item `name`s, which are stored as
        plain strings from the API, not translation keys) remain
        unchanged.
  - [ ] Keyboard-only pass on `/cart`: tab reaches every quantity
        stepper button, remove link, and action button in a sensible
        order; each has a non-empty accessible name (stepper buttons'
        `aria-label`s mention the item name; remove is a visible labeled
        button).
- [ ] `/validate` skill run (full lint + build + live HTTP smoke sweep) —
      confirm no regression on existing routes (`/`, `/en/menu`,
      `/api/categories`, `/api/menu-items`, `/health`).
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

A customer can add a flat-priced or price-option item to a cart from the
public menu (unavailable items excluded entirely), see a running item-count
badge in the header, visit `/cart` to review lines with working
quantity/remove/clear controls and a correct subtotal, and have that cart
survive a page reload via `localStorage`. The empty-cart state links back
to the menu, and a visibly-disabled "Proceed to Order" placeholder marks
exactly where Phase 10 (Order Submission) will attach next — no `apps/api`
change, no new dependency, and no `any` anywhere in the new code.
