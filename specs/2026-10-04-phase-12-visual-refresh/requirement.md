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
