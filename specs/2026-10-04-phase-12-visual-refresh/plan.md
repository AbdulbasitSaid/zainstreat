# Phase 12 — Visual & Interactive Refresh: Implementation Plan

Group 1 removes the decorative shapes; Group 2 replaces Framer Motion with
GSAP; Group 3 rolls out placeholder imagery; Group 4 is the roadmap
renumbering and cross-reference pass.

## Group 0 — Branch

Branched `2026-10-04-phase-12-visual-refresh` off `master` (Phase 11 —
Admin Auth — already merged; this phase touches no file unique to any
other unmerged branch).

## Group 1 — Remove the floating shapes

- Deleted `apps/web/components/decorative-shape.tsx`.
- `apps/web/app/globals.css`: removed `--animate-float-rotate` (in
  `@theme`) and the `@keyframes float-rotate` block. `--animate-marquee`/
  `marquee-scroll` untouched (unrelated, still used elsewhere).
- `apps/web/app/[locale]/page.tsx`: removed the `decoration` prop (2
  shapes on the hero `SplitRow`, 1 on the catering-CTA section), the
  `DecorativeShape` import, and the now-pointless `relative
  overflow-hidden` classes (both call sites now just use `className="container"`,
  and the catering-CTA section uses plain `className="container"`).
- `apps/web/app/[locale]/about/page.tsx`: same — removed `decoration` and
  the `DecorativeShape` import on the hero `SplitRow`. That `SplitRow`
  previously passed `className="relative overflow-hidden"` for no reason
  beyond containing the shape, so the `className` prop is dropped entirely
  (not just the two classes) — `<main className="container">` already
  provides the row's horizontal layout.
- `apps/web/components/split-row.tsx`: removed the now-unused
  `decoration?: ReactNode` prop and its render line. Confirmed via grep
  that no other call site used it.

## Group 2 — Replace Framer Motion with GSAP

New files:
- `apps/web/lib/gsap.ts` — single registration point: imports `gsap`,
  `ScrollTrigger` (`gsap/ScrollTrigger`), `CustomEase` (`gsap/CustomEase`),
  and `useGSAP` (`@gsap/react`); calls `gsap.registerPlugin(...)` once at
  module scope; defines `CustomEase.create("outExpo", "0.22, 1, 0.36, 1")`
  matching the existing `--ease-out-expo` design token in `globals.css`.
