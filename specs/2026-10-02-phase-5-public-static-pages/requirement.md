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

5. **Pico CSS custom-property remapping, not a rewrite.** Brand colors
   (README §3) are layered on top of the existing Pico import via CSS
   custom properties, remapping the subset Pico itself reads
   (`--pico-primary*`) so built-in Pico components (buttons, links, form
   focus rings) inherit the brand with no per-component overrides.

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
