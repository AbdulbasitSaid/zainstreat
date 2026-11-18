# Phase 36 — Snacks, Rentals & Email Encoding Fix: Plan

Two migrations, no new packages. Decisions refer to `requirement.md`.

## Group 0 — UTF-8 emails (Decision 1)

Ship this first: it is independent and fixes every customer email.

`apps/api/src/email.rs`:

```rust
use lettre::message::header::ContentType;
// build_message:
Message::builder()
    .from(from)
    .reply_to(reply_to)
    .to(…)
    .subject(…)
    .header(ContentType::TEXT_PLAIN)
    .body(email.body.clone())
```

Tests in the same module:

- `email_body_is_declared_utf8`: `header(&message, "Content-Type")`
  equals `text/plain; charset=utf-8`.
- `email_body_keeps_non_ascii`: body `"€40.00 — you’re 🤎"`; the
  formatted message (`message.formatted()`) has the charset header and a
  `Content-Transfer-Encoding` of `quoted-printable` or `base64`, and
  decoding that part gives back the exact string (decode with lettre's
  own encoding or assert the quoted-printable bytes `=E2=82=AC` for €).

Check: `grep -rn "Message::builder" apps/api/src` shows only
`build_message`.

## Group 1 — Migration: `min_quantity`, rentals table

`apps/api/migrations/20261118160000_min_quantity_rentals.up.sql`:

```sql
-- Phase 36: per-item minimum order (Decision 2) and rentals (Decision 8).
ALTER TABLE menu_items
    ADD COLUMN min_quantity INT NOT NULL DEFAULT 1
        CHECK (min_quantity BETWEEN 1 AND 999);

CREATE TABLE rentals (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    price NUMERIC(10, 2) NOT NULL CHECK (price >= 0),
    image_url TEXT,
    is_available BOOLEAN NOT NULL DEFAULT true,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
```

`.down.sql`: `DROP TABLE rentals; ALTER TABLE menu_items DROP COLUMN min_quantity;`

## Group 2 — Migration: snacks and rentals data (Decisions 6, 7, 12)

`apps/api/migrations/20261118160100_snacks_rentals_data.up.sql`,
insert-if-missing only:

```sql
-- Phase 36: client's snack menu and rentals list (Decisions 6, 7, 12).
-- Never updates or deletes: the owner's later edits win.
INSERT INTO categories (name, description, display_order)
SELECT 'Snacks', 'Snacks, combo packs and small chops platters. Minimum orders apply.',
       COALESCE(MAX(display_order), 0) + 1
FROM categories WHERE deleted_at IS NULL
HAVING NOT EXISTS (SELECT 1 FROM categories WHERE name = 'Snacks' AND deleted_at IS NULL);

WITH snacks AS (
    SELECT id FROM categories WHERE name = 'Snacks' AND deleted_at IS NULL
    ORDER BY id LIMIT 1
)
INSERT INTO menu_items (category_id, name, description, price, min_quantity, display_order)
SELECT s.id, v.name, v.description, v.price, v.min_quantity, v.display_order
FROM snacks s
CROSS JOIN (VALUES
    ('Meat Pie', 'Golden, flaky pastry filled with seasoned minced meat.', 4.00, 10, 1),
    ('Snack Pack', '1 samosa • 1 spring roll • 4 puff puff • 1 grilled chicken', 7.00, 10, 2),
    ('Puff Puff (small chops size)', 'Soft, sweet, deep-fried dough balls.', NULL, 1, 3),
    -- … rows 4–13 exactly as the requirement.md Decision 6 table
) AS v(name, description, price, min_quantity, display_order)
WHERE NOT EXISTS (
    SELECT 1 FROM menu_items m
    WHERE m.category_id = s.id AND m.name = v.name AND m.deleted_at IS NULL
);
```

(Cast `v.price` to `NUMERIC(10,2)` in the VALUES so the NULL row
types.) Then the two puff puff options, only if the item has none:

```sql
INSERT INTO menu_item_price_options (menu_item_id, label, price, display_order)
SELECT m.id, o.label, o.price, o.display_order
FROM menu_items m
JOIN categories c ON c.id = m.category_id AND c.name = 'Snacks'
CROSS JOIN (VALUES ('50 pieces', 30.00, 1), ('100 pieces', 45.00, 2)) AS o(label, price, display_order)
WHERE m.name = 'Puff Puff (small chops size)' AND m.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM menu_item_price_options p
                  WHERE p.menu_item_id = m.id AND p.deleted_at IS NULL);
```

Rentals: the seven rows of Decision 12 with `display_order` 1–7, each
`WHERE NOT EXISTS (SELECT 1 FROM rentals WHERE name = v.name AND deleted_at IS NULL)`.
Use a real en dash in "Chafing Dish – 10 L" (UTF-8 file).

