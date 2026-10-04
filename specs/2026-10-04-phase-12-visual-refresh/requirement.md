# Phase 12 — Visual & Interactive Refresh: Requirements

## Source

Not an original roadmap phase — inserted ahead of Phase 13 (Admin: Orders)
at the client's explicit, urgent request to make the public site "more
interactive and more beautiful" before the admin-dashboard build-out
continues. `specs/roadmap.md`, Phase 12 (as written by this phase):

> Not an original roadmap phase — inserted ahead of the admin-dashboard work
> at the client's request, to make the public site "more interactive and
> more beautiful" before continuing the MVP build-out.
>
> - Remove the decorative "floating shapes" (`DecorativeShape`) entirely from
>   the homepage and about page — no replacement decoration, just gone.
> - Replace Framer Motion (the `motion` package) with GSAP + ScrollTrigger as
>   the site's animation system, for page transitions and scroll-triggered
>   reveals; Lenis (smooth scroll) stays, driven off `gsap.ticker`.
> - Fill in the site's missing imagery (homepage, about, services, contact)
>   with seeded stock/placeholder photography until real brand photography is
>   available — real photos remain a Phase 17 "Image optimization pass"
>   follow-up (or sooner, once the client supplies them).

## Context

Three concrete asks came out of the client conversation:

1. The decorative floating shapes (`DecorativeShape`, a circle that bobs and
   rotates via a CSS keyframe animation) read as distracting rather than
   polished — the client wants them gone entirely, with no replacement
   decoration.
2. The client wants GSAP-driven animation for a more interactive, "fluid"
   feel — page transitions and scroll reveals in particular.
3. Most of the site still renders text-label `ImageSlot` placeholders
   instead of real images (hero, category tiles, about page, every Services
   section, the Contact page map) — the client wants it to "feel complete."

This phase branches off `master` (Phase 11 — Admin Auth — is already
merged; this phase touches no file unique to any other unmerged branch).

## Decisions

1. **Formally inserted phase, following the Phase 8 (Telemetry) precedent.**
   Own branch + `specs/` folder, taking the "Phase 12" slot and bumping
   every phase from the previous Phase 12 (Admin: Orders) onward by one
   (now Phase 13–18). Chosen over an ad-hoc branch so the roadmap/spec-kit
   history stays consistent and this work is properly checked off like any
   other phase.

