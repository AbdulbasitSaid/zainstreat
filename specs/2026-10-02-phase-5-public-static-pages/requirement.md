# Phase 5 — Public Static Pages: Requirements

## Source

`specs/roadmap.md`, Phase 5 (amended during this planning pass to fold in
the Services page — see Decision 2 below):

> - Home, About, Services, Contact (static shell), Terms & Conditions, Food
>   Regulations pages — no dynamic data yet.
> - Services page (Meals, Snacks, Catering, Event Rentals detail sections
>   per README §12, each with a "Request a Quote" CTA linking to Contact —
>   the enquiry form itself is Phase 12's job).
> - Brand system applied: color tokens, typography, logo placement, base
>   layout/navigation (desktop + mobile hamburger).

## Context

Phases 1 (scaffolding), 2 (AWS deployment), 3 (Dutch/English i18n), and 4
(Data Model) are merged to `master`. `apps/web` is still exactly Phase 3's
output: a single locale-prefixed route (`app/[locale]/page.tsx`) rendering
a hardcoded "hello" headline/subtitle plus a `<LocaleToggle>` client
component, with `i18n/{navigation.ts,request.ts,routing.ts}` and
`proxy.ts` providing the locale-routing infrastructure. No header, nav, or
footer component exists. `@picocss/pico@^2` is already a dependency
(added in Phase 1) and already imported in `apps/web/app/globals.css`
(`@import "@picocss/pico/css/pico.min.css";` — the file's only line), but
no brand color override layer, no typography pairing, and no logo asset
exist on top of it. `apps/web/public/` contains only a `.gitkeep` — no
logo file has ever been supplied despite README describing one as "the
supplied logo."

This phase has no dependency on `apps/api` or the Phase-4 schema — it is
purely static markup/content, which is why it can run independently of
Phase 6 (the first phase to query the database).

## Decisions

Four genuine open questions were identified during planning and put to
the user directly; all four were resolved to the option below.

1. **Logo asset.** README describes a supplied logo (burgundy/crimson
   typography, chef hat, rolling pin, circular emblem) as the primary
   brand asset, but no such file exists anywhere in the repo.
   **Decision: ship a styled text wordmark behind a `<Logo>` component
   now** (brand color + display typeface), structured so that dropping a
   real `public/logo.svg`/`.png` in later and pointing `<Logo>` at it is a
   one-line change, not a redesign.

2. **Services page gap.** `specs/roadmap.md`'s original Phase 5 bullet
   omitted Services, but README §§8, 9.1, 12, 44 all treat it as a
   permanent top-level nav item with its own dedicated page, and no phase
   in the original 15-phase roadmap ever scheduled building it — a gap
   between the README's product spec and the roadmap's phase breakdown.
   **Decision: fold `/services` into Phase 5** (this phase's roadmap
   bullet was amended accordingly) since README §12 already has full
   static content for it (Meals/Snacks/Catering/Event Rentals, each with
   a "Request a Quote"/"Request Catering Quote" CTA) — the same
   no-database-dependency shape as every other page in this phase.

3. **Missing real business info.** The Contact page needs a phone number,
   WhatsApp number, address, and opening hours; Terms & Conditions and
   Food Regulations need real specifics (e.g. a halal-certifying body,
   actual hygiene practices) — none of which exist in any spec file or
   README section. **Decision: ship clearly-labeled placeholders**
   (e.g. "Phone: [to be added]") and generic README-derived copy for
   Terms/Food Regulations with a visible draft/review-pending notice,
   rather than inventing business facts. Filling these in for real is
   tracked as a fast-follow, not re-litigated per phase.

4. **Dutch translation volume.** Phase 3 established a best-effort/
   placeholder translation precedent for a single subtitle string; Phase 5
   adds full page copy across six routes (nav, footer, hero, About,
   Services, Contact, Terms, Food Regulations). **Decision: keep the same
   best-effort-placeholder approach** for Dutch copy in this phase,
   flagged for a later native-speaker review pass, rather than blocking
   merge on professionally reviewed translations at this stage.

Additional implementation decisions (not genuine forks — sensible,
reversible defaults, not put to the user):

5. **Pico CSS custom-property remapping, not a rewrite.** *(Superseded —
   see "Addendum — Tailwind CSS v4 migration" below; kept here for
   history.)* Brand colors (README §3) are layered on top of the existing
   Pico import via CSS custom properties, remapping the subset Pico itself
   reads (`--pico-primary*`) so built-in Pico components (buttons, links,
   form focus rings) inherit the brand with no per-component overrides.

6. **`next/font/google` self-hosted pairing** — a strong display face for
   headings, a clean sans face for body copy (README §34). Exact typeface
   choice is low-stakes and reversible; not treated as a blocking decision.

7. **Light-theme-only** (`color-scheme: light` pinned in `:root`, no
   dark-mode toggle component). Strongly implied by README §34's "white
   main background, food photography does the visual work" direction;
   reversing this later is a small CSS change, not a redesign.

8. **Nav item set: Home / About / Services / Contact / Order Now.** Menu
   is intentionally omitted from the nav until Phase 6 builds `/menu` —
   a nav link to a page that doesn't exist yet would be worse than a
   slightly incomplete nav for one phase.

9. **`<LocaleToggle>` relocates from the homepage into the new header**,
   closing the forward-reference Phase 3 explicitly left in
   `docs/local-development.md` ("once a real header/nav exists (Phase 5),
   relocate it there").

## Out of scope

- Dynamic menu data and the `/menu` route (Phase 6).
- Cart (Phase 7).
- Working form submissions: the Contact form, the Services page's
  "Request a Quote" CTAs, and any catering/event enquiry workflow render
  as static markup only — no API wiring (Phases 8 and 12).
- Admin auth and dashboard (Phases 9–11).
- Real logo asset, real business contact details, legally-reviewed Terms
  copy, and professionally reviewed Dutch translations — explicitly
  deferred per Decisions 1, 3, 4 above, tracked as fast-follows rather
  than blocking this phase.
- Accessibility audit, SEO metadata polish, and image-optimization pass
  beyond baseline `next/image` usage (Phase 14).
- A map embed on the Contact page (no real address exists yet to plot).
- `apps/api` changes of any kind — this phase is `apps/web`-only.

## Open risks flagged during planning

- Placeholder contact details, Terms copy, and Dutch translations all
  need a tracked follow-up before a real production launch — none of
  this phase's placeholder content should be mistaken for launch-ready.
- The Services page's "Request a Quote" CTAs and the Catering CTA both
  link to `/contact`, which itself has no working form submission yet —
  by design (Phase 12 builds the real enquiry workflow), but worth noting
  so it isn't mistaken for an oversight during review.

## Addendum — real brand assets supplied

Before this phase merged, the real logo and a professional marketing
banner were supplied, resolving part of Decisions 1 and 3 above:

- **Decision 1 (logo) resolved.** The real circular logo asset now ships
  at `apps/web/public/logo.jpg` and renders via `<Logo>`
  (`apps/web/components/logo.tsx`), replacing the text wordmark. Also used
  for `apps/web/app/icon.png` (favicon).
- **Color accuracy.** Pixel-sampling the logo and banner confirmed
  README's `--color-primary` (`#A7041B`) and `--color-accent` (`#F93173`)
  already matched the real logo closely. The banner's sampled dark color
  (`#71013B`, a wine/magenta) differed from README's documented
  `--color-primary-dark` (`#5E0407`, a brownish red); the user chose to
  adopt the banner's value. README §3 and `globals.css` were updated to
  `#71013B`.
- **Decision 3 (contact details) partially resolved.** Phone and WhatsApp
  number (`+31 6 30545277`, used for both) and an Instagram handle
  (`@zain_treats`) are now real and shown on the Contact page and the
  homepage's WhatsApp CTA. A TikTok handle (`@Zain's Treats.nl`) is shown
  as text only — not linked, since its URL-safe handle isn't known.
  Business address and opening hours remain genuinely unknown and still
  show `[to be added]`-style placeholders.

## Addendum — motion/animation system

After this phase's static shell shipped, the user asked for the public
pages to feel more "award-winning" — real animation, transitions, and
hover/click feedback, referencing sites like elevaremarket.com,
burgerfuel.com, and awwwards.com — rather than the completely static
baseline this phase originally delivered. This is treated as an extension
of this phase's own "brand system applied... base layout/navigation" scope
rather than a new phase, since no new product functionality is involved.

Decisions (confirmed with the user):

- **Scope:** the animated treatment applies to all 6 pages, with Home/
  About/Services/Contact getting the richer treatment (scroll reveals,
  decorative shapes, image slots) and Terms/Food Regulations a calmer one
  (plain fade-up only, no decorative motion) — legal/compliance pages
  should read as trustworthy and scannable, not flashy.
- **No real food photography exists yet.** The visual "pop" comes from
  motion, color, shape, and typography instead. A new `<ImageSlot>`
  component (`apps/web/components/image-slot.tsx`) extends the existing
  `[to be added]`-style placeholder pattern (Decision 3 above) to imagery —
  swapping in a real photo later is a one-line change per call site.
- **Library choice: Motion (`motion`, formerly Framer Motion) + Lenis**
  (smooth scroll), not GSAP, not Tailwind — see `specs/tech-stack.md`'s
  Frontend section for the full rationale. Both are fully typed, so this
  doesn't conflict with the no-`any` rule.
- **Decorative motifs are simple CSS-drawn circles for v1**, not a custom
  SVG illustration — fast, zero asset risk, swappable once real brand
  illustration/photography exists.
- **Animated page-to-page route transitions are included** (a fade on
  navigation, via `apps/web/components/page-transition.tsx`), not deferred.
- **`prefers-reduced-motion` is respected from the start** (not deferred to
  the Phase 14 accessibility audit): every animating component checks
  Motion's `useReducedMotion()`, Lenis never initializes under reduced
  motion, and a blanket CSS safety net zeroes any remaining
  animation/transition durations.
- **The mobile hamburger panel's underlying mechanism changed**: the
  `hidden` attribute (toggled previously) is replaced with the native
  `inert` attribute, with Motion animating `height`/`opacity` for the
  open/close transition instead of a mount/unmount. The desktop
  `@media (min-width: 769px)` override in `globals.css` was updated
  accordingly (forces `height`/`opacity` rather than overriding `[hidden]`)
  so desktop nav visibility is unaffected. `validation.md`'s existing
  hamburger checklist item is updated to reference `inert` instead of
  `hidden`.
- **Per-item stagger inside list/grid groups (e.g. `.trust-indicators`,
  `.services-overview`) is explicitly out of scope for v1** — each group
  gets one unified fade-up via `<Reveal>` rather than per-item staggered
  reveals, to avoid converting more server-rendered markup into client
  components than necessary. Flagged as a possible follow-up, not silently
  dropped.

## Addendum — Tailwind CSS v4 migration

After the brand system and motion/animation work above shipped, the user
found the UI "not coming out well" under Pico and asked to replace it with
Tailwind CSS before this phase merges, superseding Decision 5. This wasn't
only a tooling swap: Pico's classless model was already being fought —
the footer newsletter input stripped Pico's input chrome with `!important`
overrides and shipped with **no replacement focus-visible style at all** (a
genuine WCAG 2.4.7 gap, not a style preference), and a single blanket
`a[role="button"], button { ... }` tag-selector rule styled every button on
the page indiscriminately. Decisions (confirmed with the user):

- **Scope: framework swap *and* a real redesign pass**, not a 1:1 utility
  reproduction, specifically on nav/header/hamburger, forms, buttons/CTAs,
  and overall layout/spacing/cards. The brand palette and the two
  `next/font/google` typefaces (Work Sans body / Fraunces display) are
  preserved exactly — the freedom granted is for layout, spacing, component
  shapes, and interaction patterns, not colors or fonts.
- **Tailwind CSS v4, CSS-first.** `apps/web/app/globals.css` now does
  `@import "tailwindcss";` followed by an `@theme` block carrying the full
  README §3 brand palette plus layout/motion tokens as native Tailwind
  design tokens (`bg-primary`, `rounded-card`, `max-w-brand`, `py-section`,
  `ease-out-expo`, `animate-marquee`, `animate-float-rotate`) — no
  `tailwind.config.ts`; `apps/web/postcss.config.mjs` registers
  `@tailwindcss/postcss`. `next/font/google`'s own `--font-display`/
  `--font-body` custom properties are deliberately kept outside `@theme`
  (that namespace would collide with Tailwind's own `--font-*` tokens) and
  wired in `@layer base` exactly as before.
- **New shared components replace most of the old global CSS classes**:
  `components/button.tsx` (`<Button>`/`<ButtonLink>`, 4 variants, replacing
  the blanket button tag-selector rule and several ad hoc overrides),
  `components/split-row.tsx` (the alternating text/image layout, folding in
  the old `.section-band` tint as a boolean prop), `components/
  check-list.tsx`, `components/notice.tsx`, `components/page-hero.tsx`, and
  `components/eyebrow.tsx`. `.container` and `.field` remain the only
  named classes, kept in `@layer components` since both apply identically
  across every call site with no conditional logic.
- **Accessibility fix riding along with the redesign**: the footer
  newsletter input's missing focus-visible state (see above) now gets the
  same visible accent-colored focus ring as every other form field.
- **Nav/header mechanics are unchanged** — `aria-expanded`/`aria-controls`/
  `aria-label`/`inert`/the `useSyncExternalStore` desktop-query check/the
  pathname-close effect all carry over verbatim; only the hamburger's
  visuals (now an animated glyph-to-X), the mobile panel's surface
  treatment, and a new active-route highlight changed. The old desktop
  `!important` height/opacity override (needed because Motion drives those
  same properties via inline style) is now Tailwind's `!`-prefixed
  important modifier — the one easy-to-drop-by-accident mechanical detail
  carried over from the original implementation.
- **`.container` gained explicit centering/padding** (`margin-inline: auto`
  plus responsive `padding-inline`) that Pico's own built-in `.container`
  utility class had been silently supplying underneath the brand override
  (the project's own `.container` rule only ever set `max-width`); removing
  Pico removed that free centering, so it's now set explicitly.

## Addendum — logo revision and burgundy rebrand

The client supplied a revised logo (a new wordmark + chef-hat + rolling-pin
lockup, on a solid black background) and asked for the brand to shift from
the crimson/hot-pink palette documented in the addendum above to
burgundy-primary with white as a prominent color and pink used only as a
minor accent:

- **Logo replaced.** `apps/web/public/logo.jpg` was removed in favor of
  `apps/web/public/logo.png`: the new artwork was background-matted to a
  transparent PNG with `rembg` (AI matting, not a chroma-key/fuzz cutout —
  the glossy dark-red edges needed real alpha matting to avoid halos), then
  recolored in HSV space (deep fills hue-shifted to a burgundy ~332°; the
  brighter pink script/rolling-pin fill desaturated to a muted blush; the
  near-white highlights pushed to true white). `apps/web/app/icon.png`
  (favicon) was regenerated from the same recolored artwork, kept
  transparent (256×256).
- **No more circular crop.** The new logo is a wide wordmark lockup, not a
  circular emblem, so `components/logo.tsx` dropped the `rounded-full`
  48×48 treatment in favor of showing the full lockup at its native aspect
  ratio (53×48 display size, `width`/`height` on `next/image`).
- **Brand tokens re-sampled from the recolored logo.** `globals.css`'s
  `@theme` block and README §3 were both updated: `--color-primary` →
  `#72123C`, `--color-primary-dark` → `#4E0C2A`, `--color-primary-light` →
  `#8A1E4E`, `--color-accent` → `#C66C84`, `--color-accent-light` →
  `#F0AEC0`. `--color-text`/`--color-text-muted` and every other structural
  token (`--color-background*`, `--color-cream`, `--color-card`,
  `--color-whatsapp*`, `--color-on-dark-*`) were left untouched — they
  already read as burgundy-compatible and no component hardcodes a hex
  value, so every `bg-primary`/`text-accent`/etc. call site picked up the
  new palette automatically.