- `apps/web/lib/use-reduced-motion.ts` — `useSyncExternalStore` +
  `matchMedia("(prefers-reduced-motion: reduce)")` hook, mirroring the
  `DESKTOP_QUERY` pattern already in `site-header.tsx`. Used anywhere a
  plain boolean is needed outside a GSAP timeline (Lenis setup, the
  header's toggle tweens); inside one-shot scroll reveals, `gsap.matchMedia()`
  is used directly instead (`reveal.tsx`, `page-transition.tsx`).

Rewrites:
- `apps/web/components/motion-provider.tsx` — keeps Lenis, now driven by
  `gsap.ticker.add` instead of its own `requestAnimationFrame` loop;
  `lenis.on("scroll", ScrollTrigger.update)`; `gsap.ticker.lagSmoothing(0)`.
  `gsap.ticker`'s callback passes **seconds**; `lenis.raf()` expects
  **milliseconds** — the callback does `lenis.raf(time * 1000)`.
- `apps/web/components/reveal.tsx` — same exported API (`<Reveal
  stagger?>`), so every call site (`page.tsx`, `about/page.tsx`,
  `contact/page.tsx`, `food-regulations/page.tsx`, `terms/page.tsx`,
  `split-row.tsx`) needed no changes. `gsap.from()` wrapped in
  `gsap.matchMedia()` (`(prefers-reduced-motion: no-preference)`) +
  `ScrollTrigger` (`start: "top 80%", once: true`); `stagger` now animates
  direct children individually via `gsap.utils.toArray(":scope > *", ...)`
  when true.
- `apps/web/components/page-transition.tsx` — enter-only fade+slide
  (opacity + y:8, 0.3s, `ease: "outExpo"`), wrapped in the same
  `gsap.matchMedia()` pattern. A `Panel` component keyed by `pathname`
  (`<Panel key={pathname}>`) fully remounts per navigation so `useGSAP`'s
  automatic per-mount/unmount cleanup drives the animation with no manual
  revert logic. Calls `ScrollTrigger.refresh()` after each navigation
  (the new route's layout is already in the DOM by the time `useGSAP`
  runs). No true exit animation — see `requirement.md` Decision 6.
- `apps/web/components/site-header.tsx` — the one file not named in the
  original client ask but required because `motion` is being removed
  entirely. Replaced the three `motion.span` hamburger bars and the
  `motion.ul` mobile nav with plain `span`/`ul` elements plus refs
  (`bar1Ref`/`bar2Ref`/`bar3Ref`/`navListRef`). A single `useGSAP` call,
  keyed on `dependencies: [isMenuOpen, shouldReduceMotion]`, runs `.to()`
  tweens on every toggle (not a one-shot `.from()` — this animates from
  whatever the element's current live values are, correct for a toggle in
  either direction). The mobile `<ul>` tweens `height`/`opacity` using
  GSAP's native support for tweening to/from `"auto"` height. Added
  `height: 0, opacity: 0` to the `<ul>`'s initial inline `style` (alongside
  the pre-existing `overflow: "hidden"`) so the closed state renders
  correctly on first paint, before any GSAP effect runs — Framer Motion's
  `motion.ul` got this for free by using `animate` as the implicit
  mount-time initial state; a plain `<ul>` needs it set explicitly. The
  `min-[769px]:!h-auto min-[769px]:!opacity-100` Tailwind `!important`
  overrides for desktop are untouched and still take precedence over
  whatever inline style GSAP or the initial style sets, exactly as they
  did over Framer's inline styles before.
- "Why choose us" 4-card grid on the homepage: left as-is (CSS-only
  `transform`/`shadow` hover) — not touched this phase, per
  `requirement.md`'s out-of-scope list.

`apps/web/package.json`: added `gsap@^3.15.0`, `@gsap/react@^2.1.2`;
removed `motion`. Order of operations followed exactly: installed the new
packages first, added the two new `lib/` files (unused, no risk), rewrote
`motion-provider.tsx` → `reveal.tsx` → `page-transition.tsx` →
`site-header.tsx` in that order, confirmed `grep -rl "motion/react"
apps/web` returned nothing outside `node_modules`/`.next`, then ran `pnpm
remove motion`.

**Deviation from the original plan sketch**: the plan's draft code for
`reveal.tsx`/`page-transition.tsx` imported `ScrollTrigger` from
`@/lib/gsap` into files that only reference it via the `scrollTrigger:` /
`ScrollTrigger.refresh()` config keys — `reveal.tsx` doesn't call
`ScrollTrigger` directly at all (the plugin only needs to be registered,
which `lib/gsap.ts` already does at module scope), so that unused import
was dropped to keep ESLint's `no-unused-vars` clean; `page-transition.tsx`
does call `ScrollTrigger.refresh()` directly, so it keeps the import.

## Group 3 — Placeholder imagery rollout

**Deviation from the original plan**: the plan's draft used
`picsum.photos` as the placeholder image host. While implementing,
`apps/web/next.config.ts` turned out to already allowlist `placehold.co`
("Dev seed data's placeholder host", with the SVG `dangerouslyAllowSVG` +
CSP mitigation already wired in), and `apps/api/seed.sql` already uses
`https://placehold.co/600x400?text=...` for every menu item without a real
photo — an established project convention, not a new one. Reusing it:
- Needs zero `next.config.ts` changes (no new remote host, no picsum/
  fastly redirect-host gotcha to work around).
