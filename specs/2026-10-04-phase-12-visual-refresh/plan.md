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

## Addendum — real imagery, idle float, custom cursor, nav indicator

Groups 0–4 above are already committed (`3432423`). Groups 5–8 below are
new, planned follow-up scope — see `requirement.md`'s Addendum for the
decisions behind each.

## Group 5 — Real imagery pipeline

New directory `apps/web/assets/images/`, 8 generated files (dimensions are
targets for the generator, ~4:3 for `SplitRow` media panels, 1:1 for the
circular category tiles):

| File | ~Size | Content brief |
|---|---|---|
| `hero.jpg` | 1600×1200 | Warm, appetizing overhead spread of home-style/international dishes on a table, natural light, no text/logos. |
| `category-meals.jpg` | 1200×1200 | A single generous plate of a hearty home-cooked meal, close-up, natural light. |
| `category-snacks.jpg` | 1200×1200 | An assortment of fried/baked snacks on a plate, close-up. |
| `category-catering.jpg` | 1200×1200 | A catering buffet spread set up at an event, wide shot. |
| `category-event-rentals.jpg` | 1200×1200 | An elegantly set event/banquet table with rented chairs and linens, wide shot. |
| `about-hero.jpg` | 1600×1200 | A warm, candid photo representing a small food business's kitchen/team at work. |
| `about-mission.jpg` | 1600×1200 | Fresh ingredients being prepared, close-up, natural light. |
| `contact.jpg` | 1600×1200 | The business's storefront, or a delivery/pickup moment — warm, welcoming. |

(Implementer: adjust briefs to match real brand tone/feedback; regenerate
rather than ship an off-brand first pass — see `requirement.md` Open risk 1.)

**Deviation from the original plan**: no image-generation capability was
available in the implementing session (see `requirement.md` addendum Open
risk 1). Asked directly, the user chose to source free, real stock
photography instead (over solid-color placeholder stand-ins, or pausing
this group entirely) — each of the 8 briefs above was matched against a
real Pexels photo via its direct CDN URL (`images.pexels.com/photos/
<id>/pexels-photo-<id>.jpeg`), downloaded with `curl`, and visually
reviewed one by one before committing. One early contact-page candidate
was rejected and replaced after review because it visibly showed a real
competing food brand's storefront signage — using another company's
branded building as this site's own contact photo would misrepresent it;
the replacement is an unbranded delivery-handoff photo instead. This
satisfies Decision 7's "real imagery, statically imported, `next/image`
blur placeholder" goals but not its literal "AI-generated" wording — see
`validation.md`'s addendum checklist for the explicit note.

New `apps/web/components/site-image.tsx` (replaces
`apps/web/components/placeholder-image.tsx`, which is deleted once no call
site imports it):

```tsx
"use client";

import { useRef } from "react";
import Image, { type StaticImageData } from "next/image";
import { gsap, useGSAP } from "@/lib/gsap";

export function SiteImage({
  src,
  alt,
  className = "",
  priority = false,
  float = false,
}: {
  src: StaticImageData;
  alt: string;
  className?: string;
  priority?: boolean;
  float?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (!float) return;
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.to(ref.current, { y: 10, duration: 3, ease: "sine.inOut", repeat: -1, yoyo: true });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [float] },
  );

  return (
    <div ref={ref} className={`relative min-h-[200px] overflow-hidden rounded-card ${className}`.trim()}>
      <Image
        src={src}
        alt={alt}
        fill
        sizes="(max-width: 768px) 100vw, 50vw"
        className="object-cover"
        placeholder="blur"
        priority={priority}
      />
    </div>
  );
}
```

Call-site updates (import + tag name + props, `label` string → `alt` string
+ imported `src`):
- `apps/web/app/[locale]/page.tsx`: hero (`src={hero}`, `priority`,
  `float`), the 4 category tiles (`src={categoryMeals}` etc., `float`),
  about-preview (`src={aboutHero}`, no `float`).
- `apps/web/app/[locale]/about/page.tsx`: hero (`src={aboutHero}`),
  mission (`src={aboutMission}`).
