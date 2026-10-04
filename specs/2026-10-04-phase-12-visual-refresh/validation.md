# Phase 12 — Visual & Interactive Refresh: Validation

## Pass/fail checklist

- [x] `cd apps/web && pnpm lint` — clean, zero errors/warnings (also ran
      automatically per-file via the project's `lint-changed-file.sh` hook
      throughout implementation; every error it surfaced was fixed
      immediately before moving on).
- [x] `cd apps/web && pnpm build` (Turbopack, production build) — compiles
      cleanly, TypeScript passes, all 25 pages generate successfully.
- [x] `cd apps/api && SQLX_OFFLINE=true cargo check --all-targets` — clean
      (only comment changes this phase, in `routes/orders.rs`).
- [x] `cd apps/api && SQLX_OFFLINE=true cargo clippy --all-targets` —
      clean, zero warnings.
- [x] `grep -rl "motion/react" apps/web` (excluding `node_modules`/`.next`)
      → empty.
- [x] `grep -rn "DecorativeShape" apps/web` → empty.
- [x] `grep -rn "float-rotate" apps/web/app/globals.css` → empty.
- [x] `cd apps/web && pnpm list motion` → not found (fully removed).
- [x] Live smoke test against the project's existing dev Docker Compose
      stack (`zainstreat-web-1`, `zainstreat-api-1`, already running under
      bind mounts — restarted just the `web` service so its
      `web_node_modules` named volume would pick up the new `gsap`/
      `@gsap/react` dependencies and drop `motion`; this is the Phase 1 dev
      convention, see `docs/local-development.md`):
      - `GET /` → `307` to `/en`.
      - `GET /en`, `GET /nl` → both `200`, correct `<html lang="en">` /
        `<html lang="nl">`.
      - `GET /en/about`, `/en/services`, `/en/contact`, `/en/menu` → all
        `200`.
      - `GET /health` (API, port 8080) → `200`.
      - Confirmed `placehold.co` placeholder image URLs (proxied through
        `next/image`'s optimizer) are present in the rendered `/en` HTML,
        e.g. the hero image's `text=` query carries the actual hero
        headline copy — confirms `PlaceholderImage` is wired up and
        rendering, not silently falling back to nothing.
      - First attempt returned a build-time `Module not found: Can't
        resolve 'gsap/ScrollTrigger'` 500 from inside the container — not
        a code bug: the container's `node_modules` is a separate named
        Docker volume from the host's, populated once at container start
        (`pnpm install && pnpm dev`), so it hadn't seen the new
        dependencies added via the host-side `pnpm add`. Restarting the
        `web` service re-ran `pnpm install` inside the container and
        resolved it; all routes passed on the retest.
- [ ] **Manual — not checked here (needs a browser):**
      - No shapes visible anywhere on the homepage or about page.
      - Scroll-triggered `Reveal` sections fade up once per section, not
        repeatedly on scroll up/down.
      - Route navigation fades/slides in smoothly (enter-only, per
        `requirement.md` Decision 6 — there is intentionally no fade-out of
        the outgoing page).
      - Mobile hamburger menu opens/closes smoothly in both directions,
        bars rotate/fade correctly, and the desktop breakpoint still shows
        the nav list statically regardless of `isMenuOpen` state.
      - Toggling the OS "reduce motion" setting disables Lenis smooth
        scroll, `Reveal`'s fade-up, `PageTransition`'s fade/slide, and
        slows the header's toggle tweens to instant, consistent with the
        pre-existing `prefers-reduced-motion` CSS override in
        `globals.css`.
      - Visual check that placeholder images render as recognizable,
        appropriately-sized boxes (hero, circular category tiles, the four
        services sections, the about page, and the contact "map") rather
        than broken-image icons or badly cropped text.
      - The brief GSAP-has-no-SSR-bake-in flash noted as an accepted
        trade-off in `requirement.md` risk 1 — worth a quick look on a
        throttled connection to confirm it's as minor as expected.
- [x] `grep -n "^## Phase" specs/roadmap.md` — phases run 1–18
      sequentially, no gaps or duplicates.
- [x] Repo-wide grep for `Phase 1[0-9]` and the hyphenated `Phase-1[0-9]`/
      `Phase1[0-9]` forms, outside historical `specs/<date>-phase-N-*/`
      folders — every live cross-reference updated (`specs/roadmap.md`,
      `specs/tech-stack.md`, `docs/deployment.md`,
      `apps/web/next.config.ts`, `apps/api/src/routes/orders.rs`); no
      remaining stale references.
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Addendum — Pass/fail checklist (real imagery, float, cursor, nav indicator)

- [x] `cd apps/web && pnpm lint` — clean after Groups 5–8.
- [x] `cd apps/web && pnpm exec tsc --noEmit -p tsconfig.json` — clean.
- [x] `cd apps/web && pnpm build` (Turbopack, production build) —
      compiles cleanly, all 25 pages generate successfully.
- [x] `grep -rn "placeholder-image" apps/web` (excluding `.next`) → empty
      (component fully replaced and deleted).
- [x] `grep -rln "placehold.co" apps/web/app apps/web/components` → only
      `image-slot.tsx`/its call sites (`menu-item-card.tsx`,
      `cart-line-item.tsx`) remain; no static page still references
      `placehold.co`.
- [x] `ls apps/web/assets/images/` → exactly the 8 files named in
      `plan.md` Group 5. **Deviation from the original plan**: these are
      real, freely-licensed stock photographs (sourced from Pexels direct
      CDN URLs, verified individually for on-brand content and absence of
      third-party branding — one initial contact-page candidate showing a
      competitor's storefront signage was rejected and replaced) rather
      than AI-generated images as Decision 7 specifies — no image
      generation capability was available in the implementing session; the
      user explicitly chose this sourcing method when asked. Functionally
      equivalent (real photos, statically imported, `next/image`
      blur-placeholder support) but worth a conscious note for whoever
      reviews this before merge, since it doesn't literally match "AI-
      generated."
- [x] `grep -rn "SiteImage" apps/web/app` → all 10 original call sites
      present (hero, 4 category tiles, about-preview, about hero, about
      mission, 4 services sections, contact — 10 call sites total using 8
      distinct `src` imports per `plan.md` Group 5's reuse table).
      Confirmed server-rendered HTML also carries the base64 blur
      placeholder and the hashed `/_next/static/media/...jpg` URLs for all
      8 images via a live HTTP fetch of `/en`, `/en/about`, `/en/services`,
      `/en/contact`.
- [ ] **Manual — not checked here (no browser available in this session):**
      - Every image renders as a real photo, correctly cropped (circular
        category tiles, rectangular `SplitRow` panels), no broken-image
        icons, no visible blur-placeholder flash lingering past the real
        image's load.
      - Hero image and the 4 category tiles visibly, subtly float
        (slow vertical drift) when `prefers-reduced-motion` is not set;
        the about/services/contact images do not float.
      - Toggling OS "reduce motion" stops the float entirely (images sit
        still) and leaves the native cursor visible (custom cursor never
        activates).
      - On a touchscreen device/emulation (`pointer: coarse`), the native
        cursor is never hidden and no custom cursor dot/ring appears.
      - On desktop with a mouse: the native cursor is hidden, a small dot
        tracks the pointer exactly, a trailing ring lags slightly behind,
        and the ring visibly scales up when hovering any link/button/
        form control across the homepage, about, services, menu, cart,
        and contact pages (not just one page).
      - Fast scrolling (via Lenis) while also moving the mouse quickly
        doesn't produce visible jank or a frozen cursor (`requirement.md`
        addendum Open risk 2).
      - Desktop nav: hovering each nav link smoothly slides/resizes the
        indicator under it; moving the mouse off the nav snaps the
        indicator back to the current route's link; the indicator is
        absent entirely on the mobile stacked menu.
- [ ] CI green on PR.

## Addendum 2 — Pass/fail checklist (menu item imagery)

- [x] `ls apps/web/public/images/menu/` → the existing 4 files
      (`egusi-soup.jpeg`, `efo-soup.jpeg`, `chin-chin.jpeg`,
      `small-chops.jpeg`, the last two still unreferenced — see
      `requirement.md` Addendum 2 Context) plus 17 new `.jpg` files — 21
      total. **Deviation from `plan.md` Group 9's table**: that table
      specified 16 files but missed one row — `2 L Fried Plantain` (By
      The Litre) wasn't assigned a file during planning. Caught during
      implementation and gap-filled with a 17th file,
      `fried-plantain.jpg`, rather than leaving that one row on
      `placehold.co`.
- [x] `grep -c "placehold.co" apps/api/seed.sql` → `0`.
- [x] `grep -n "image_url" apps/api/seed.sql` spot-check: every row's
      `image_url` either stays untouched (`Egusi Soup`, `Efo Riro`) or
      matches the (corrected, 17-file) mapping, including the two reused
      pairs (`Chicken Stew`/`Turkey Stew` → `poultry-stew.jpg`; `Box of
      Peppered Turkey`/`Box of Peppered Chicken` → `peppered-poultry.jpg`)
      and the gap-filled `2 L Fried Plantain` → `fried-plantain.jpg`.
- [x] `cd apps/api && SQLX_OFFLINE=true cargo check --all-targets` — clean.
- [x] Re-ran the seed against the project's actual running dev stack
      (`docker compose exec -T postgres psql -U zainstreat -d zainstreat <
      apps/api/seed.sql`, per `docs/local-development.md`) — `TRUNCATE`
      cascaded cleanly, all 4 `INSERT` statements succeeded (4 categories,
      18 flat-price items, 10 variant items, 20 price options). Confirms
      the new `image_url` strings aren't malformed SQL.
- [x] Live smoke test against that same running stack: `GET /en/menu` →
      `200`, `grep -c placehold.co` on the response body → `0`, and every
      one of the 18 distinct `/images/menu/*` files referenced in the
      rendered HTML (both as the raw `src` and through
      `/_next/image?url=...`). Direct-fetched one file
      (`/images/menu/ewa-agoyin.jpg`) and its `/_next/image`-optimized
      form — both `200`.
- [x] **Sourcing deviation (same caveat as Addendum 1):** no
      image-generation tool was available in the implementing session, so
      all 17 images were sourced as real, freely-licensed stock
      photography (16 from Pexels, 1 — `ogbono-soup.jpg` — from Unsplash;
      both no-attribution-required licenses), per Addendum 2's
      Open risk 1 fallback.
- [x] **Quality review pass (manual, performed in-session via direct
      image inspection, not deferred)**: all 17 sourced images were
      individually viewed and checked against their `plan.md` brief. 6 of
      the first-pass picks were rejected as misleading or off-brand and
      re-sourced before being committed:
      - `ewa-agoyin.jpg` — rejected a photo of whole beans and carrots
        (not mashed, no sauce); replaced with a thick, glossy, dark
        bean stew that actually reads as mashed/pureed.
      - `moin-moin-fish-egg.jpg` — rejected an East-Asian whole-grilled-
        fish-and-pickled-vegetables plate (completely wrong dish);
        replaced by reusing `moin-moin-egg.jpg`'s photo (steamed
        orange-red pudding in a ramekin topped with egg) — a deliberate,
        documented reuse across both moin moin rows rather than a
        distinct fish-specific photo, since no suitable stock photo of a
        fish-studded steamed bean pudding could be found.
      - `ogbono-soup.jpg` — rejected a bright lime-green, fine-dining-
        styled pea soup (wrong color/tone and clashing presentation);
        replaced with a dark, thick, leafy soup served with a pounded-yam
        swallow (labeled "eforiro" at the source, but a materially better
        visual stand-in for a thick, dark Nigerian soup than the
        original pick).
      - `smoked-mackerel-sauce.jpg` — rejected a Mediterranean shrimp-
        and-dill stew; replaced with a tight crop (via local
        `magick`/ImageMagick, not a different source photo) of a dark
        fish-in-tomato-sauce dish from a West African flat-lay,
        removing the surrounding clutter.
      - `peppered-poultry.jpg` — rejected a Turkish-styled mezze plate
        (sumac dust border, pickled-onion salad); replaced with plain
        grilled chicken pieces on a simple plate.
      - `fried-rice-plantain.jpg` — rejected Indonesian nasi goreng
        (fried egg, prawn crackers — strong wrong-cuisine signals);
        replaced with a plainer fried-rice-and-salad plate with no
        cuisine-specific markers.
      All 6 replacements were re-verified visually before committing. The
      remaining 11 first-pass picks (`jollof-rice-plantain`,
      `fried-plantain`, `vegetable-salad`,
      `white-rice-beans-assorted-stew`, `white-rice-beans-pepper-beef`,
      `assorted-meat-stew`, `moin-moin-egg`, `pepper-sauce`, `fried-fish`,
      `ofada-stew`, `poultry-stew`) were judged acceptable as-is, some
      with minor noted imperfections (e.g. `poultry-stew.jpg`'s
      background has fine-dining pepper mills; `fried-fish.jpg`'s
      background shows an out-of-frame Asian-style side bowl) that don't
      rise to the same "actively misleading" bar as the 6 replaced.
- [x] All 17 new files resized/normalized to 1200×800 (matching the
      aspect ratio `MenuItemCard`/`CartLineItem` render at) via local
      `magick`, keeping file sizes consistent with the pre-existing 4
      real photos (85–373 KB each, no outliers).
- [ ] **Manual — still needs an actual browser, not just an HTML
      fetch:**
      - Crop behavior at the cart line item's 64×64 thumbnail size
        specifically (only the menu card's 600×400 size was checked via
        the rendered page fetch above).
      - No visual seams between the Addendum 1 site-decoration photos and
        these menu photos when seen side by side in the real layout.
      - Unavailable items (`is_available = false`, most of the catalogue)
        still show their new photo correctly grayscaled/dimmed via
        `MenuItemCard`'s `grayscale-[30%]` class, not broken.
- [ ] CI green on PR.

## Definition of done

The floating decorative shapes are gone from the codebase entirely (file
deleted, CSS keyframes removed, no remaining references). GSAP + `@gsap/react`
+ ScrollTrigger fully replace Framer Motion as the site's animation system —
`motion` is no longer a dependency — while Lenis smooth scroll is preserved
and now correctly integrated with ScrollTrigger via `gsap.ticker`. Every
page that previously showed a text-label `ImageSlot` placeholder for
decorative/content imagery (homepage hero and category tiles, homepage
about-preview, the about page's hero and mission sections, all four
services sections, and the contact page's map area) now renders a real photo
(sourced as free stock photography rather than AI-generated — see the
Addendum checklist's noted deviation from Decision 7) via the new
`SiteImage` component, statically imported
from `apps/web/assets/images/`. `ImageSlot` itself remains in place for its one legitimate remaining use —
the real-`image_url` fallback on menu items and cart line items, now
rendering a real photo for every one of the 28 `seed.sql` menu items (16
new reused images plus the pre-existing `Egusi Soup`/`Efo Riro` photos
covering all 26 previously-`placehold.co` rows; see Addendum 2), with
`placehold.co` itself no longer referenced anywhere in `seed.sql` — only
`ImageSlot`'s own component code still mentions it, for the case where a
future menu item genuinely has no `image_url` set. The homepage hero and
category tiles idly float; a sitewide custom cursor reacts to every
interactive element on the public site, gated on `pointer: fine` and
`prefers-reduced-motion`; the desktop nav shows an animated hover/active
indicator. The roadmap is renumbered and internally consistent,
`tech-stack.md`'s Animation section accurately describes GSAP (not Motion),
with this phase's own `specs/` folder following the established format.
