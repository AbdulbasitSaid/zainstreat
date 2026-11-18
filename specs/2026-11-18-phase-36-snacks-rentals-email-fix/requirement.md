# Phase 36 — Snacks, Rentals & Email Encoding Fix: Requirements

## Source

A new phase with no earlier roadmap entry. Client feedback relayed on
2026-11-18:

> - issues with email: (screenshot of the event booking email)
> - need to add snacks categorie and retental menu (admin need to manage
>   the rentals here). no need for snacks menu but a category added to
>   the menu.
> - items: `Zains_Treat_n_More_Snack_Menu.pdf`,
>   `Zains_Treat_n_More_Catering_Rentals.pdf`

The screenshot shows the "awaiting payment" booking email with
"Fried Plantain) â€” â‚¬40.00", "Total: â‚¬115.00", "youâ€™re" and
"Experience! ðŸ¤Ž".

Answers to the planning questions (2026-11-18):

- **Minimum orders:** enforced per item (admin-set minimum quantity;
  cart and API both enforce it).
- **Rental booking:** WhatsApp only, as the PDF says. No cart.
- **Getting the data live:** a data migration adds the snacks and
  rentals on staging and production.
- **Rentals link:** header nav and the Services page.

Follow-up answers to the open risks (2026-11-18):

- **Rental price unit:** per event ("€15 per event").
- **Production snacks:** the owner has added no snacks on production,
  so the migration inserts everything.
- **Photos:** launch without photos; the owner uploads them in admin.

## Context

### Why now

The client is promoting snacks and rentals alongside The Abula
Experience, and every customer email with a € sign currently looks
broken.

### What exists

- **Emails.** `apps/api/src/email.rs` `build_message` calls
  `Message::builder()…body(String)` without a `Content-Type` header.
  lettre picks a transfer encoding for the UTF-8 body but no charset, so
  mail clients fall back to US-ASCII / Windows-1252 and show UTF-8 bytes
  as "â‚¬". This hits **every** email (orders, quotes, events, owner
  notifications), not just bookings; the screenshot is simply the first
  one with lots of non-ASCII text. Subjects are fine: lettre
  RFC 2047-encodes non-ASCII subjects.
- **Menu.** `categories` and `menu_items` (single language, admin
  managed since Phase 14, draft → publish since Phase 25). An item has
  either a flat `price` or `menu_item_price_options` rows. Rows inserted
  by hand default to `published_at = now()` (live). `image_url` must be
  null or an `/api/media/` URL this API issued. Categories have
  `display_order`. There is no minimum quantity: the "Individual Plates"
  MOQ lives only in its category description.
- **Cart and orders.** `apps/web/lib/cart-context.tsx` (localStorage),
  `components/add-to-cart-controls.tsx`, `components/cart-line-item.tsx`
  with `QuantityStepper`; `POST /api/orders`
  (`apps/api/src/routes/orders.rs`) prices each line from the database
  and checks availability.
- **Rentals.** Mentioned only as copy: the Services page's "Event
  Rentals" section, the contact form's "Event Rental" subject. No data.
- **WhatsApp.** `apps/web/lib/whatsapp.ts` (`buildWhatsAppLink`,
  number `31630545277`, the same as on the PDFs); `ButtonLink` has a
  `whatsapp` colour.

## Decisions

1. **Emails are UTF-8.** `build_message` sets
   `.header(ContentType::TEXT_PLAIN)` (lettre's
   `text/plain; charset=utf-8`). Bodies stay plain text; no HTML email
   in this phase. A unit test asserts the header and that a body with
   "€ — ’ 🤎" round-trips through `message.formatted()` decoding. Since
   every email goes through `build_message`, nothing else changes.
2. **Menu items get `min_quantity`.** New column
   `menu_items.min_quantity INT NOT NULL DEFAULT 1 CHECK (min_quantity BETWEEN 1 AND 999)`.
   It is part of `MenuItemFields` (so it goes through draft → publish
   like price) with a serde default of 1, so drafts saved before this
   phase still parse. The public menu API returns it.
