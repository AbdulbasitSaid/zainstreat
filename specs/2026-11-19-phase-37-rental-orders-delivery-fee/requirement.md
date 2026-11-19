# Phase 37 — Home Tiles, Rental Orders & Delivery Fee: Requirements

## Source

A new phase with no earlier roadmap entry. It pulls "delivery fee
calculation" forward from the roadmap's "Beyond MVP" list. Client
feedback relayed on 2026-11-19, with a phone screenshot of the home
page's "What we offer" collage (Meals, Catering, Snacks, Catering
Rentals):

> in the home page:
> - meals go to menu all
> - caterering goes to catering page
> - snacks go to menu in snacks categori
> - rentals go to rentals
>
> rentals should also have an order experience silimar to buying food.
> delivery on any type of order should have delivery fee which the admin
> can control. The admin told me develivery fee is 1 euro per km for now.
> we should have a sort of estimated delivery calculator for the users to
> see when they add thier address. use the event we added to the current
> event as the business address ( keep hidden from users ).

Answers to the planning questions (2026-11-19):

- **Catering tile:** goes to the catering quote form
  (`/contact?tab=catering`); no new catering page.
- **Distance:** road (driving) distance. Address lookup through PDOK
  (free, Dutch government, no key); driving distance through
  OpenRouteService (free key) — *superseded below: Google Routes, then
  Mapbox*.
- **Business address:** a private admin setting, filled once by the
  migration from the current event's address. The owner can change it.
  It is never shown to customers.
- **Rentals:** join the existing cart next to food. When the cart has
  rentals, checkout asks for the rental date. One order, one invoice,
  one delivery fee.

Follow-up answers to the open risks (2026-11-19):