2. **GSAP fully replaces Framer Motion.** The `motion` package (Framer
   Motion's current package name) is removed entirely; GSAP + `@gsap/react`
   (`useGSAP`) + ScrollTrigger become the one animation system. Rejected
   layering GSAP alongside Framer Motion — that would ship two animation
   libraries long-term for no real benefit. Lenis (smooth scroll) is kept
   and rewired to drive off `gsap.ticker` instead of its own
   `requestAnimationFrame` loop — the standard, documented GSAP + Lenis +
   ScrollTrigger integration.

3. **Placeholder imagery reuses the project's existing `placehold.co`
   convention**, not a new external host. `apps/web/next.config.ts` already
   allowlists `placehold.co` (with the SVG CSP mitigation already in place)
   and `apps/api/seed.sql` already uses
   `https://placehold.co/600x400?text=...` for every menu item without a
   real photo. A new `PlaceholderImage` component reuses the exact same
   `label`/`className` prop signature as the existing `ImageSlot` component,
   so swapping is a one-line import change per call site with zero new
   remote-image-host configuration. Real food/brand photography isn't
   available yet; swapping these placeholders for real photos is explicitly
   deferred (client-supplied imagery, or Phase 17's "Image optimization
   pass").

4. **Placeholder imagery rolls out to every page currently showing an
   `ImageSlot`** used as decorative/content imagery: homepage (hero, 4
   category tiles, about-preview), about page (hero + mission), services
   page (all 4 sections), and the contact page map placeholder — not just
   the homepage and about page where the shapes are being removed. The
   `ImageSlot` fallback inside `MenuItemCard`/`CartLineItem` (used only when
   a real `image_url` is absent from the database) is explicitly untouched
   — that's live, DB-driven content, not static page decoration.

5. **`site-header.tsx`'s hamburger/mobile-nav animation is in scope**, even
   though the client didn't name it directly. It's the one remaining
   consumer of `motion/react` beyond `PageTransition`/`Reveal`; since
   Decision 2 removes the `motion` package entirely, this file must migrate
   to GSAP too or the build breaks.

6. **Page transitions are enter-only.** GSAP has no first-class
   AnimatePresence-style exit-then-enter crossfade for React unmount. A
   true exit animation would need to defer unmounting the outgoing route
   tree — real complexity for a marginal visual gain on a food-ordering
   site. The new route's content fades/slides in on mount (keyed by
   pathname); there's no fade-out of the previous route.

## Out of scope

- Real food/brand photography — explicitly deferred (see Decision 3).
- A real embedded map on the Contact page — the map `ImageSlot` becomes a
  placeholder *photo*, not a functioning map; a real map integration is
  future work.
- A true AnimatePresence-style exit animation for page transitions (see
  Decision 6).
- A GSAP hover micro-interaction (e.g. magnetic/tilt) on the homepage "Why
  choose us" 4-card grid — stays CSS-only `transform`/`shadow` hover; a
  later polish pass's job, not bundled into this animation-system swap.
- Admin routes (`apps/web/app/admin/**`) — confirmed zero `motion`/
  `Reveal`/`PageTransition` usage there today; left untouched.
- Any backend (`apps/api`) change — this phase is entirely `apps/web` +
  `specs/`.

## Open risks flagged during planning

1. **No SSR bake-in for GSAP's enter animations.** Framer Motion can bake
   `initial={{opacity:0}}` into server-rendered HTML; `gsap.from()` cannot —
   the server-rendered markup paints at full opacity first, and only once
   client JS runs does GSAP reset it before tweening up. This can read as a
   brief flash/reset rather than a smooth reveal, especially on slow
   connections. Accepted trade-off for this phase (see `plan.md`); the fix
   if it proves visually objectionable post-launch is a baked-in
   `opacity-0` class overridden by `gsap.set` on mount, not `gsap.from()`
   alone.
2. **ScrollTrigger + Lenis "double smoothing."** Must drive Lenis from
   `gsap.ticker` (not its own RAF), call `lenis.on('scroll',
   ScrollTrigger.update)`, and set `gsap.ticker.lagSmoothing(0)` — all three
   are required together, or scroll-linked triggers get jittery. See
   `plan.md` for the exact implementation.
3. **`gsap.ticker`/`lenis.raf()` unit mismatch.** `gsap.ticker` passes
   seconds; `lenis.raf()` expects milliseconds. A missing `* 1000` is a
   silent-failure bug (scrolling crawls, nothing throws) rather than a
   build error — called out explicitly in `plan.md` for implementation and
   review.

## Addendum — Real imagery, idle float, custom cursor, nav indicator

Groups 1–4 above landed in commit `3432423` ("migrated to gsap"). This
addendum is follow-up scope the client raised in the same conversation,
before this phase is merged — folded into Phase 12 rather than opened as a
new phase, per explicit instruction. Client's own words (paraphrased):
GSAP is in, but "we are yet to use the power of GSAP" — add real images
where placeholders are, nice GSAP animations including "some floating
item," "amazing navigation... GSAP transition states," and "a mouse
follow that interacts with every element."

### Decisions (continued)

7. **Real, AI-generated imagery replaces the `placehold.co` placeholder
   convention** (supersedes Decision 3) for every one of the 10 static-page
   call sites. Images are generated once and committed as static files
   under `apps/web/assets/images/`, imported via a Next.js static `import`
   (not `public/` + a manual URL string) so `next/image` gets automatic
   width/height and a built-in base64 blur placeholder for free. The
   DB-driven `ImageSlot` fallback in `MenuItemCard`/`CartLineItem` is
   untouched — Decision 4 still applies, different concern (no real
   `image_url` yet, nothing to do with static page decoration).
8. **Image reuse to minimize generation count.** The homepage's 4 category
   tiles and the Services page's matching 4 detail sections are the same
   category, so each pair shares one generated image
   (`category-meals.jpg`, `category-snacks.jpg`, `category-catering.jpg`,
   `category-event-rentals.jpg`) instead of 8 near-duplicates. The
   homepage's "about preview" `SplitRow` reuses the About page's "who we
   are" hero image (`about-hero.jpg`) rather than generating another
   near-duplicate. Net: **8 distinct generated images** cover all 10 call
   sites (hero, 4 category/services pairs, about-hero/about-preview pair,
   about-mission, contact).
9. **`PlaceholderImage` is replaced by a new `SiteImage` component**
   (`apps/web/components/site-image.tsx`); `apps/web/components/
   placeholder-image.tsx` is deleted once nothing calls it. `SiteImage`'s
   props change from `{ label, className }` (a visible placeholder
   caption) to `{ src: StaticImageData, alt, className?, priority?,
   float? }` — a deliberate break from Decision 3's "zero prop-shape
   change" goal, since a real image needs a real `src` and accessible
   `alt` text, not a caption string. All 10 call sites are updated
   accordingly, not just the import name. `priority` is set on the
   homepage hero only (it's the page's LCP element — the old
   `PlaceholderImage` never had this, so this is a genuine improvement
   that falls out of the swap).
10. **Idle "floating" motion is added only to the homepage hero image and
    the 4 category tiles** (not the about/services/contact images) — a new
    `float` prop on `SiteImage`: a slow (`duration: 3`, `ease:
    "sine.inOut"`, infinite yoyo), subtle vertical drift, wrapped in the
    same `gsap.matchMedia("(prefers-reduced-motion: no-preference)")`
    pattern `reveal.tsx`/`page-transition.tsx` already use. **This does not
    reverse Decision 1** — the abstract `DecorativeShape` stays removed,
    no decorative shape returns; this is motion applied to real content
    imagery, a different thing entirely.
11. **A sitewide custom cursor** (`apps/web/components/custom-cursor.tsx`),
    mounted once in `apps/web/app/[locale]/layout.tsx` next to
    `MotionProvider` (public site only — the admin layout is untouched,
    consistent with the original "admin routes left untouched" decision).
    A small dot tracks the raw pointer position; a trailing ring lags
    behind it and scales up over any `a, button, [role="button"], input,
    textarea, select, label`. Uses `gsap.quickTo` (cheap per-frame
    tweening) rather than calling `gsap.to` on every `mousemove`. Gated on
    **both** `prefers-reduced-motion: no-preference` (existing
    `useReducedMotion` hook) **and** `(pointer: fine)` (a new check, same
    `useSyncExternalStore`/`matchMedia` pattern `site-header.tsx` already
    uses for `isDesktop`) — on touch devices or under reduced motion the
    component renders nothing and the native cursor is never hidden. The
    native cursor is hidden only via a `body.custom-cursor-active` class
    toggled in a client effect — never a static CSS rule — so
    server-rendered HTML always paints with a normal cursor first. This is
    an independent decorative layer, unrelated to the "Why choose us"
    4-card grid's existing CSS-only hover lift (that per-card
    magnetic/tilt hover stays exactly as deferred in the original
    Out-of-scope list — different mechanism, still untouched).