3. **The cart enforces it.** Add-to-cart starts at the item's minimum.
   In the cart, the − button at the minimum removes the line (as it does
   at 1 today). The card and cart line show "Minimum order: {n}" when
   n > 1. A cart line keeps the `minQuantity` it was added with; lines
   saved before this phase read as 1.
4. **The API enforces it.** `POST /api/orders` rejects a line whose
   quantity is below the item's published minimum with
   `400 validation_error`, field `items[i].quantity`, message
   `below_minimum`. The minimum is per line (item + price option). The
   checkout shows "{item}: the minimum order is {n}." and links back to
   the cart. Order history is unaffected (no snapshot needed: the
   minimum is checked, not stored).
5. **Admin edits the minimum** in the menu item form: a "Minimum order"
   number input (1–999, default 1) with the hint "Customers can't order
   fewer than this." Existing items keep 1, so nothing changes for them.
6. **Snacks is a category on the existing menu.** A data migration adds
   a **Snacks** category (description "Snacks, combo packs and small
   chops platters. Minimum orders apply.", placed after the last
   category) and these items, flat-priced unless noted, available, not
   featured, no photo (the owner adds photos in admin):

   | # | Name | Description | Price | Min |
   |---|------|-------------|-------|-----|
   | 1 | Meat Pie | Golden, flaky pastry filled with seasoned minced meat. | €4.00 | 10 |
   | 2 | Snack Pack | 1 samosa • 1 spring roll • 4 puff puff • 1 grilled chicken | €7.00 | 10 |
   | 3 | Puff Puff (small chops size) | Soft, sweet, deep-fried dough balls. | options: "50 pieces" €30.00, "100 pieces" €45.00 | 1 |
   | 4 | 50 Puff Puff & 10 Meat Skewers | 50 puff puff and 10 meat skewers. | €75.00 | 1 |
   | 5 | Meat Skewer | Spiced, grilled meat on a stick. | €5.00 | 10 |
   | 6 | Snack Combo 1 | 1 samosa • 1 spring roll • 4 puff puff • 1 grilled chicken • 1 meat pie | €12.00 | 5 |
   | 7 | Snack Combo 2 | 2 samosas • 1 spring roll • 4 puff puff • 1 meat pie | €11.50 | 5 |
   | 8 | Snack Combo 3 | 2 samosas • 2 spring rolls • 5 puff puff • 1 grilled chicken | €11.00 | 5 |
   | 9 | Snack Combo 4 | 1 meat pie • 5 puff puff • 1 peppered chicken | €10.00 | 5 |
   | 10 | Snack Combo 5 | 1 meat pie • 1 grilled chicken | €7.00 | 5 |
   | 11 | Silver Small Chops Platter | 10 samosas • 10 spring rolls • 30 puff puff • 10 grilled chicken | €70.00 | 1 |
   | 12 | Gold Small Chops Platter | 15 samosas • 15 spring rolls • 60 puff puff • 15 grilled chicken | €95.00 | 1 |
   | 13 | Diamond Small Chops Platter | 20 samosas • 20 spring rolls • 100 puff puff • 20 grilled chicken | €150.00 | 1 |

   Puff puff 50/100 become one item with two price options (same
   product, two sizes). "Gold package"/"Diamond package" are named
   "… Small Chops Platter" for consistency in the cart and on invoices.
   Item names and descriptions are English only, like the rest of the
   menu. No copy approval step (standing rule).
7. **The migration is safe to run anywhere.** It inserts the category
   only if no live (non-archived) category is named "Snacks", and each
   item only if no live item with that name exists in it. It never
   updates or deletes, so the owner's later edits survive. `seed.sql`
   (dev, TRUNCATE-based) gets the same rows so a dev reseed matches.