- `apps/web/app/[locale]/services/page.tsx`: the 4 sections reuse the
  category images (`src={categoryMeals}` etc.) — same file imported again
  from `@/assets/images/...`, no cross-file coupling.
- `apps/web/app/[locale]/contact/page.tsx`: `src={contact}`, no `float`.

## Group 6 — Custom cursor

New `apps/web/components/custom-cursor.tsx`: a `useSyncExternalStore` +
`matchMedia("(pointer: fine)")` check (mirrors `site-header.tsx`'s
`isDesktop` pattern) combined with the existing `useReducedMotion()` hook;
if either gate fails, renders `null` and never touches `document.body`'s
class list. When both pass: adds `custom-cursor-active` to `document.body`
in a `useEffect`, renders a small fixed dot + a larger trailing ring (both
`pointer-events-none`), driven by four `gsap.quickTo` setters (dot:
`duration 0.1`; ring: `duration 0.35`, so it visibly lags) updated on a
single `window` `mousemove` listener. A delegated `document` `mouseover`/
`mouseout` pair checks `event.target.closest("a, button, [role='button'],
input, textarea, select, label")` and scales the ring up/down
(`gsap.to(ring, { scale: 1.6 / 1, duration: 0.2 })`). Cleanup on unmount
removes all listeners and the body class.

`apps/web/app/[locale]/layout.tsx`: mount `<CustomCursor />` next to
`<MotionProvider />` (public-site layout only).

`apps/web/app/globals.css`: inside the existing `@layer base` block, next
to the `prefers-reduced-motion` safety net, add:
```css
body.custom-cursor-active,
body.custom-cursor-active * {
  cursor: none;
}
```

## Group 7 — Animated desktop nav indicator

`apps/web/components/site-header.tsx`: add an `indicatorRef`
(`useRef<HTMLSpanElement>`) and an item-ref map keyed by `href`, plus a
`moveIndicatorTo(href)` helper that reads the target `<li>`'s and the nav
`<ul>`'s `getBoundingClientRect()` and `gsap.to`s the indicator's `x`/
`width` (`duration: 0.3`, `ease: "outExpo"`). Call it on mount and on
`pathname` change (active route), on each desktop nav `<li>`'s
`onMouseEnter`, and on the `<ul>`'s `onMouseLeave` (back to the active
route). The indicator `<span>` renders only when `isDesktop` is true (the
existing boolean already in this component) — mobile's stacked menu is
untouched.

## Group 8 — tech-stack.md correction + imagery convention

`specs/tech-stack.md`'s Frontend "Animation" bullet and its "_Why Motion +
Lenis over GSAP:_" rationale paragraph still describe the pre-migration
stack even though Group 2 (committed in `3432423`) already fully replaced
`motion` with GSAP. Rewrite both to describe what's actually running:
GSAP + ScrollTrigger + CustomEase + `@gsap/react` (`useGSAP`), Lenis kept
and driven off `gsap.ticker`. Update the Summary Table's "Animation" row
to match. Add a short new bullet (or extend the existing one) documenting
the `apps/web/assets/images/` static-import convention for site-decoration
imagery, noting it supersedes `placehold.co` for that purpose while the
DB-driven `ImageSlot` fallback still uses `placehold.co`.

## Group 9 — Menu item imagery (`apps/api/seed.sql`, Addendum 2)

16 new files under `apps/web/public/images/menu/`, each `.jpg`, generated
(or, per Addendum 2's Open risk 1 fallback, sourced as freely-licensed
stock photography) at a 3:2 aspect ratio to match the existing
`width={600} height={400}` `object-cover` usage in `MenuItemCard`/
`CartLineItem` (`apps/web/components/menu-item-card.tsx`,
`apps/web/components/cart-line-item.tsx` — unchanged, they already render
whatever string `image_url` holds). Style brief for all 16: natural light,
a neutral dark-wood or rustic-ceramic surface, 45°-or-overhead angle,
consistent across the set — matching whatever visual direction the
Addendum 1 site-decoration photos (`apps/web/assets/images/*.jpg`)
established, since both now appear on the same pages.