*Implementation note:* the inserts only run on a database that already
has a live category (staging, production). `#[sqlx::test]` databases
run every migration and must start empty, and a fresh dev database gets
the same rows from `seed.sql`. `seed.sql` does not TRUNCATE `rentals`
(the production menu reseed must not wipe the owner's rentals); it
inserts them if missing.

`.down.sql`: delete exactly these rentals by name, the Snacks items by
name (and their options via cascade), and the Snacks category if it has
no other live items. Orders keep their snapshots (`order_items.menu_item_id`
is `ON DELETE SET NULL`).

`apps/api/seed.sql`: add the Snacks category (display_order 5), the 13
items (with `min_quantity`), the puff puff options, and the seven
rentals; add `rentals` to the TRUNCATE list. Keep the
flat-price/options invariant comment true.

Test (`apps/api/tests/` or a `#[sqlx::test]` with migrations): after
migrating, Snacks has 13 items, puff puff has 2 options and a NULL
price, 7 rentals exist; running the data migration's SQL a second time
inserts nothing.

## Group 3 — Minimum quantity in the API (Decisions 2, 4, 5)

`apps/api/src/routes/admin/menu_items.rs`:

- `MenuItemFields` gains `#[serde(default = "one")] pub min_quantity: i32`.
- `validate_menu_item`: `field_error("min_quantity", "invalid")` unless
  `1..=999`.
- Create, draft save, publish (in `routes/admin/drafts.rs`, wherever the
  draft is applied to the row) and `AdminMenuItemResponse` carry it.

`apps/api/src/routes/menu_items.rs`: `MenuItemResponse.min_quantity`.
Check `routes/admin/drafts.rs`' preview path returns it too.

`apps/api/src/routes/orders.rs`:

- `MenuItemRow` gains `min_quantity: i32`; select it.
- After pricing a line: if `item.quantity < menu_item.min_quantity`,
  collect `field_error(&format!("items[{i}].quantity"), "below_minimum")`
  and return `AppError::Validation` before inserting (roll back the tx).
  Include the minimum for the UI: if `FieldError` has no room for it,
  the web looks it up from the menu (see Group 4); don't change the
  error shape.

Tests: an order of 9 meat pies → 400 `items[0].quantity`
`below_minimum`; 10 → 201; an item with min 1 behaves as before; a draft
saved without `min_quantity` (old JSON) parses as 1; admin rejects 0
and 1000.

Regenerate `apps/api/.sqlx/` (`cargo sqlx prepare`).

## Group 4 — Minimum quantity in the web (Decisions 3, 4, 5)

- `apps/web/lib/menu.ts` (or wherever `MenuItem` is typed):
  `min_quantity: number`.
- `apps/web/lib/cart-context.tsx`: `CartLine.minQuantity: number`;
  the loader's validator defaults a missing value to 1; `add` uses
  `Math.max(quantity, minQuantity)` for a new line; `setQuantity` below
  `minQuantity` removes the line.
- `components/add-to-cart-controls.tsx`: pass `min_quantity`, start the
  stepper (if any) and the add at the minimum.
- `components/menu-item-card.tsx` and `components/cart-line-item.tsx`:
  when `minQuantity > 1`, a small muted line
  `t("minimumOrder", { count })`.
- `components/order-view.tsx` / `lib/orders.ts`: on a
  `items[i].quantity` `below_minimum` field error, show
  `errors.belowMinimum` ("{item}: the minimum order is {count}.") using
  the cart line's name and `minQuantity`, with a link to the cart.
- Admin: `apps/web/lib/admin-menu.ts` (type) and
  `components/admin-menu-item-form.tsx`: "Minimum order" number input
  (`min={1} max={999}`, default 1), hint "Customers can't order fewer
  than this.", inline `min_quantity` error.
- Messages en/nl/fr: `Menu.minimumOrder` ("Minimum order: {count}" /
  "Minimale bestelling: {count}" / "Commande minimum : {count}"),
  `Cart.minimumOrder` (same), `Order.errors.belowMinimum`
  ("{item}: the minimum order is {count}." / "{item}: de minimale
  bestelling is {count}." / "{item} : la commande minimum est de
  {count}.").

## Group 5 — Rentals API (Decisions 8, 9)

`apps/api/src/routes/rentals.rs` (public):

- `GET /api/rentals` → `[{id, name, description, price (string), image_url}]`,
  `WHERE deleted_at IS NULL AND is_available ORDER BY display_order, id`.

`apps/api/src/routes/admin/rentals.rs`:

- `RentalFields { name, description, price: Decimal, image_url, is_available, display_order }`
  with `validate_rental` (name trimmed 1–120, description ≤ 500,
  price ≥ 0 and ≤ 2 decimals, image via `is_valid_media_url`,
  display_order 0–9999).
- `GET /admin/rentals` (all non-archived, any availability),
  `POST /admin/rentals` (201), `GET /admin/rentals/{id}`,
  `PUT /admin/rentals/{id}` (body includes `updated_at`; stale → the
  existing conflict error used by categories/menu), `POST
  /admin/rentals/{id}/archive` (sets `deleted_at`; deletes the media
  object like menu item archive does, if that's what it does).
- Register in `routes/mod.rs` and `routes/admin/mod.rs`.
- If the privacy export/anonymise or retention code enumerates tables,
  nothing to add: rentals hold no personal data.

Tests: CRUD round-trip, validation errors, archived and unavailable
rentals hidden from the public list, stale `updated_at` → conflict.

## Group 6 — Admin rentals UI (Decision 9)

Mirror the categories admin:

- `apps/web/lib/admin-rentals.ts`: types and fetchers.
- `apps/web/app/api/admin/rentals/route.ts` and
  `app/api/admin/rentals/[id]/route.ts` (+ `archive`): proxies like
  `app/api/admin/categories`.
- `apps/web/app/admin/(protected)/rentals/page.tsx` (list),
  `rentals/new/page.tsx`, `rentals/[id]/page.tsx`.
- `components/admin-rental-form.tsx`: fields per Decision 9, photo via
  the existing media upload component the menu item form uses, archive
  with an inline confirm step.
- `components/admin-nav.tsx`: `{ label: "Rentals", href: "/rentals", enabled: true }`
  after Categories.

## Group 7 — Public rentals page and nav (Decisions 10, 11)

- `apps/web/lib/api.ts`: `getRentals()` (same fetch/revalidate pattern
  as the menu).
- `apps/web/app/[locale]/rentals/page.tsx`: `PageHero` heading and
  intro, a responsive grid of rental cards (reuse the menu card's image
  / placeholder styling), price as
  `t("pricePerEvent", { price })` with the existing euro formatter, a
  `ButtonLink color="whatsapp"` per card to
  `buildWhatsAppLink(t("whatsappMessage", { name }))`, the booking note
  box and a page-level WhatsApp button; empty state. `generateMetadata`
  with `localeAlternates(locale, "/rentals")`.
- `apps/web/app/sitemap.ts`: add `/rentals`.
- `components/site-header.tsx`: `{ href: "/rentals", key: "rentals" }`
  after `menu`; check the desktop nav still fits at 769px and the
  sliding indicator works.
- `app/[locale]/services/page.tsx`: in the Event Rentals row, a
  `ButtonLink href="/rentals"` "See rentals".
- Messages en/nl/fr: `SiteHeader.rentals` ("Rentals" / "Verhuur" /
  "Location"), `ServicesPage.seeRentals` ("See rentals" / "Bekijk
  verhuur" / "Voir la location"), new `RentalsPage` namespace:
  - `metaTitle`: "Catering Rentals" / "Cateringverhuur" / "Location de
    matériel traiteur"
  - `metaDescription`: "Chafing dishes, cooking pots, coolers and more
    to rent for your event." / "Chafing dishes, kookpannen, koelboxen
    en meer te huur voor je evenement." / "Chauffe-plats, marmites,
    glacières et plus à louer pour votre événement."
  - `heading`: same as `metaTitle`
  - `intro`: "Serving equipment and cooking essentials" /
    "Serveermateriaal en kookbenodigdheden" / "Matériel de service et
    essentiels de cuisine"
  - `pricePerEvent`: "{price} per event" / "{price} per evenement" /
    "{price} par événement"
  - `reserve`: "Reserve on WhatsApp" / "Reserveer via WhatsApp" /
    "Réserver sur WhatsApp"
  - `whatsappMessage`: "Hi! I'd like to rent: {name}." / "Hoi! Ik wil
    graag huren: {name}." / "Bonjour ! Je voudrais louer : {name}."
  - `bookingHeading`: "Booking" / "Reserveren" / "Réservation"
  - `bookingNote`: "Contact us on WhatsApp to reserve rental items and
    confirm availability." / "Neem contact op via WhatsApp om
    huurartikelen te reserveren en de beschikbaarheid te bevestigen." /
    "Contactez-nous sur WhatsApp pour réserver le matériel et confirmer
    sa disponibilité."
  - `empty`: "No rentals are listed right now. Message us on
    WhatsApp." / "Er staat nu geen verhuur online. Stuur ons een
    WhatsApp-bericht." / "Aucun matériel n'est proposé pour le moment.
    Écrivez-nous sur WhatsApp."
- Run `pnpm check:messages`.

## Group 8 — Docs and roadmap

- `specs/roadmap.md`: Phase 36 entry (done in planning).
- After deploying to staging, check the Snacks category and rentals
  appear, and send a test booking/order email to confirm the encoding.