12. **An animated active/hover indicator in the desktop nav**
    (`site-header.tsx`, `min-[769px]` breakpoint only — the mobile stacked
    menu is unchanged, still plain color-based active state). A new
    absolutely-positioned `<span>` inside the nav `<ul>`, GSAP-tweened
    (`x`, `width`, `ease: "outExpo"`) to match whichever `<li>`'s
    `getBoundingClientRect()` is currently hovered, snapping back to the
    active route's `<li>` on `mouseleave`/route change. This is the
    concrete answer to "amazing navigation... transition states" beyond
    the hamburger-toggle tweens and enter-only page transitions Group 2
    already shipped — those stay exactly as they are (no exit-transition
    rework; see Decision 6, still standing).

### Out of scope (addendum-specific)

- A real, functioning map on the Contact page — the contact image is now a
  real generated photo (e.g. storefront/delivery moment), not a map; same
  deferral as the original spec, just no longer phrased as "placeholder."
- Per-card magnetic/tilt hover on the "Why choose us" grid — still
  deferred (see Decision 11); the sitewide custom cursor doesn't
  substitute for it.
- A true `AnimatePresence`-style route exit animation — still deferred
  (Decision 6 stands; the nav indicator answers "amazing navigation," not
  a rework of `PageTransition`'s exit behavior).
- Any backend (`apps/api`) change.

### Open risks flagged during planning (addendum-specific)

1. **The image-generation mechanism itself isn't pinned here.** Whatever
   image-generation capability is available in the implementing
   session/environment at build time produces the 8 files; this spec only
   fixes their destination path, naming, dimensions, and content brief
   (see `plan.md`), not the generator. The actual photographic
   style/consistency is a quality judgment call for implementation,
   reviewed before merge — if it doesn't read as on-brand, regenerate
   before shipping rather than accepting a bad first pass.
2. **`gsap.quickTo` cursor tracking running alongside Lenis/ScrollTrigger
   on the same `gsap.ticker`.** Should be fine — `quickTo` is a normal GSAP
   tween under the hood — but worth a real-browser check that fast
   scrolling and fast cursor movement don't visibly fight each other;
   flagged for `validation.md`.
3. **Hiding the native cursor sitewide is an accessibility-adjacent
   choice.** Gating on `pointer: fine` AND `prefers-reduced-motion:
   no-preference` (Decision 11) is the mitigation, not a nice-to-have —
   don't loosen either condition without re-checking this risk.

## Addendum 2 — Real imagery for menu items (`apps/api/seed.sql`)

Client-requested follow-up ("seed images for all food in the menu"), raised
after Addendum 1 landed, before this phase is merged — folded in here
rather than opened as a new phase or spec, per the same "fold it into
Phase 12" precedent Addendum 1 established. Resolved with the user via
three up-front questions (phase framing, image source, reuse strategy)
before writing this addendum.

### Context

The original spec's Decision 4 and Addendum 1's Decision 9 both explicitly
left the DB-driven `ImageSlot`/`image_url` fallback on `MenuItemCard` and
`CartLineItem` untouched — that's live menu content, a different concern
from static-page decoration. `specs/roadmap.md`'s Phase 12 entry defers
"real photos" for menu items to Phase 17 ("Image optimization pass") "or
sooner, once the client supplies them." The client has now asked for this
sooner, for every menu item rather than real client-supplied photography.

`apps/api/seed.sql` (dev-only fixture data, Phase 7) currently has 28
`menu_items` rows. Two already use a real committed photo instead of
`placehold.co`: `Egusi Soup` → `/images/menu/egusi-soup.jpeg` and
`Efo Riro` → `/images/menu/efo-soup.jpeg` (both under
`apps/web/public/images/menu/`, referenced as a plain relative path string
in the `image_url` column — not the `apps/web/assets/images/` static-import
convention Addendum 1 established, since that convention is reserved for
site-decoration imagery per `tech-stack.md`; a SQL fixture column needs a
URL string, not a TypeScript `import`). The remaining 26 rows still use
`placehold.co`. Two other files already sit in that same folder —
`chin-chin.jpeg` and `small-chops.jpeg` — but no current `menu_items` row
references either; there is no "Snacks" category in `seed.sql` today (the
homepage/services "Snacks" category tile is a separate, marketing-only
concept from Addendum 1, unrelated to this DB table). Those two files are
unreferenced leftovers, not this addendum's concern.

### Decisions (continued)

13. **Scope stays inside this phase's existing branch/spec**, as a second
    addendum, rather than a new phase or an early start on Phase 17.
    Resolved with the user directly: same rationale as Addendum 1 — the
    client raised it live, before merge, and the spec-kit history stays
    simpler as one phase folder than a proliferation of near-duplicate
    phases. Phase 17's own "Image optimization pass" (formats, lazy
    loading, broader responsive/a11y polish) remains future work; this
    addendum only replaces `placehold.co` URLs with real photos.
