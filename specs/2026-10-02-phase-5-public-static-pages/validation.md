# Phase 5 — Public Static Pages: Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

- [ ] `docker compose up` (or `pnpm dev` inside `apps/web`) starts cleanly
      with no build errors.
- [ ] `pnpm lint` and `pnpm build` (inside `apps/web`) both succeed with no
      `@typescript-eslint/no-explicit-any` violations and no TypeScript
      errors.
- [ ] `http://localhost:3000/en` renders the redesigned homepage: hero,
      trust indicators row, services overview, "why choose us," catering
      CTA, about preview — in English — with no "Featured Menu" section
      (deliberately deferred to Phase 6).
- [ ] `http://localhost:3000/nl` renders the same homepage sections in
      Dutch (best-effort translation, per requirement.md Decision 4).
- [ ] `http://localhost:3000/en/about`, `/en/services`, `/en/contact`,
      `/en/terms`, `/en/food-regulations` all return `200` and render
      page-specific content (not the homepage); the same five routes
      under `/nl` also return `200` with Dutch copy.
- [ ] `curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/en/<route>`
      returns `200` for each of: `about`, `services`, `contact`,
      `terms`, `food-regulations`.
- [ ] Header renders on every page: text wordmark logo, nav links
      (Home/About/Services/Contact), language toggle, Order Now CTA.
- [ ] Footer renders on every page with the dark-burgundy
      (`--color-primary-dark`) background and working locale-aware links
      to About/Services/Contact/Terms/Food Regulations.
- [ ] Resize the viewport below 768px (or use browser devtools device
      emulation): the nav list disappears, a hamburger button appears.
- [ ] Clicking the hamburger toggles `aria-expanded` (`false` → `true`) and
      reveals the nav list (`#primary-navigation` loses its `inert`
      attribute and animates open) — confirmed via devtools, not just
      visually.
- [ ] Navigating to another page while the mobile menu is open
      automatically closes it (no stale open menu after a route change).
- [ ] Resize back above 769px: the hamburger disappears and the full nav
      list is always visible regardless of toggle state.
- [ ] Keyboard-only pass: Tab reaches the hamburger button, Enter/Space
      toggles it, Tab continues into the revealed nav links in order — no
      keyboard trap.
- [ ] `<html lang="en">` / `<html lang="nl">` still correct on every new
      route (not just `/`), confirmed via page source.
- [ ] Services page shows Meals/Snacks/Catering/Event Rentals sections;
      the Catering and Event Rentals "Request a Quote" CTAs link to
      `/contact`.
- [ ] Contact page's form fields render (Name/Email/Phone/Subject/Message)
      but the submit button is disabled with the "not connected yet"
      notice visible — confirms the "static shell" scope wasn't
      accidentally wired to a real submission.
- [ ] Contact page's phone/address/hours fields show visible placeholder
      text (e.g. "[to be added]"), not invented business details.
- [ ] Terms page shows the legal-review-pending notice visibly (not
      buried).
- [ ] No regressions to Phase 3/4 behavior: `Accept-Language` redirect and
      `NEXT_LOCALE` cookie persistence still work from `/`; `apps/api`
      migrations/seed script untouched.
- [ ] `git diff master --stat` for this branch touches only
      `apps/web/app/**`, `apps/web/components/**`, `apps/web/messages/**`,
      `apps/web/app/globals.css`, `apps/web/package.json`,
      `apps/web/pnpm-lock.yaml`, `docs/local-development.md`,
      `specs/tech-stack.md`, `specs/roadmap.md`, and the new
      `specs/2026-10-02-phase-5-public-static-pages/` folder — nothing in
      `apps/api/**`, `docker-compose*.yml`, or `.github/workflows/**`.
- [ ] Manual contrast spot-check: brand crimson/burgundy text on white
      background and white text on the burgundy footer both read as
      comfortably readable (full WCAG AA audit is Phase 14's job).
- [ ] Contrast fix follow-up (2026-10-02): the WhatsApp hero button
      (`--color-whatsapp-dark`), muted body/caption text
      (`--color-text-muted`), and the footer's link/body/label/note tiers
      (`--color-on-dark-*`) were re-checked in devtools and all clear
      WCAG AA (≥4.5:1) — see `apps/web/app/globals.css` `:root` tokens.

## Motion/animation system (addendum)

- [ ] Chrome DevTools → Rendering → "Emulate CSS prefers-reduced-motion:
      reduce" (or OS-level Reduce Motion), reload every page: Lenis never
      initializes, `<Reveal>` content shows instantly (no fade-up delay),
      the hamburger and route transitions have effectively zero duration,
      and the decorative shape's float animation is absent.
- [ ] With no emulation (default motion): each page's `<Reveal>`-wrapped
      sections fade up as they scroll into view (not pre-visible on load,
      not stuck invisible after scrolling past them).
- [ ] Decorative shapes and image slots appear only on Home, About,
      Services, and Contact — confirmed absent on Terms and Food
      Regulations via devtools inspection.
- [ ] Button/CTA/nav/footer links show a visible hover lift and press
      state, and a visible focus ring when tabbed to.
- [ ] Hamburger panel animates open/closed smoothly below 768px; resizing
      back above 769px still always shows the full nav regardless of the
      toggle's last state (the `inert`-based rework didn't regress this).