- **Routing provider:** OpenRouteService's free Standard plan does not
  allow commercial use ([ORS plans](https://openrouteservice.org/plans/)),
  so driving distance comes from the **Google Routes API** instead
  (commercial use allowed, needs a Google Cloud billing account and a
  key; *later replaced by Mapbox, below*). PDOK stays for address lookup.
- **Origin:** seeded from the event, but delivery starts **off** until
  the owner checks the address in Settings and switches it on.
- **PDOK down:** checkout falls back to a typed address and the order
  is placed with "Delivery fee to be confirmed". The owner sets the fee
  in admin and the customer gets an updated invoice before paying.

Switch to Mapbox (2026-11-19, during implementation): the Google Cloud
billing account on techsihir@gmail.com came up closed and couldn't be
reopened, so the user chose **Mapbox Directions** instead (no Google
project needed; 100,000 requests a month free). Mapbox's Product Terms
(20 Oct 2026) forbid storing Navigation API results (§2.10.1) and
require attribution (§1.4.1); the user chose to **store the fee only**:
a Mapbox-routed order keeps fee, rate and method but not the km
(Decisions 10, 12, 13). The empty Google Cloud project `zainstreat` and a
closed billing account remain on techsihir@gmail.com, unused.

## Context

### Why now

The home page collage looks like four doors but all of them go to
`/services`. Phase 36 added rentals as "Reserve on WhatsApp" only; the
client now wants them bought like food. Every delivery order today is
charged €0 for delivery, and the owner charges €1/km.

### What exists

- **Home collage.** `apps/web/app/[locale]/page.tsx` `CATEGORY_ITEMS`
  (Meals, Snacks, Catering, Event Rentals); every tile is
  `<Link href="/services">`.
- **Menu filter.** `/menu?category={id}` (`parseCategoryId`). Category
  ids differ per environment; Snacks was added by migration in
  Phase 36 under the name "Snacks".
- **Catering.** No dedicated page: a section on `/services` and the
  enquiry form at `/contact?tab=catering` (Phase 15/23 quote workflow).
- **Rentals.** Phase 36: `rentals` table, admin CRUD at
  `/admin/rentals`, public `/{locale}/rentals` with "Reserve on
  WhatsApp" per card, `GET /api/rentals`.
- **Cart.** `apps/web/lib/cart-context.tsx` (localStorage key
  `zainstreat:cart:v1`, `CartLine` keyed `{itemId}:{optionId|flat}`,
  menu items only).
- **Orders.** `POST /api/orders` (`apps/api/src/routes/orders.rs`)
  prices lines from the database; `orders` already has `delivery_type`
  (`pickup`/`delivery`/`on_site`/`event`), free-text `delivery_address`
  and `delivery_fee NUMERIC(10,2) DEFAULT 0`. The handler hard-codes
  `let delivery_fee = Decimal::ZERO; // Decision 9 — Beyond MVP.`
  `order_items.menu_item_id` is a nullable FK; name and price are
  snapshots.
- **Checkout.** `components/order-details-form.tsx`: pickup/delivery
  radio and a free-text address (min 5 chars).
- **Quotes.** Catering quotes are priced by the owner line by line in
  the quote editor; delivery is whatever line they add.
- **Events.** `events.address` (private, Phase 31/32) — The Abula
  Experience in Almere. Event bookings are `delivery_type = 'event'`
  and have no delivery.
- **Outbound HTTP.** `reqwest` (rustls) is already a dependency
  (Phase 27). The site CSP's `connect-src` allows only `'self'` and
  `api.{DOMAIN}`, so third-party lookups must go through the API.

## Decisions

1. **Home tiles get their own targets.** Meals → `/menu`; Snacks →
   `/menu?category={snacksId}`; Catering → `/contact?tab=catering`;
   Catering Rentals → `/rentals`. The page already fetches data
   server-side; it adds `getCategories()` and finds the first category
   named "Snacks" (case-insensitive). If none exists, the Snacks tile
   falls back to `/menu`. Tiles keep their look; only `href` changes.
2. **Rentals are cart lines.** `CartLine` gains
   `kind: "menu" | "rental"` (lines saved before this phase read as
   `"menu"`). Rental keys are `rental:{rentalId}`, so ids never collide
   with menu items. Rental lines show "per event" next to the price, use
   the same quantity stepper (min 1, max 99) and photo/placeholder.
   The rentals page swaps "Reserve on WhatsApp" for the same
   add-to-cart controls and feedback (Phase 30) as menu cards; the
   page's closing WhatsApp note stays for questions.
3. **The order API takes rentals.** `items[]` entries become either
   `{ menu_item_id, price_option_id?, quantity }` or
   `{ rental_id, quantity }` (exactly one of the two ids). Rentals are
   priced from the database, must be available and not archived, and
   are snapshotted like menu items: `order_items.rental_id BIGINT NULL
   REFERENCES rentals(id) ON DELETE SET NULL`, with a check that a row
   has at most one of `menu_item_id`/`rental_id` set (both may become
   null later via `SET NULL`). Unknown/unavailable rental →
   `400 validation_error`, `items[i].rental_id`, `unavailable`.
4. **Rental date.** `orders.rental_date DATE NULL`. Required when the
   order has a rental line, rejected otherwise
   (`rental_date` `required` / `not_allowed`). It must be at least
   tomorrow (Europe/Amsterdam) and at most 365 days ahead
   (`too_soon` / `too_far`). Checkout shows a date input "Rental date"
   with the hint "The day of your event. We'll confirm pickup and
   return times with you." No stock or availability check (owner
   confirms by message, as today). The date shows on the invoice,
   tracking page, admin order page and owner email.
5. **Delivery settings are admin data.** A single-row
   `delivery_settings` table: `origin_address TEXT`, `origin_lat`,
   `origin_lon` (`DOUBLE PRECISION`), `rate_per_km NUMERIC(10,2) NOT
   NULL DEFAULT 1.00 CHECK (>= 0)`, `minimum_fee NUMERIC(10,2) NOT NULL
   DEFAULT 0 CHECK (>= 0)`, `max_distance_km NUMERIC(6,1) NULL`
   (null = no limit), `delivery_enabled BOOLEAN NOT NULL DEFAULT false`,
   `updated_at`. Saves are live at once (no draft → publish), with
   `updated_at` optimistic concurrency like the other admin forms.
6. **The origin is seeded from the current event.** The migration
   copies the address of the next upcoming non-archived published event
   (falling back to the most recently created event with an address)
   into `origin_address`, leaving coordinates null. On first use (or
   after the owner edits the address) the API geocodes it with PDOK and
   stores the coordinates. If no event has an address (dev, fresh
   databases), the origin is null and delivery quotes return
   `delivery_unavailable` until the owner sets it. **Delivery starts
   off** (user's answer, 2026-11-19): the event venue may not be the
   kitchen, so checkout offers pickup only until the owner opens
   Settings → Delivery, checks the address (and tests a few customer
   addresses), and switches delivery on. Switching it on is refused
   while the origin has no coordinates. Admin's dashboard shows a
   "Delivery is off — check the business address in Settings" notice
   until then.
7. **The origin is private.** It is only returned by the admin API.
   Public responses carry distance (km, one decimal) and fee, never the
   origin address or coordinates. Logs never print it.
8. **Fee formula.** `fee = max(minimum_fee, round_half_up(distance_km ×
   rate_per_km, 2))`, where `distance_km` is the one-way driving
   distance rounded to one decimal. With the defaults: 12.34 km →
   12.3 km → €12.30. Distance over `max_distance_km` →
   `out_of_range` (checkout: "We don't deliver this far yet. Choose
   pickup or message us on WhatsApp."). `delivery_enabled = false`
   hides the delivery option at checkout and the API rejects
   `delivery_type = delivery` (`delivery_type` `unavailable`).
9. **Address lookup.** Delivery addresses are Dutch addresses picked
   from suggestions, not free text. The API proxies PDOK Locatieserver:
   - `GET /api/delivery/suggest?q=` → up to 7 `{ id, label }`
     (`/locatieserver/v3/suggest`, `fq=type:adres`), q 3–100 chars.
   - The chosen `id` is resolved with `/locatieserver/v3/lookup` to the
     display address and coordinates.
   Both have a 4 s timeout and the existing per-IP rate limiter
   (a looser limit than orders: suggestions fire as the user types,
   debounced 300 ms client-side). Customers outside the Netherlands
   are out of scope (Out of scope). PDOK is open government data with
   no usage fee or commercial restriction, but no SLA (see Decision 17
   for when it is down).
10. **Driving distance: Mapbox Directions API** (user's answer,
    2026-11-19, replacing Google Routes; ORS's free plan is
    non-commercial). `GET https://api.mapbox.com/directions/v5/mapbox/driving/{lon},{lat};{lon},{lat}`
    with `alternatives=false&overview=false&steps=false` and the token
    in env `MAPBOX_ACCESS_TOKEN`; `routes[0].distance` is metres; 5 s
    timeout. `mapbox/driving`, not `driving-traffic`: traffic doesn't
    belong in a fixed fee. Only coordinates are sent to Mapbox, never the
    customer's name or address text. Nothing from Mapbox is cached or
    stored (Product Terms §2.10.1): the routed km is shown live at
    checkout and in admin's "Test an address", but orders don't keep it
    (Decision 12). The request URL carries the token and coordinates, so
    it is never logged. If Mapbox is down, unconfigured, over its limits
    or finds no route, the API falls back to the straight-line
    (haversine) distance × 1.3 and marks the result
    `estimated_by: "straight_line"`; the order is still accepted. Admin
    sees the method on the order page so the owner can adjust.
    **Attribution** (§1.4.1): under a routed estimate, checkout shows
    "Driving distance by © Mapbox © OpenStreetMap", linked to
    mapbox.com/about/maps and openstreetmap.org/about.
    **Cost:** 100,000 requests a month free, then $2 per 1,000; this
    shop needs a few per order.
11. **The estimate calculator.** `POST /api/delivery/quote`
    `{ address_id }` → `{ address, distance_km, fee, estimated_by }` or
    an error (`out_of_range`, `delivery_unavailable`, `not_found`).
    At checkout, choosing delivery shows an address combobox
    (accessible: `role="combobox"`, arrow keys, Enter, Escape); picking
    a suggestion shows "About {km} km · Delivery €{fee}" and the order
    summary's Delivery line and Total update. Until an address is
    picked the summary shows "Delivery: calculated from your address".
    The cart page shows "Delivery fee is calculated at checkout from
    €{rate}/km" (rate from a public `GET /api/delivery/info` →
    `{ enabled, rate_per_km, minimum_fee, max_distance_km }`).
12. **The server decides the fee.** `POST /api/orders` with delivery
    takes `delivery_address_id` (the PDOK id) instead of trusting a
    typed address: the API resolves it, recomputes distance and fee,
    and stores `delivery_address` (PDOK label + optional
    `delivery_address_extra`, e.g. "3rd floor, ring twice", ≤ 120 chars),
    `delivery_fee`, `delivery_distance_km NUMERIC(6,1)`,
    `delivery_rate_per_km NUMERIC(10,2)` and `delivery_estimated_by
    TEXT` on the order. `delivery_distance_km` is only kept for our own
    straight-line estimate; a Mapbox-routed order stores null there
    (Decision 10). Old orders keep their columns null and fee 0
    (historical accuracy rule). `total = subtotal + delivery_fee`.
    **The customer confirms any change.** The order request carries
    `expected_delivery_fee` (the fee checkout showed). If the server's
    fee differs (the owner changed the rate mid-checkout, or Mapbox
    recovered/failed between quote and order), the API places nothing
    and returns `409 { error: "delivery_fee_changed", fee, distance_km }`;
    checkout updates the summary, shows "The delivery fee changed to
    €{fee}. Please check your total and place the order again.", and
    the customer resubmits.
13. **Fee everywhere the total is.** Invoice email, owner new-order
    email, tracking page, order success page, admin order list/detail,
    and push notification text show "Delivery ({km} km): €{fee}"
    when the fee is above zero, or "Delivery: €{fee}" when the order has
    no stored km (every Mapbox-routed order). Pickup orders are unchanged.
14. **Catering quotes get a calculator, not an automatic fee.** In the
    admin quote editor, a "Calculate delivery" button (shown when the
    quote's fulfilment is delivery or on-site) looks up the enquiry's
    address with PDOK (owner picks the match), computes the fee with
    the same settings, and inserts or updates a line "Delivery
    ({km} km)" priced at the fee. The owner can edit or remove it.
    Quotes have no rental lines in this phase.
15. **Admin settings page.** `/admin/settings` (Phase 27's Settings)
    gains a "Delivery" section: business address (combobox, same PDOK
    proxy via an admin route; shows "Private — customers never see this
    address"), rate per km, minimum fee, maximum distance (empty = no
    limit), delivery on/off, and a "Test an address" field that shows
    distance and fee. Defaults: €1.00/km, €0 minimum, no maximum,
    delivery **off** (Decision 6).
16. **Copy.** All new customer copy in en/nl/fr (`CheckoutPage`,
    `CartPage`, `RentalsPage`, `OrderPage`, email bodies), written
    directly (standing rule: no approval gate). Admin copy is English
    only, like the rest of the admin.
17. **When address lookup is down: fee confirmed later** (user's
    answer, 2026-11-19). If PDOK suggest/lookup fails (timeout or 5xx),
    the address combobox turns into a plain text address field (min 5
    chars, as before this phase) with the note "We can't look up
    addresses right now. Place your order and we'll confirm the
    delivery fee before you pay." The order is sent with
    `delivery_address` text and no `delivery_address_id`; the API
    accepts it only after its own PDOK call also fails (so this can't
    be used to skip the fee while PDOK is up — then it returns
    `delivery_address_id` `required`). Such an order gets
    `orders.delivery_fee_pending BOOLEAN NOT NULL DEFAULT false` = true,
    fee 0, and:
    - **Customer:** the success page, tracking page and invoice email
      show "Delivery fee: to be confirmed" and the total as
      "€{subtotal} + delivery", and **hide the payment instructions
      and "I've paid"** until the fee is set. The payment-claim API
      returns `409 delivery_fee_pending`.
    - **Owner:** the new-order email/push say "Delivery fee needed";
      the admin order page shows a "Set delivery fee" panel with the
      typed address, a PDOK lookup (now working) that pre-fills the
      calculated fee, and a manual fee input. Saving sets the fee,
      distance (if looked up), `delivery_estimated_by` (`route`,
      `straight_line` or new value `manual`), recomputes the total,
      clears the flag and emails the customer an updated invoice
      ("Your delivery fee is confirmed") with the payment instructions.
    - The bank-transfer reference does not change.

## Out of scope

- **A dedicated catering page** (user chose the quote form).
- **Rental stock, availability calendars, deposits, return times** —
  the owner still confirms by message.
- **Rentals inside catering quotes**.
- **Delivery outside the Netherlands** — PDOK only covers Dutch
  addresses; Belgian customers choose pickup or message on WhatsApp.
- **Delivery time slots or a requested delivery time for food**.
- **Round-trip pricing or zone pricing** — one-way distance only;
  the owner can raise the rate or minimum.
- **Fees on event bookings** (`delivery_type = event`, no delivery).
- **Production rollout steps** beyond setting `MAPBOX_ACCESS_TOKEN`
  as a secret (standing rule: staging is the target).
- **A map on the checkout page** (only distance and fee are shown).

## Open risks flagged during planning

1. **Third-party availability.** *Resolved:* Mapbox down, unconfigured
   or over its limits → straight-line × 1.3 fallback, flagged in
   admin (Decision 10). PDOK down → typed address with "fee to be
   confirmed", payment held until the owner sets the fee
   (Decision 17). Pickup always works.
2. **The event address may not be a usable origin.** *Resolved:*
   delivery ships switched off; the owner checks the seeded address,
   tests a few customer addresses and switches delivery on in Settings
   (Decision 6). It can't be switched on without a geocoded origin.
3. **Old carts.** *Resolved:* carts saved before this phase have no
   `kind` and read as menu lines (Decision 2); there was never a saved
   address in the cart, so nothing else carries over.
4. **Routing terms and cost.** *Resolved:* ORS's free plan is
   non-commercial and Google's billing account couldn't be activated,
   so Mapbox Directions is used (Decision 10), well inside its 100,000
   free requests a month. *Remaining:* storing the fee, which is
   km × rate, could be read as deriving from a stored result; it is
   our own price, and the km itself is never stored. Revisit if Mapbox
   says otherwise. A Mapbox token is needed for real distances on
   staging; without it the fallback runs.
5. **Fee changes between estimate and order.** *Resolved:* the order
   carries `expected_delivery_fee`; a mismatch returns
   `409 delivery_fee_changed` and the customer confirms the new fee
   before the order is placed (Decision 12).
6. **New:** the straight-line fallback can under- or over-charge
   compared with a real route (rivers and the IJsselmeer around Almere
   make detours common). *Mitigation:* those orders are flagged
   "Straight-line estimate" in admin. The "Set delivery fee" panel
   (Decision 17) works on any delivery order still awaiting payment
   (not once the customer has claimed payment); changing the fee
   emails the customer an updated invoice.