8. **Rentals are their own table**, not menu items, because they are
   never ordered through the cart:

   ```
   rentals(id, name, description, price NUMERIC(10,2) NOT NULL CHECK (price >= 0),
           image_url, is_available, display_order, created_at, updated_at, deleted_at)
   ```

   Single-language name and description, like menu items. No draft →
   publish workflow: admin saves are live at once (like events), with
   `updated_at` optimistic concurrency like the other admin forms.
9. **Admin manages rentals** at `/admin/rentals` (new nav item
   "Rentals" after "Categories"): a list (name, price, available
   toggle, order), "New rental", and an edit page with name (required,
   ≤ 120), description (optional, ≤ 500), price (≥ 0, two decimals),
   photo (the existing media upload, same URL rule as menu items),
   available, display order, and Archive (soft delete, with a confirm
   in-page, not a browser dialog).
10. **The public `/{locale}/rentals` page** lists available rentals in
    `display_order`: photo (or the existing placeholder treatment),
    name, description, and the price per event: "€15 per event" /
    "€15 per evenement" / "€15 par événement" (`RentalsPage.pricePerEvent`;
    the unit is the same for every rental, so it is copy, not data —
    user's answer, 2026-11-18). Each card has a "Reserve on
    WhatsApp" link pre-filled with "Hi! I'd like to rent: {name}." (per
    locale), and the page ends with the PDF's booking note: "Contact us
    on WhatsApp to reserve rental items and confirm availability." and
    a WhatsApp button. Heading "Catering Rentals", intro "Serving
    equipment and cooking essentials". Page chrome in en/nl/fr; empty
    list shows "No rentals are listed right now. Message us on
    WhatsApp." Metadata and `localeAlternates` like other pages; added
    to the sitemap.
11. **Navigation.** The header gets "Rentals" after "Menu"
    (`SiteHeader.rentals`), and the Services page's Event Rentals
    section gets a "See rentals" link to `/rentals`.
12. **Seven rentals are seeded** by the same kind of insert-if-missing
    migration, in PDF order:

    | Name | Description | Price |
    |------|-------------|-------|
    | Chafing Dish – 10 L | Keeps food warm on the buffet. | €15.00 |
    | Chafing Dish – 5 L | Keeps food warm on the buffet. | €12.00 |
    | Double Cocktail Dispenser | Two-tank drinks dispenser. | €25.00 |
    | Large Cooking Pot | Holds up to 10 kg of rice. | €30.00 |
    | Cooler | Keeps food or drinks hot or cold. | €10.00 |
    | Electric Warmer Pot | Keeps dishes warm. | €10.00 |
    | Gas Burner | Supports up to 50 kg. | €15.00 |

## Out of scope

- **Renting through the cart**, dates, deposits, stock or availability
  calendars (WhatsApp only, user's choice).
- **HTML emails** — Decision 1 only fixes the charset.
- **Translating menu or rental item names** (the menu is English-only
  today).
- **A separate snack menu page** (client: "no need for snacks menu").
- **Minimum order per category** (e.g. "Individual Plates MOQ 10"): the
  owner can now set per-item minimums if they want that enforced.
- **Production rollout steps** (standing rule: staging is the target;
  the migrations simply run on each deploy).

## Open risks flagged during planning

1. **Rental price basis.** *Resolved:* the PDF gives "€15" with no
   unit; the user confirmed prices are per event (Decision 10). A
   per-day or per-item unit would be a follow-up.
2. **Snacks already typed in by the owner.** *Resolved:* the user
   confirmed production has no snacks yet. Insert-if-missing still
   guards against re-runs and never overwrites.
3. **Old carts.** A cart saved before this phase may hold a snack below
   its minimum (unlikely: snacks are new). *Mitigation:* the API's
   `below_minimum` error explains it.
4. **No photos at launch.** New items and rentals have no image (user's
   choice: the owner uploads them in admin). *Mitigation:* menu cards
   already handle a missing image, and rental cards reuse that
   placeholder.
5. **Emails already sent** stay garbled; only new emails are fixed.