- [ ] Keyboard-only pass still holds: Tab reaches the hamburger, Enter/
      Space toggles it, Tab continues into the revealed nav links in
      order, no keyboard trap.
- [ ] Navigating between pages shows a brief fade transition; scroll
      position resets to the top of the new page as before.
- [ ] Scrolling with a mouse wheel/trackpad shows the Lenis smooth-scroll
      feel; keyboard scrolling (arrow keys, Page Down, Space) and
      screen-reader scrolling still work natively and aren't degraded.
- [ ] `pnpm lint` and `pnpm build` both succeed with no
      `@typescript-eslint/no-explicit-any` violations (confirms no
      accidental `motion/react` import inside a Server Component, which
      would otherwise fail the build).

## Tailwind CSS v4 migration (addendum)

- [x] `pnpm lint` and `pnpm build` (inside `apps/web`) both succeed with no
      `@typescript-eslint/no-explicit-any` violations and no TypeScript
      errors.
- [x] All 6×2 locale routes (`/`, `about`, `services`, `contact`, `terms`,
      `food-regulations` under both `/en` and `/nl`) return `200` via
      `pnpm start` (or `node .next/standalone/server.js`).
- [x] No `@picocss/pico` CSS ships in the built output — confirmed zero
      `--pico-*` properties in the generated stylesheet, and zero
      occurrences of "pico" in rendered page HTML.
- [x] Sample `@theme`-generated utilities exist in the built CSS
      (`bg-primary`, `rounded-card`, `ease-out-expo`, `animate-marquee`)
      and the `@theme` tokens resolve to the correct README §3 values
      (spot-checked `--container-brand`, `--color-whatsapp-dark`).
- [x] `<html lang="en">`/`<html lang="nl">` still correct on every route
      (confirmed via page source).
- [x] Decorative shapes and image slots still appear only on Home/About
      (hero + about-preview) and Services/Contact have none — unchanged
      from before this migration (pre-existing, not a regression).
- [x] Contact page's submit button still renders `disabled` with the
      "not connected yet" `<Notice>` visible; footer newsletter form
      fields still render `disabled`.
- [ ] **Manual — not checked here** (no connected browser this session):
      hamburger open/close animation and the new animated-X glyph;
      visible focus-visible ring on every form field and the previously
      unstyled footer newsletter input (the accessibility fix); keyboard-
      only tab order through the redesigned nav; resize behavior exactly
      at 768/769px (the `min-[769px]:!h-auto`/`!opacity-100` Tailwind
      important-modifier rewrite of the old desktop `!important` override);
      `prefers-reduced-motion: reduce` emulation; WCAG AA contrast on the
      new/changed interactive states (focus rings, active-route nav
      highlight); narrow (360px) and wide (1920px+) viewport layout; Dutch
      copy length not breaking the tightened nav/button spacing. Re-run
      these the same way the original Phase 5 checklist above specifies,
      against the redesigned markup.

## Definition of done

Phase 5 is complete when every box above is checked: Home, About,
Services, Contact (structural shell, no working submission), Terms, and
Food Regulations all render correctly in both locales under
locale-prefixed routes; a shared header (wordmark logo, nav, language
toggle, Order Now CTA, working mobile hamburger) and footer render on
every public page; the brand palette from README §3 is wired through
Tailwind CSS v4's `@theme` design tokens (`apps/web/app/globals.css`)
rather than hardcoded colors scattered through components; all placeholder
content (logo, contact details, Terms
wording, Dutch translations) is visibly flagged rather than presented as
final; no `apps/api` or deployment surface was touched; and
`specs/roadmap.md`/`specs/tech-stack.md` reflect what was actually built.
Ready for Phase 6 (Menu Browsing) to add the homepage's "Featured Menu"
section and the `/menu` route on top of this layout.