- Lets the new `PlaceholderImage` component mirror `ImageSlot`'s exact
  `label`/`className` prop signature (not a `seed`/`alt` rename), so every
  call site swap is a one-line import + tag-name change with no prop
  plumbing changes.
- Produces a self-describing placeholder (the label text is baked into the
  image itself via `?text=`), which reads more honestly as "placeholder,
  not final photography" than an unrelated stock photo would.

New file `apps/web/components/placeholder-image.tsx`:
```tsx
import Image from "next/image";

export function PlaceholderImage({ label, className = "" }: { label: string; className?: string }) {
  return (
    <div className={`relative min-h-[200px] overflow-hidden rounded-card ${className}`.trim()}>
      <Image
        src={`https://placehold.co/800x800?text=${encodeURIComponent(label)}`}
        alt={label}
        fill
        sizes="(max-width: 768px) 100vw, 50vw"
        className="object-cover"
      />
    </div>
  );
}
```

`ImageSlot` → `PlaceholderImage` swapped at every decorative/content call
site (import + tag name only, `label`/`className` unchanged):
- `apps/web/app/[locale]/page.tsx`: hero media, the 4 category tiles (also
  dropped the now-unneeded `p-2 text-[0.72rem]` classes that existed only
  to fit placeholder *text* inside the circle — irrelevant once an image
  fills it), about-preview media.
- `apps/web/app/[locale]/about/page.tsx`: hero split row, mission split row.
- `apps/web/app/[locale]/services/page.tsx`: all 4 sections (meals,
  snacks, catering, event rentals).
- `apps/web/app/[locale]/contact/page.tsx`: the map placeholder (still
  just a placeholder photo, not a real map — see `requirement.md`
  out-of-scope).

`ImageSlot` itself (`apps/web/components/image-slot.tsx`) is untouched and
still imported by `apps/web/components/menu-item-card.tsx` and
`apps/web/components/cart-line-item.tsx` as the fallback when a real
database `image_url` is absent — explicitly out of scope (Decision 4).

## Group 4 — Roadmap renumbering + live cross-references

`specs/roadmap.md`: inserted "Phase 12 — Visual & Interactive Refresh";
renumbered old Phase 12 (Admin: Orders) through old Phase 17 (Production
Hardening & Final Rollout) up to Phase 13–18. Updated in-roadmap
cross-references: Phase 2's two self-references ("that polish is Phase
17's job" → "Phase 18's job"; "see Phase 17 for verifying an actual
restore" → "Phase 18"), Phase 3's "(Phase 11/12)" → "(Phase 11/13)", Phase
5's "the enquiry form itself is Phase 14's job" → "Phase 15's job", and old
Phase 14's "once Phase-16 email is wired in" → "Phase-17 email".

Updated forward-looking references in current (non-historical) docs,
following the same precedent Phase 8 (and Phase 7 before it) set:
`specs/tech-stack.md` ("restore is verified in Phase 17" → "Phase 18"),
`docs/deployment.md` ("no admin CMS exists yet (Phase 13)" → "(Phase 14)";
"Phase 17 verifies an actual restore" → "Phase 18"), `apps/web/next.config.ts`'s
comment ("added alongside this entry in Phase 13" → "Phase 14", the
renumbered Admin: Menu & Categories), and `apps/api/src/routes/orders.rs`'s
two comments ("real validation hardening to roadmap.md Phase 16" → "Phase
17"; "Phase 13 doesn't exist yet" → "Phase 14 doesn't exist yet"). Verified
via a repo-wide grep for `Phase 1[0-9]` and `Phase-1[0-9]`/`Phase1[0-9]`
(space and hyphen forms) outside historical `specs/<date>-phase-N-*/`
folders — those historical folders are left untouched, per the same
precedent Phase 8 and Phase 7 established. `specs/mission.md` has no phase
number references.

## Verification

See `validation.md`.
