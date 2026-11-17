# Phase 30 — Add-to-Cart Feedback: Validation

## Pass/fail checklist

### Build

- [ ] `pnpm check:messages` passes with the new `CartFeedback` keys in
      en, nl and fr.
- [ ] `pnpm lint` and `pnpm build` pass in `apps/web`, with no `any`.
- [ ] `/validate` passes.
- [ ] `git diff --stat master -- apps/api` is empty (web only).

### Mobile cart icon

- [ ] At 320, 375 and 430px (simulator presets Mobile S, M, XL): the
      header shows the logo, a cart icon and the menu button on one row,
      with nothing wrapped or clipped.
- [ ] With items in the cart, the mobile header icon shows the count.
      Opening the menu doesn't show a second cart link.
- [ ] From 769px up there's exactly one cart icon, in the nav row, as
      before.
- [ ] `document.querySelectorAll("[data-cart-target]")` returns two
      elements, and exactly one has client rects at any width.

### Add feedback (flat-price item)

- [ ] Clicking "+" on `/menu` (desktop and phone): droplets plus an
      expanding ring, the "+" turns to "✓" for about 1.2 s, the dish photo
      flies into the visible cart icon, and the badge bounces as it lands.
- [ ] The toast shows "**Name** added to your order" with View cart and
      ×. It's bottom centre on a phone, clear of the WhatsApp button at
      320px, and top right under the header on desktop. It hides after
      about 3.5 s, and stays while hovered or focused.
- [ ] View cart opens `/{locale}/cart` with the item there.
- [ ] Clicking "+" three times quickly adds three, shows one toast
      (restarted), and the count ends at three.
- [ ] Same on the homepage featured section.

### Add feedback (item with price options)

- [ ] Choosing an option in the dialog adds it. The dialog closes, the
      toast says "**Name (Option)** added to your order", and the fly
      starts from the tile's "+".

### No false celebrations

- [ ] Reloading a page with items already in the cart: no toast, no
      bounce.
- [ ] Changing quantity or removing on `/cart`: no toast or fly. The
      badge updates without bouncing when the count goes down.
- [ ] In the admin live preview, the add buttons stay disabled and
      nothing animates.

### Reduced motion

- [ ] With "Reduce motion" on (OS setting, or DevTools → Rendering →
      `prefers-reduced-motion: reduce`): no ring, no fly, no bounce. The
      ✓ swap and the toast still show, without sliding.

### Accessibility

- [ ] With VoiceOver or NVDA, adding an item announces "Name added to
      your order" once, without moving focus.
- [ ] The toast's View cart and × can be reached with Tab and work with
      Enter. × hides it.
- [ ] The en, nl and fr toasts show the right translations
      (`/nl/menu`, `/fr/menu`).

## Definition of done

Adding anything to the cart, on a phone or a computer, gives
unmistakable feedback. The button splashes and shows a ✓, the dish photo
flies into the cart icon, which is now always visible in the mobile
header, the count bounces, and an "added to your order" toast with a
View cart link appears in the visitor's language and is announced to
screen readers. Users who prefer reduced motion get the ✓ and the toast
without animation. Loading the page or editing the cart never triggers
it. No new library, no API change, and lint, build, `check:messages` and
`/validate` pass.
