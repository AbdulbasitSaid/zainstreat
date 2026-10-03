# Phase 9 — Cart: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 9 (unchanged by
this phase):

> - Client-side cart: add item, change quantity, remove item, subtotal.
> - Empty-cart state and "continue shopping" / "proceed to order" actions.
> - Unavailable items cannot be added to the cart.

## Context

Phase 6 (Menu Browsing) and Phase 7 (Menu Price Variants) built a read-only
public menu: `MenuItemCard` renders either a flat `price` or a
`price_options` list (`{ id, label, price }`), with no interactivity at all.
Phase 7's `requirement.md` explicitly deferred "choosing a variant and
adding it to a cart" to this phase, since no cart existed yet to wire it
into.

This is also the first phase to introduce any client-side shared state —
`apps/web` currently has zero `createContext` usage, no state library
(`zustand`/`jotai`/etc. — none installed), and no `localStorage` usage
anywhere (confirmed by repo-wide grep). Category filtering on the menu page
is done via URL search params, not client state.

This phase is purely client-side, per its own roadmap bullets. It does not
touch `apps/api`, does not persist orders anywhere server-side, and does not
build the "proceed to order" destination — that's Phase 10 (Order
Submission), which doesn't exist yet.

This phase branches directly off `master` (Phase 8 is already merged; this
phase touches no file unique to any unmerged branch).

## Decisions

1. **Cart persistence: `localStorage`, not in-memory-only.** Decided with
   the user. A customer building an order across several menu categories
   losing everything on an accidental refresh or tab close is bad UX for
   this site's core flow (mission.md: "go from seeing a meal to submitting
   an order with minimal friction"). Stored under a versioned key
   (`zainstreat:cart:v1`) so a future shape change can invalidate old data
   cleanly instead of crashing on a stale parse.

2. **Cart UI surface: a dedicated `/cart` page, not a slide-over drawer.**
   Decided with the user. Matches the roadmap's page-ish phrasing
   ("continue shopping" / "proceed to order" read as navigations, not
   panel actions) and keeps the implementation scoped to cart *logic*
   rather than also building a drawer's overlay/focus-trap/animation
   machinery. It also sets up a page-based precedent for Phase 10's order
   review step to follow.

3. **State management: React Context + `useReducer`, no new dependency.**
   The cart's shape (a list of line items plus add/set-quantity/remove/clear
   actions) is simple enough that React's built-ins are sufficient; adding
   `zustand`/`jotai` for this would be an unjustified new dependency given
   `tech-stack.md`'s general minimal-dependency posture. `CartProvider`
   wraps `UtilityBar` + `SiteHeader` + the page content + `SiteFooter` inside
   `app/[locale]/layout.tsx`'s `NextIntlClientProvider`, so both the header
   badge and the `/cart` page read the same state.

4. **Cart line identity: `(menu item id, price option id | null)`.** A
   flat-priced item's line key is `"<itemId>:flat"`; a priced-option item's
   line is `"<itemId>:<optionId>"` — e.g. Egusi Soup "2 L" and Egusi Soup
   "3 L" are two independent lines, each with its own quantity. Adding the
   same key again increments that line's quantity instead of duplicating
   it. Each line snapshots `name`, `optionLabel`, `unitPrice`, and
   `imageUrl` at add-time (not a live reference to the `MenuItem`), the same
   "snapshot, don't re-derive" principle `mission.md` already mandates for
   historical orders — this phase just applies it one step earlier, to the
   cart itself.

5. **No variant-selection dropdown — one "Add" control per price option.**
   An item with `price_options` renders one small button next to each
   option's existing label/price line (reusing the existing display from
   Phase 7, not replacing it) instead of a single "Add" button plus a
   separate size-picker. Simpler to build, and avoids a second UI control
   class (`<select>`) for what Phase 7 already renders as a short list (at
   most 2 options per item in the real menu data).

6. **Unavailable items get no add control at all**, not a disabled one.
   `MenuItemCard` already renders an "Unavailable" badge over the image;
   `AddToCartControls` (the new client sub-island) returns `null` when
   `item.is_available` is `false`. Satisfies the roadmap's "unavailable
   items cannot be added to the cart" bullet at the only point this phase
   touches menu rendering.

7. **`MenuItemCard` stays a server component.** Only the new
   `AddToCartControls` component (and the handful of other new
   cart-specific components) are `"use client"`. `MenuItemCard` imports and
   renders the client island as a child — valid in the App Router and
   keeps the existing server-rendered data-fetching path in `menu/page.tsx`
   untouched.

8. **"Proceed to order" is a disabled placeholder button**, not a link to a
   non-existent route. Phase 10 (Order Submission) doesn't exist yet, so
   there is nowhere real to send the customer; a `disabled` `Button` with a
   `title` tooltip (translated) is honest about current scope rather than
   linking somewhere misleading or 404ing. Phase 10 wires this button up.

9. **No live re-validation of item availability once something is already
   in the cart.** If an item is marked unavailable (or its price changes)
   by an admin after a customer already added it, the cart keeps showing
   the snapshot taken at add-time; this phase does not re-fetch
   `/api/menu-items` to check. Server-side validation at actual order
   submission is Phase 10's job, consistent with Decision 4's
   "snapshot, don't re-derive" model and with `mission.md`'s non-negotiable
   rule that historical records don't silently change.

10. **Parsing stored `localStorage` JSON uses a type guard over `unknown`,
    never `any`.** `JSON.parse` returns `any` by default; the cart module
    reads it as `unknown` and narrows with a hand-written `isCartLine`
    predicate before trusting any of it, satisfying the repo's strict
    no-`any` rule (`tsconfig.json` + `eslint.config.mjs`) and guarding
    against a hand-edited or stale-shape value in storage.

## Out of scope

- The actual "proceed to order" destination/flow — Phase 10 (Order
  Submission).
- Any `apps/api` change — this phase is 100% client-side per its roadmap
  bullets.
- Re-validating cart line availability/pricing against the live menu —
  deferred to Phase 10's server-side submission validation (Decision 9).
- A slide-over drawer UI — explicitly decided against (Decision 2).
- Promo/discount codes, delivery fee calculation — both already listed
  under `roadmap.md`'s "Beyond MVP" section.
- Automated unit tests for the reducer — `apps/web` has no test runner
  installed (no `jest`/`vitest` in `package.json`) in any prior phase;
  consistent with that precedent, this phase validates manually via
  `/validate`'s live smoke test rather than introducing a new test
  dependency as a one-off.

## Open risks flagged during planning

1. **"Proceed to order" ships disabled.** Whoever plans Phase 10 needs to
   wire this specific button (`components/cart-view.tsx`) to the real
   order-review flow once it exists, rather than building a parallel entry
   point.
2. **Stale cart data isn't reconciled with the live menu.** A customer who
   leaves a full cart for days (`localStorage` persists indefinitely) could
   submit against a `price`/`is_available` snapshot that's since drifted
   from the real menu. Acceptable for now per Decision 9, but Phase 10's
   submission endpoint must treat the client's cart payload as untrusted
   and re-price/re-validate server-side regardless — it already needs to
   do this anyway per `mission.md`'s "only available items can be ordered"
   rule.
3. **`localStorage` can throw** (Safari private browsing with a full quota,
   disabled storage, etc.). Both the read and write paths are wrapped in
   `try`/`catch` and degrade to an in-memory-only cart for that session
   with no user-facing error — acceptable, but means a customer in that
   situation gets no explanation for why their cart didn't survive a
   reload.