14. **Real, AI-generated photography, same mechanism as Addendum 1's
    Decision 7**, not a new sourcing method. Resolved with the user
    directly. Carries the same caveat Addendum 1's `validation.md` already
    recorded: if no image-generation capability is available in the
    implementing session, the documented fallback is sourcing real,
    freely-licensed stock photography (verified individually for on-brand
    content and no third-party branding) instead of leaving `placehold.co`
    in place — never silently reverting to placeholders.
15. **Images are reused per base dish, not generated one-per-row.**
    Resolved with the user directly, same precedent as Addendum 1's
    Decision 8 (4 category images reused across 8 call sites). Several
    `seed.sql` rows are the same underlying dish at a different quantity,
    packaging, or near-identical protein swap (e.g. "Jollof Rice &
    Plantain with Chicken or Turkey" also appears as a 5 L bulk item and a
    "Cooler of Jollof Rice"; "Chicken Stew" and "Turkey Stew" are the same
    preparation with the protein swapped). The full reuse mapping — 16
    distinct images covering the 26 remaining rows — is in `plan.md`
    Group 9.
16. **File convention matches the existing `Egusi Soup`/`Efo Riro`
    precedent, not Addendum 1's static-import convention.** New files
    committed under `apps/web/public/images/menu/<slug>.jpg`, referenced
    in `seed.sql` as the plain relative path string `/images/menu/
    <slug>.jpg` in the `image_url` column — consistent with how
    `MenuItemCard`/`CartLineItem` already consume `image_url` as an
    arbitrary string passed straight to `next/image`, no remote-host
    allowlisting needed for a same-origin `public/` path. `.jpg` for the
    16 new files (the 2 existing real photos stay `.jpeg` — cosmetic only,
    not worth a rename-and-reseed detour).
