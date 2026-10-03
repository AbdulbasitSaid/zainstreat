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

## Definition of done

The floating decorative shapes are gone from the codebase entirely (file
deleted, CSS keyframes removed, no remaining references). GSAP + `@gsap/react`
+ ScrollTrigger fully replace Framer Motion as the site's animation system —
`motion` is no longer a dependency — while Lenis smooth scroll is preserved
and now correctly integrated with ScrollTrigger via `gsap.ticker`. Every
page that previously showed a text-label `ImageSlot` placeholder for
decorative/content imagery (homepage hero and category tiles, homepage
about-preview, the about page's hero and mission sections, all four
services sections, and the contact page's map area) now renders a real
(placeholder) image via the new `PlaceholderImage` component, reusing the
project's existing `placehold.co` convention. `ImageSlot` itself remains in
place for its one legitimate remaining use — the real-`image_url` fallback
on menu items and cart line items. The roadmap is renumbered and internally
consistent, with this phase's own `specs/` folder following the established
format.
