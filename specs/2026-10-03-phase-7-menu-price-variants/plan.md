# Phase 7 — Menu Price Variants: Implementation Plan

Dependency-ordered groups. Backend groups (1–6) touch `apps/api`; frontend
groups (7–8) touch `apps/web`; Group 9 touches `docs/`; Group 10 touches
`specs/roadmap.md` and current-code comments referencing phase numbers.

## Group 0 — Branch

Branched `2026-10-03-phase-7-menu-price-variants` off
`2026-10-03-phase-6-menu-browsing` (not `master` — Phase 6 is not yet
merged, and this phase rewrites files that only exist on that branch).

## Group 1 — Migration

`apps/api/migrations/20261003143036_add_menu_price_variants.{up,down}.sql`
(generated via `sqlx migrate add -r add_menu_price_variants`, additive
only — `20261002162443_initial_schema.{up,down}.sql` untouched):

- `ALTER TABLE categories ADD COLUMN display_order INT NOT NULL DEFAULT 0`
- `ALTER TABLE menu_items ADD COLUMN display_order INT NOT NULL DEFAULT 0,
  ALTER COLUMN price DROP NOT NULL`
- New `menu_item_price_options` table (`id`, `menu_item_id` FK `ON DELETE
  CASCADE`, `label`, `price`, `display_order`, timestamps, `deleted_at`)
  plus an index on `menu_item_id`.
- `down.sql` drops the new table and both `display_order` columns, leaves
  `price` nullable (reverting NOT NULL would fail once any row has
  `price IS NULL`; this migrator only ever runs by hand in local dev).

Applied locally and confirmed with `sqlx migrate run`.

## Group 2 — `apps/api/src/routes/menu_items.rs`

Full rewrite. `price` is now `Option<Decimal>`
(`rust_decimal::serde::str_option` — confirmed to exist in the pinned
`rust_decimal` 1.43.0 under the already-enabled `serde-with-str` feature,
no `Cargo.toml` change needed). Added `PriceOptionResponse { id, label,
price }`. `MenuItemResponse` gained `price_options: Vec<PriceOptionResponse>`
(always present, `[]` for flat-priced items).

Two queries: `menu_items` (`ORDER BY display_order, id`, same
`category_id` filter and "include unavailable" behavior as Phase 6 —
no `is_available` filter added), and all non-deleted
`menu_item_price_options` (`ORDER BY menu_item_id, display_order, id`),
grouped into a `HashMap<i64, Vec<PriceOptionResponse>>` and attached per
item.

## Group 3 — `apps/api/src/routes/categories.rs`

One-line change: `ORDER BY id` → `ORDER BY display_order, id`.
`CategoryResponse` and `routes/mod.rs` unchanged.

## Group 4 — sqlx offline cache

`cargo sqlx prepare -- --tests` (plain `cargo sqlx prepare` only covers
the lib target by default — `-- --tests` is required to also cache the
integration tests' queries). Regenerated `apps/api/.sqlx/`; stale entries
from the old `ORDER BY id`/pre-split queries were automatically pruned by
the tool (no manual cleanup needed). Verified `SQLX_OFFLINE=true cargo
check --all-targets` passes on the committed cache alone.

## Group 5 — `apps/api/tests/menu_browsing.rs`

Kept the 4 existing tests (unaffected by the nullable `price` column).
Added: `categories_ordered_by_display_order_then_id`,
`menu_items_flat_price_has_empty_price_options`,
`menu_items_with_price_options_have_null_price_and_ordered_options`,
`menu_items_excludes_soft_deleted_price_options`. `cargo test` → 8/8 pass.

## Group 6 — `apps/api/seed.sql`

Full replacement. Opens with `TRUNCATE TABLE menu_item_price_options,
menu_items, categories RESTART IDENTITY CASCADE` (idempotent — a behavior
change from Phase 6's append-only script). Seeds:

- 4 categories (Individual Plates, By The Litre, Soups & Stews, Bulk
  Orders) with `display_order` 1–4.
- 18 flat-price `menu_items` rows + 10 variant `menu_items` rows (`price`
  omitted/NULL) with `display_order` matching the PDF's per-category item
  order.
- 20 `menu_item_price_options` rows for the 10 variant items, joined by
  item name.
- `egusi-soup.jpeg`/`efo-soup.jpeg` reused for Egusi Soup/Efo Riro; every
  other item uses a `placehold.co` placeholder URL-encoded from its name.
- Featured (`is_featured = true`): Jollof Rice & Plantain with Chicken or
  Turkey, Egusi Soup, Cooler of Jollof Rice. All 28 items `is_available =
  true` (this is the real catalogue, not a fixture demoing an
  out-of-stock state).

Loaded against the dev DB and verified via `curl` (see validation.md).

## Group 7 — `apps/web/lib/api.ts` and `apps/web/lib/format.ts`

Added `MenuItemPriceOption { id, label, price }`; `MenuItem.price`
changed to `string | null`; added `MenuItem.price_options:
MenuItemPriceOption[]`. Added `formatPriceOrNull(price: string | null):
string | null` to `format.ts`; `formatPrice`/`parsePrice` unchanged.

## Group 8 — `apps/web/components/menu-item-card.tsx`

When `item.price` is non-null, renders exactly as Phase 6 (single price
line). When null, renders a `<ul>` of `price_options` as "label — price"
rows, keyed by `option.id`, no interactivity. No changes needed to
`menu/page.tsx`, the homepage, or `category-filter.tsx` — all three
already delegate rendering to `MenuItemCard` and consume whatever
order/shape the API returns.

## Group 9 — Docs

`docs/local-development.md`: noted `seed.sql`'s truncate-and-replace
behavior (including the `order_items` cascade) and the
flat-price-XOR-options invariant.

## Group 10 — Roadmap renumbering + live-doc phase references

`specs/roadmap.md`: inserted "Phase 7 — Menu Price Variants"; renumbered
old Phase 7 (Cart) through Phase 15 (Production Hardening & Final
Rollout) to Phase 8–16; updated in-roadmap cross-references (`Phase
9/10` → `Phase 10/11`, `Phase 12's job` → `Phase 13's job` ×2 different
occurrences, `Phase 14` → `Phase 15`/`Phase 16` as appropriate, `Phase-14
email` → `Phase-15 email`). Historical `specs/<date>-phase-N-*/` folders
(1–6) left untouched, per the precedent Phase 3's own renumbering set.

Also updated forward-looking references in current (non-historical) docs,
the same category as the roadmap itself: `apps/web/next.config.ts`'s
comment (Phase 11 → Phase 12, the new Admin: Menu & Categories number),
`specs/tech-stack.md` and `docs/deployment.md`'s backup-restore-verification
references (both said "Phase 14"; the restore-verification bullet was
always in the old "Production Hardening & Final Rollout" phase, so this
also fixes a pre-existing off-by-one — now correctly "Phase 16").

## Verification

See `validation.md`.