17. **No `apps/api` code change.** `image_url` is already a plain
    nullable text column (Phase 4); this addendum only edits fixture data
    (`seed.sql`) and adds static files under `apps/web/public/`. The
    eventual production path for real per-item photos — admin-uploaded
    images landing in MinIO (per `tech-stack.md`'s architecture diagram
    and Phase 14's "Add/edit menu item... image" scope) — is unrelated and
    untouched; this addendum is dev-fixture-only, same spirit as the
    existing Egusi/Efo precedent it extends.

### Out of scope (Addendum 2-specific)

- `chin-chin.jpeg`/`small-chops.jpeg` — unreferenced leftover files in
  `apps/web/public/images/menu/`; left alone (see Context above).
- A "Snacks" `menu_items`/`categories` row — doesn't exist in `seed.sql`
  today; out of scope to add one here.
- Production/admin-uploaded MinIO imagery (Phase 14) — this addendum is
  dev-fixture-only (Decision 17).
- Phase 17's broader "Image optimization pass" (responsive formats, lazy
  loading audit, a11y pass) — unaffected; this addendum only swaps URLs.
- Any `apps/api` change (Decision 17).

### Open risks flagged during planning (Addendum 2-specific)

1. **Same image-generation-mechanism caveat as Addendum 1's open risk 1**,
   restated here because it already materialized once this phase (Addendum
   1's `validation.md` recorded falling back to sourced stock photography
   when no generation capability was available) — don't assume generation
   will succeed; `plan.md` Group 9 names the fallback explicitly so
   implementation isn't blocked if it recurs.
2. **16 images is still 16 reviews.** Reusing per base dish (Decision 15)
   cuts the count from 26 to 16, but each one still needs the same
   on-brand/no-mismatched-protein/no-broken-crop check Addendum 1's
   `validation.md` applied to the 8 site-decoration images — don't skip
   that review pass just because this batch is DB fixture data rather than
   static page content.
