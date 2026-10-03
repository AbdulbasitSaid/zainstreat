# Phase 7 — Menu Price Variants: Requirements

## Source

Not an original roadmap phase — inserted after Phase 6 was already
merged-in-progress, in response to the client supplying the real food menu
(`Zains_Treat_n_More_Food_Menu.pdf`). `specs/roadmap.md`, Phase 7 (as
written by this phase):

> - Data model: `display_order` on `categories`/`menu_items` so the public
>   menu's ordering can be controlled explicitly, and a
>   `menu_item_price_options` child table so one item can offer several
>   priced sizes/options (e.g. a soup in 2 L or 3 L, a bulk order as a full
>   or half size) instead of a single flat price.
> - API: `/api/menu-items` returns a nullable flat `price` plus a
>   `price_options` array (empty when the item is flat-priced); both list
>   endpoints order by `display_order`.
> - Menu page: item cards render either the existing single price or a
>   label/price list per option — display only, no selection UI yet.
> - Full replacement of the Phase 4/6 placeholder menu seed data with the
>   real client menu.

## Context

The client's PDF menu has pricing shapes Phase 6's schema cannot
represent: `menu_items.price` is a single `NUMERIC(10,2)` per item, but
several real dishes are sold at two sizes/prices (e.g. Egusi Soup: 2 L
€75 or 3 L €95), and several bulk items have a full/half-size surcharge
(e.g. Cooler of Jollof Rice: full €200, half €230, per the PDF's
"Additional Charges" section). There was also no `display_order` column
on `categories`/`menu_items` — Phase 6's own `plan.md` already flagged
this as an open risk, since both list endpoints sorted by raw `id`, with
no way to make the site's menu order match a real-world catalogue order.

This phase branches directly off the (not-yet-merged) Phase 6 branch,
since it rewrites `apps/api/src/routes/menu_items.rs`,
`apps/web/lib/api.ts`, and `apps/web/components/menu-item-card.tsx` —
all of which exist only on that branch. It is scoped as its own roadmap
phase, inserted between the already-validated Phase 6 (Menu Browsing) and
the not-yet-planned Cart phase, rather than reopening Phase 6: the change
alters the DB schema and the `/api/menu-items` wire contract, which the
Cart phase (now Phase 8) will build against from the start.

## Decisions

1. **Variants modeled as a child table, not extra columns.** A new
   `menu_item_price_options` table (`menu_item_id`, `label`, `price`,
   `display_order`, soft-deletable) holds one row per priced
   size/option. `menu_items.price` becomes nullable. An item has **either**
   a flat `price` and zero option rows, **or** a null `price` and
   one-or-more option rows — documented as an application-level invariant,
   not DB-enforced (Postgres can't `CHECK` across tables). This keeps the
   common case (a flat-priced item) exactly as simple as it was in Phase 6,
   and avoids forcing every item through an options table.

2. **`display_order` added to `categories` and `menu_items`** in the same
   migration, since a schema change was already required — resolves Phase
   6's own flagged risk. Both list endpoints change `ORDER BY id` to
   `ORDER BY display_order, id`.

3. **This phase is display-only for variants** — `MenuItemCard` renders a
   label/price list in place of the single price line when options exist,
   with no selection interactivity. Choosing a variant and adding it to a
   cart is the Cart phase's job once it exists; no cart exists yet to wire
   this into.

4. **Half-size bulk pricing.** The PDF lists "½ Cooler of Rice +€30" and
   "½ Box of Chicken or Turkey +€20" under a general "Additional Charges"
   section, not tied to a specific item. Resolved (with the user) as
   explicit Full/Half price-option rows per affected item: Cooler of
   Jollof Rice (Full €200 / Half €230), Cooler of Fried Rice (Full €300 /
   Half €330), Box of Peppered Turkey (Full €200 / Half €220), Box of
   Peppered Chicken (Full €150 / Half €170) — i.e. the surcharge applied
   on top of each item's own full price, not a single shared discount.

5. **Full seed-data replacement**, not an addition. All Phase 4/6
   placeholder categories (Rice Dishes, Snacks, Drinks, Soups) and items
   (Jollof Rice, Fried Rice, Chicken Suya, Puff Puff, Meat Pie, Chin Chin,
   Small Chops, Efo Soup, Egusi Soup) are removed. `seed.sql` now opens
   with `TRUNCATE ... CASCADE` so it is idempotent (re-running it replaces
   data instead of appending), a deliberate behavior change from Phase 6's
   run-once-only script. Two of the four real photos from Phase 6
   (`egusi-soup.jpeg`, `efo-soup.jpeg`) are reused for the matching new
   items (Egusi Soup, Efo Riro); `chin-chin.jpeg` and `small-chops.jpeg`
   become unreferenced (left in place, no matching dish in the real menu).
   Every other item uses a `placehold.co` placeholder, same mechanism as
   Phase 6.

6. **API response shape.** `MenuItemResponse` gains `price_options:
   Vec<PriceOptionResponse>` (always present, `[]` for flat-priced items)
   and `price` becomes `Option<Decimal>`, still serialized as a JSON
   string (`rust_decimal::serde::str_option`) for the same
   float-rounding-avoidance reason Phase 6 chose `rust_decimal::serde::str`
   for the flat case. Fetching strategy: one query for `menu_items`, one
   for all non-deleted `menu_item_price_options` (table stays tiny, ~20
   rows), grouped by `menu_item_id` in Rust — no SQL-side aggregation,
   matching this codebase's existing un-aggregated query style.

## Out of scope

- Cart / variant **selection** UI — Phase 8 (Cart).
- Enforcing the flat-price-XOR-options invariant in code (DB constraint or
  admin-form validation) — revisit when Phase 12 (Admin: Menu &
  Categories) builds the menu-editing form.
- The Individual Plates category's "5-plate orders: add €2 per plate"
  MOQ-tier surcharge — this is a quantity-based rule, not a selectable
  size/variant, and has no natural home in `menu_item_price_options`. Kept
  as plain text in the category's `description` for now; revisit once the
  Cart phase needs to model order-quantity-dependent pricing.
- Deleting the now-unreferenced `chin-chin.jpeg`/`small-chops.jpeg` image
  files.

## Open risks flagged during planning

1. The flat-price-XOR-options invariant is documented only, not enforced.
   Nothing stops a future hand-written `INSERT`/`UPDATE` (or, later, an
   admin form) from setting both a `price` and option rows, or neither —
   the API will serialize whatever exists without validating it. No DB
   trigger was added to keep this phase scoped; the admin menu-editing
   phase should validate this client- and server-side.
2. `seed.sql`'s `TRUNCATE ... CASCADE` also cascades to `order_items`
   (which references `menu_items`), wiping any dev orders on every reseed.
   Acceptable since this script is explicitly dev-only and no production
   data path ever runs it, but worth remembering if dev orders are ever
   relied on across a reseed.
3. This phase branches off the Phase 6 branch rather than `master`, since
   Phase 6 is not yet merged. Whoever merges these branches needs to land
   Phase 6 into `master` first, then this phase on top of it — not two
   independent PRs against `master` in parallel.
