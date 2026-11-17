# Phase 30 — Add-to-Cart Feedback: Requirements

## Source

A new phase with no earlier roadmap entry. The user raised it on
2026-11-17, after Phase 27:

> here is a poor user experience for when adding to cart on both mobile
> and web: There is not feedback on adding to cart. we should add splash
> effect and notification animation.

## Context

### What exists

- **Add buttons.** `components/add-to-cart-controls.tsx` renders a round
  "+" in every bento tile (`components/menu-item-card.tsx`), on `/menu`
  and the homepage featured section (Phase 25). A flat-price item adds on
  click. An item with price options opens a native `<dialog>` that lists
  each option with its own "+" and closes after an add. Clicking only
  calls `useCart().addLine(...)` (`lib/cart-context.tsx`), and nothing on
  screen confirms it.
- **The splash.** Each "+" contains a `LiquidSurface`
  (`components/liquid-surface.tsx`), which throws 12 small droplets on
  any click, on touch too. It's subtle, it reads as a click effect
  rather than "added", and the button looks the same afterwards.
- **The cart icon.** `components/site-header.tsx` has a cart link with a
  `CartBadge` (item count) **inside the nav list**. From 769px up it's in
  the header row. **Below 769px the list is the collapsed hamburger menu,
  so on phones the cart and its count are hidden** until the menu is
  opened. `CartBadge` changes its number with no animation.
- **Motion tools.** GSAP (`lib/gsap.ts`, `useGSAP`) and
  `useReducedMotion()` (`lib/use-reduced-motion.ts`) are used across the
  site. `LiquidPageTransition` already stops when the user prefers
  reduced motion.
- **Stacking.** The header is `sticky z-50`, the WhatsApp float button
  is `fixed bottom-6 right-6 z-50` (56px), the page-transition screen is
  `z-40` (its bubbles `z-[90]`), and the custom cursor is `z-[100]`.
- **Preview** (Phase 25): add buttons are disabled inside the admin's
  live preview, so none of this runs there.
- **Messages.** `MenuPage.addToCart`, `MenuPage.addToCartOption` and
  `SiteHeader.cart` exist in en/nl/fr. `pnpm check:messages` enforces key
  parity.

## Decisions

1. **A cart icon is always visible in the mobile header** (asked during
   planning). Below 769px a second cart link (icon + `CartBadge`) sits
   just left of the hamburger button. It's the same link, label and
   liquid hover as the desktop one, and is hidden from 769px up. The one
   inside the menu list is hidden below 769px, so a phone never shows two.
   Both carry `data-cart-target`, and the fly animation (Decision 4)
   aims at whichever one is visible.
2. **One feedback layer, triggered by the add button, not by the cart.**
   A new `CartFeedbackProvider` (`lib/cart-feedback.tsx`) wraps the
   public layout inside `CartProvider`. It exposes
   `announceAdd({ name, optionLabel, imageUrl, origin })`, where
   `origin` is the clicked button's `DOMRect`, and renders the toast and
   the fly layer. `AddToCartControls` calls it right after `addLine`.
   Hydrating the cart from `localStorage` and changing quantities on
   `/cart` never trigger it, because only a deliberate add should
   celebrate.
3. **Button feedback: a bigger splash and a ✓.**
   - The existing `LiquidSurface` click splash stays.
   - An expanding ring (a CSS `scale` + `opacity` keyframe, 450 ms) adds
     a burst behind the button.
   - The "+" swaps to "✓" for 1.2 s, then back. The ✓ is `aria-hidden`,
     and the button's `aria-label` doesn't change; the toast does the
     announcing.
   - Repeat clicks during the 1.2 s restart it, and every click still
     adds one.
   - For an option added from the `<dialog>`, the ✓ shows on that
     option's button. The ring and the fly start from the tile's "+"
     button, because the dialog closes straight after the add.
4. **Fly to cart: the dish photo** (the recommended option; the user
   didn't choose, so it's easy to swap for a brand droplet). A 48px round
   copy of the item's photo starts at the button and arcs to the visible
   `[data-cart-target]` in about 650 ms (GSAP: an `x` tween with
   `power1.out` and a `y` tween with `power2.in`, so it curves). It
   shrinks to 16px and fades at the end. Then the badge bounces
   (Decision 5). Without a photo it's a `bg-primary` dot. It's a
   `position: fixed`, `pointer-events-none`, `z-[60]` element that's
   removed afterwards. If no target is visible, there's no fly.
5. **The cart badge bounces** when the count goes **up** after
   hydration. A GSAP scale of 1 → 1.35 → 1 (`back.out`, 400 ms) is timed
   to the fly's landing when there is one, or runs right away. It
   never bounces when the count goes down or on first load.
6. **The toast** (the recommended wording; the user didn't choose):
   - The text is "**{name}** added to your order", or "**{name}
     ({option})** added to your order", with a **View cart** link
     (`/cart`) and a close button.
   - It sits fixed at the bottom centre on phones, above the safe area,
     clearing the WhatsApp button by being centred and capped at
     `min(22rem, 100vw - 7rem)`. From 769px up it's at the top right,
     just under the sticky header. It's `z-[60]`.
   - It slides and fades in over 250 ms and auto-hides after 3.5 s.
     Hovering it or focusing inside it pauses the timer.
   - Only one toast shows at a time. Another add replaces the text and
     restarts the timer.
   - It's announced through a `role="status"` `aria-live="polite"`
     region that's always rendered, so screen readers hear each add.
     Focus never moves to it.
7. **Reduced motion** (`useReducedMotion()`): no ring, no fly, no
   bounce, and the toast appears and disappears without sliding. The ✓
   swap and the toast stay, because they're information, not decoration.
8. **Copy in en, nl and fr**, written by Claude
   (`feedback_no_copy_approval`), in a new `CartFeedback` namespace:
   `added` ("{name} added to your order"), `addedOption` ("{name}
   ({option}) added to your order"), `viewCart` and `dismiss`. The name
   is bold through a rich-text `<strong>` tag.
9. **No new library.** GSAP, Tailwind and plain React cover all of it,
   so `tech-stack.md` doesn't change.

## Out of scope

- Undo in the toast, or changing the quantity from it. `/cart` already
  does both.
- A mini-cart drawer or popover.
- Haptic feedback (`navigator.vibrate`). It does nothing on iOS Safari,
  and Android-only feedback would be inconsistent.
- Feedback for quantity changes on `/cart`, the order flow and catering
  quotes.
- Changing the bento tile layout or the add button's size (Phase 25).

## Open risks flagged during planning

1. **The fly and toast wording are defaults** (Decisions 4 and 6). The
   user answered the mobile-cart and roadmap questions but not these
   two. Changing to a brand droplet, or adding the count and total to the
   toast, is a small change at review.
2. **The mobile header gets more crowded**: logo, cart, menu. At 320px
   (Mobile S) the logo has to keep its size without wrapping. It's
   checked in the Phase 25 device simulator (validation).
3. **The toast vs the WhatsApp float button on small phones.** They
   could overlap at 320px. *Mitigation:* the toast is capped at
   `100vw - 7rem` and centred, which leaves the 56px button plus its
   24px offset clear. Checked at 320px and 375px (validation).
4. **The fly element and the page-transition screen.** The fly element
   (`z-[60]`) sits above the transition screen (`z-40`), and "View cart"
   triggers a page transition. The fly ends about 650 ms after the add,
   so a fast tap on View cart could show a photo over the cream screen
   for a moment. *Accepted:* it's brief and harmless. The fly is removed
   on unmount too.
