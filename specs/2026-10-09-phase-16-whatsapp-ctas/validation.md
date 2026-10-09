# Phase 16 — WhatsApp CTAs: Validation

## Pass/fail checklist

### Frontend build

- [ ] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new/changed
      file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean.
- [ ] `cd apps/web && pnpm build` — succeeds; no new routes (this phase
      adds no page/route, only edits existing ones plus a new
      `lib/whatsapp.ts` module).
- [ ] `grep -rn "31630545277" apps/web/app apps/web/components` shows the
      literal only inside `apps/web/lib/whatsapp.ts` (and the unrelated
      `tel:`/display-text occurrences already in `contact/page.tsx` for
      the phone number, which this phase doesn't touch) — not duplicated
      anywhere else (plan.md Group 1).
- [ ] `grep -rn "wa.me" apps/web/app apps/web/components` shows every
      remaining literal `https://wa.me/...` string is gone, replaced by
      `buildWhatsAppLink(...)` calls (plan.md Groups 3-6).

### Live dev stack (`docker compose up`) — manual smoke test

- [ ] **Header:** no WhatsApp icon/list-item appears in the header nav on
      either desktop (≥769px) or mobile (<769px, hamburger panel) — the
      header reverts to its pre-Phase-16 shape (requirement.md Decision 4,
      revised).
- [ ] **Floating button, all pages/breakpoints:** a circular green
      WhatsApp button using the official brand glyph is fixed to the
      bottom-right corner of the viewport, visible on scroll on every
      public page (`/`, `/menu`, `/cart`, `/order`, `/services`,
      `/contact`); clicking it opens
      `https://wa.me/31630545277?text=...` in a new tab, with the
      prefilled message visible in WhatsApp's own compose box (or the
      `wa.me` landing page if not logged into WhatsApp Web).
- [ ] **Floating button doesn't obscure critical UI:** check it doesn't
      overlap the cart/checkout CTA on `/cart` and `/order`, or the
      contact form's submit button on `/contact`, at common viewport
      sizes (mobile ~375px, desktop ~1440px).
- [ ] **Hero:** `/en` and `/nl` — the existing "WhatsApp Us"/"WhatsApp Ons"
      hero button now opens in a new tab (previously navigated away in the
      same tab) with the prefilled message.
- [ ] **Footer:** `/en` and `/nl` — the existing footer WhatsApp icon still
      looks unchanged, but its link now carries the prefilled message.
- [ ] **Contact page:** `/en/contact` and `/nl/contact` — the WhatsApp line
      in the contact-details list still looks unchanged, but its link now
      carries the prefilled message.
- [ ] **Dutch locale:** confirm the prefilled message that actually lands
      in WhatsApp's compose box is the Dutch copy on `/nl/*` pages and the
      English copy on `/en/*` pages — not a hardcoded English string
      leaking through on `/nl`.
- [ ] **All four links still point at the same number:** `31630545277` in
      every case (no typo introduced while centralizing into
      `lib/whatsapp.ts`).

### Regression

- [ ] `/validate`-equivalent sweep (lint + build + live HTTP smoke test) —
      no regression on `/`, `/en`, `/nl`, `/en/menu`, `/en/cart`,
      `/en/order`, `/en/services`, `/en/contact`, `/api/categories`,
      `/api/menu-items`, `/health`, the Phase 11 login/logout loop, and
      Phase 13/14/15's admin flows (this phase touches no admin or API
      code, so these are sanity checks, not expected-to-break areas).
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

Hero, contact page, and footer links to the same business WhatsApp
number through one shared `apps/web/lib/whatsapp.ts` helper, each opening
a new tab with the same translated, prefilled conversation starter rather
than a bare, contextless chat window. The header-nav placement originally
built for this phase is removed; in its place, a single global floating
WhatsApp button (official brand glyph, `--color-whatsapp` background)
sits fixed in the bottom-right corner on every public page
(requirement.md Decision 4, revised). Nothing about this phase touches
`apps/api`, the database, or any other public-facing page's visual layout
beyond that one floating button.