Reuse mapping — file name, the `seed.sql` row(s) it covers (by current
`name` value), and a one-line content brief:

| File | `seed.sql` row(s) | Brief |
|---|---|---|
| `jollof-rice-plantain.jpg` | `Jollof Rice & Plantain with Chicken or Turkey`; `5 L Jollof Rice + 5 Pieces of Chicken or Turkey`; `Cooler of Jollof Rice` | Smoky party jollof rice, fried plantain, chicken/turkey piece |
| `fried-rice-plantain.jpg` | `Fried Rice & Plantain with Chicken or Turkey`; `5 L Fried Rice + 5 Pieces of Chicken or Turkey`; `Cooler of Fried Rice` | Vegetable fried rice, fried plantain, chicken/turkey piece |
| `ewa-agoyin.jpg` | `Ewa Agoyin with Plantain & Fish`; `2 L Ewa Agoyin + 500 ml Agoyin Sauce` | Mashed beans, dark peppered agoyin sauce, plantain, fish |
| `vegetable-salad.jpg` | `Vegetable Salad`; `½ Tray of Vegetable Salad` | Fresh colourful mixed vegetable salad |
| `white-rice-beans-assorted-stew.jpg` | `White Rice & Beans with Plantain & Assorted Meat Stew`; `5 L White Rice + 1.5 L Assorted Meat Stew` | White rice and beans, plantain, assorted-meat stew |
| `white-rice-beans-pepper-beef.jpg` | `White Rice & Beans with Pepper Sauce & Beef` | White rice and beans, peppery sauce, beef |
| `assorted-meat-stew.jpg` | `Assorted Meat Stew` | Tomato-based stew, assorted meats, no rice in frame |
| `moin-moin-fish-egg.jpg` | `Moin Moin with Fish & Egg` | Steamed bean pudding wedge, visible fish and egg |
| `moin-moin-egg.jpg` | `Moin Moin with Egg` | Steamed bean pudding wedge, visible egg, no fish |
| `smoked-mackerel-sauce.jpg` | `Smoked Mackerel Fish Sauce` | Tomato sauce, visible smoked mackerel pieces |
| `pepper-sauce.jpg` | `Pepper Sauce` | Fiery red tomato-pepper sauce, no protein |
| `fried-fish.jpg` | `50 Pieces of Fried Fish` | Platter of golden crisp fried fish pieces |
| `ogbono-soup.jpg` | `Ogbono Soup` | Dark-green, thick ogbono draw soup |
| `ofada-stew.jpg` | `Ofada Stew` | Bold dark-red/brown ofada-style stew |
| `poultry-stew.jpg` | `Chicken Stew`; `Turkey Stew` | Classic tomato stew, poultry pieces (reused across both) |
| `peppered-poultry.jpg` | `Box of Peppered Turkey`; `Box of Peppered Chicken` | Pan-fried peppered poultry pieces (reused across both) |

`Egusi Soup`/`Efo Riro` already have real photos and are untouched; the
other 10 pre-existing `seed.sql` rows not listed above don't exist (28
total rows: 2 already real + 26 covered by the 16 files above).

Implementation mechanics:
- Generate/source the 16 files, save to
  `apps/web/public/images/menu/<name-from-table-above>`.
- In `apps/api/seed.sql`, for each VALUES tuple whose `name` appears in the
  table above, replace its `'https://placehold.co/600x400?text=...'`
  `image_url` string with `'/images/menu/<file>.jpg'` (exact string match
  per the file column above — e.g. both the `Jollof Rice & Plantain with
  Chicken or Turkey` row in the flat-price VALUES block and the `5 L Jollof
  Rice + 5 Pieces of Chicken or Turkey` row in that same block get
  `'/images/menu/jollof-rice-plantain.jpg'`; `Cooler of Jollof Rice` is in
  the separate variant-priced VALUES block further down the file). No
  other column in any row changes — names, descriptions, prices, display
  orders, and the `menu_item_price_options` INSERT are all untouched.
- No `apps/api` Rust code change (Decision 17) — `image_url` is already
  passed through as-is.

## Verification

See `validation.md`.
