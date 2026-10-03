# Phase 7 — Menu Price Variants: Validation

## Pass/fail checklist

- [x] `cd apps/api && cargo test` — 8/8 pass (4 existing Phase 6 tests +
      4 new ones for `display_order` and price-option behavior).
- [x] `SQLX_OFFLINE=true cargo check --all-targets` succeeds against the
      committed `.sqlx/` cache alone (no `DATABASE_URL` set).
- [x] `apps/web`: `npx eslint` and `npx tsc --noEmit` clean on all changed
      files (`lib/api.ts`, `lib/format.ts`, `components/menu-item-card.tsx`).
- [x] Migration applies cleanly (`sqlx migrate run`) against the dev
      database.
- [x] `apps/api/seed.sql` loads cleanly: `TRUNCATE` → 4 categories, 18
      flat-price items, 10 variant items, 20 price options inserted.
      Re-running the same script a second time keeps the counts at 4/28
      (proves replace-not-append).
- [x] `GET /api/categories` returns 4 categories in PDF order (Individual
      Plates, By The Litre, Soups & Stews, Bulk Orders).
- [x] `GET /api/menu-items` returns 28 items. Spot-checked: a flat item
      (`"price": "13.00"`, `"price_options": []`) and a variant item
      (`"price": null`, options in ascending `display_order`, e.g. Egusi
      Soup → `[{"label":"2 L","price":"75.00"},{"label":"3 L","price":"95.00"}]`).
- [x] Within each category, item order matches the PDF's item order
      exactly (verified for all 4 categories by grouping the API response
      by `category_id`).
- [x] `/en/menu` server-rendered HTML: category headings render in the
      correct order; Egusi Soup's card renders a 2-line "2 L — €75,00" /
      "3 L — €95,00" list instead of a single price; flat items
      (e.g. Jollof Rice & Plantain) still render one price line.
- [x] `/nl/menu` loads and uses the Dutch `MenuPage` translations
      (`"Ons Menu"`, `"Momenteel niet beschikbaar"`) — unchanged from
      Phase 6, confirming the locale/i18n layer wasn't disturbed.
- [x] Homepage (`/en`) "Featured Menu" section shows exactly the 3 chosen
      items (Jollof Rice & Plantain with Chicken or Turkey, Egusi Soup,
      Cooler of Jollof Rice) in that order.
- [x] `specs/roadmap.md` phase headings are sequential 1–16 with no gaps
      or duplicates (`grep -n "^## Phase" specs/roadmap.md`); all
      in-roadmap cross-references to shifted phase numbers updated.
- [x] `/validate` skill run (full lint + build + live HTTP smoke sweep):
      `apps/web` — `pnpm lint` and `pnpm build` both clean; `apps/api` —
      `cargo check`, `cargo clippy --all-targets`, and `cargo build` all
      clean (zero warnings). Live stack: `GET /health` → `200
      {"status":"ok"}`; `GET /` → `307` to `/en`; `GET /en` and `GET /nl`
      → `200` with correct `<html lang>`; `GET /api/categories` → 4
      categories; `GET /api/menu-items` → 28 items (18 flat with empty
      `price_options`, 10 with 2+ ordered options). Stack torn down with
      `docker compose down` after.
- [ ] CI green on PR (same open item Phase 6 itself left unchecked, since
      Phase 6 isn't merged yet either — both phases' PRs need to land
      together, Phase 6 first).

## Definition of done

The real Zain's Treat n More menu (28 items across 4 categories) replaces
all placeholder fixture data, with explicit display ordering matching the
client's PDF and full support for per-item price variants (sizes/options),
surfaced end-to-end from migration through API through the menu page and
homepage. No cart/selection interactivity is added — Phase 8 (Cart) picks
up from here with the `price`/`price_options` contract already in place.
Phase 6 and this phase both need to merge into `master` (Phase 6 first)
before Phase 8 can branch off `master` directly.
