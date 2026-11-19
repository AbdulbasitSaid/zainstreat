# Phase 37 — Home Tiles, Rental Orders & Delivery Fee: Plan

One migration, no new packages (`reqwest` is already in the API).
Decisions refer to `requirement.md`.

## Group 0 — Home tiles (Decision 1)

Independent; ship first.

`apps/web/app/[locale]/page.tsx`:

- Add `href` to each `CATEGORY_ITEMS` entry: Meals `"/menu"`, Catering
  `"/contact?tab=catering"`, Event Rentals `"/rentals"`; Snacks is
  computed.
- Fetch `getCategories()` alongside the existing calls and compute:

  ```ts
  const snacks = categories.find((c) => c.name.trim().toLowerCase() === "snacks");
  const snacksHref = snacks ? `/menu?category=${snacks.id}` : "/menu";
  ```

  (`Link` from `@/i18n/navigation` accepts the query string; if the
  typed `href` rejects it, use `{ pathname: "/menu", query: { category } }`.)
- Replace `href="/services"` on the tile `Link` with the entry's href.
  The "Explore services" button below stays on `/services`.

## Group 1 — Migration (Decisions 3, 4, 5, 6, 12)

`apps/api/migrations/20261119120000_rental_orders_delivery.up.sql`:

```sql
-- Phase 37: rentals in orders (Decisions 3, 4) and delivery fee (Decisions 5, 6, 12).
ALTER TABLE order_items
    ADD COLUMN rental_id BIGINT REFERENCES rentals(id) ON DELETE SET NULL,
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'menu' CHECK (kind IN ('menu', 'rental')),
    ADD CONSTRAINT order_items_one_source
        CHECK (menu_item_id IS NULL OR rental_id IS NULL);

ALTER TABLE orders
    ADD COLUMN rental_date DATE,
    ADD COLUMN delivery_distance_km NUMERIC(6, 1),
    ADD COLUMN delivery_rate_per_km NUMERIC(10, 2),
    ADD COLUMN delivery_estimated_by TEXT
        CHECK (delivery_estimated_by IN ('route', 'straight_line', 'manual')),
    -- Decision 17: placed while PDOK was down; payment held until the owner sets the fee.
    ADD COLUMN delivery_fee_pending BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE delivery_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    origin_address TEXT,
    origin_lat DOUBLE PRECISION,
    origin_lon DOUBLE PRECISION,
    rate_per_km NUMERIC(10, 2) NOT NULL DEFAULT 1.00 CHECK (rate_per_km >= 0),
    minimum_fee NUMERIC(10, 2) NOT NULL DEFAULT 0 CHECK (minimum_fee >= 0),
    max_distance_km NUMERIC(6, 1) CHECK (max_distance_km > 0),
    -- Decision 6: off until the owner checks the seeded address.
    delivery_enabled BOOLEAN NOT NULL DEFAULT false,
    CHECK (NOT delivery_enabled OR (origin_lat IS NOT NULL AND origin_lon IS NOT NULL)),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Decision 6: the business address starts as the current event's address.
INSERT INTO delivery_settings (origin_address)
SELECT (
    SELECT address FROM events
    WHERE deleted_at IS NULL AND btrim(address) <> ''
    ORDER BY (published_at IS NOT NULL AND starts_at >= now()) DESC,
             starts_at ASC, created_at DESC
    LIMIT 1
);
```

Check the real column names in `20261117120000_events.up.sql` (archive
column, publish flag, start time) before writing the `ORDER BY`; the
intent is "next upcoming published event, else newest with an
address". The row is always inserted (origin may be null).

`.down.sql`: drop `delivery_settings`, the five `orders` columns, the
constraint and `order_items.rental_id`/`kind`. (A down after rental orders
exist loses their `rental_id` link but keeps the name/price snapshot.)

*Implementation note:* `seed.sql` is **not** changed. It also runs on
staging (`staging-up.yml`) and production (`seed-production.yml`), so a
dev origin there would overwrite the real one. On a fresh database
(dev, a new staging instance where the API migrates before
`seed-events.sql` runs) the origin is empty and is set in admin
Settings → Delivery, which is also how the validator turns delivery on.

## Group 2 — API: delivery module (Decisions 7–11, 17)

New `apps/api/src/delivery.rs`:

```rust
pub struct DeliveryClient {
    http: reqwest::Client,                 // built once, 5 s timeout
    google_routes_api_key: Option<String>, // env GOOGLE_ROUTES_API_KEY
}
// No route cache: Google Maps Platform terms restrict caching (Decision 10).

pub struct ResolvedAddress { pub id: String, pub label: String, pub lat: f64, pub lon: f64 }
pub enum EstimatedBy { Route, StraightLine, Manual } // Manual: owner-set (Decision 17)
pub struct DeliveryQuote {
    pub address: ResolvedAddress,
    pub distance_km: Decimal,  // one decimal
    pub fee: Decimal,          // two decimals
    pub rate_per_km: Decimal,
    pub estimated_by: EstimatedBy,
}
// LookupDown = PDOK failed (Decision 17); Unavailable = delivery off / no origin.
pub enum DeliveryError { Unavailable, OutOfRange, NotFound, LookupDown }

impl DeliveryClient {
    pub async fn suggest(&self, q: &str) -> Result<Vec<Suggestion>, DeliveryError>;
    pub async fn lookup(&self, id: &str) -> Result<ResolvedAddress, DeliveryError>;
    pub async fn quote(&self, pool: &PgPool, address_id: &str) -> Result<DeliveryQuote, DeliveryError>;
}

pub fn fee(distance_km: Decimal, s: &DeliverySettings) -> Decimal; // Decision 8, pure
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64;
```

- PDOK: `https://api.pdok.nl/bzk/locatieserver/search/v3_1/suggest?q=…&fq=type:adres&rows=7`
  and `…/lookup?id=…&fl=id,weergavenaam,centroide_ll`. Parse
  `centroide_ll` (`POINT(lon lat)`). Confirm the current base path
  against PDOK's docs during implementation (v3_1 at planning time).
- Google Routes: `POST https://routes.googleapis.com/directions/v2:computeRoutes`,
  headers `X-Goog-Api-Key: {key}`, `X-Goog-FieldMask: routes.distanceMeters`,
  body:

  ```json
  {
    "origin": { "location": { "latLng": { "latitude": 52.37, "longitude": 5.21 } } },
    "destination": { "location": { "latLng": { "latitude": 52.35, "longitude": 4.90 } } },
    "travelMode": "DRIVE",
    "routingPreference": "TRAFFIC_UNAWARE",
    "units": "METRIC"
  }
  ```

  Read `routes[0].distanceMeters`. Only coordinates are sent.
- `quote()` reads `delivery_settings`; if `origin_lat` is null and
  `origin_address` is set, geocode it with PDOK `free?q=` (first
  `type:adres` hit) and store the coordinates; if still none →
  `Unavailable`. `delivery_enabled = false` → `Unavailable`.
- Google failure, missing key, `429`/quota or empty `routes` →
  `haversine × 1.3`, `StraightLine` (log at `warn` with the HTTP status
  only, never addresses or coordinates — Decision 7).
- PDOK timeout / 5xx / connection error → `LookupDown` (distinct from
  a PDOK `404`/empty result, which is `NotFound`).
- Distance rounding: `Decimal::from_f64(km).round_dp_with_strategy(1, MidpointAwayFromZero)`;
  fee rounding the same at 2 dp.
- Register `DeliveryClient` in app state (`Arc`), constructed in
  `main.rs` from env; add `GOOGLE_ROUTES_API_KEY: ${GOOGLE_ROUTES_API_KEY:-}` to the `api`
  service in `docker-compose.yml` and the staging deploy env/secrets in
  `.github/workflows` next to `VAPID_PRIVATE_KEY`.

Unit tests (no network): `fee` with defaults (12.34 km → €12.30),
minimum fee, zero rate, `haversine_km` against a known pair
(Amsterdam Centraal → Almere Centrum ≈ 24 km), `centroide_ll` parsing,
Google response parsing (`distanceMeters`, empty `routes`), out-of-range.

*Implementation note (found in `/validate`):* the request trace span
logged the full URI, so `GET /api/delivery/suggest?q=…` put the address a
customer typed into the API logs. `apps/api/src/lib.rs` now logs the path
without the query string for every request.

## Group 3 — API: public delivery routes (Decisions 9, 11)

`apps/api/src/routes/delivery.rs`, wired in `routes/mod.rs`:

| Route | Body / query | Response |
|---|---|---|
| `GET /api/delivery/info` | — | `{ enabled, rate_per_km, minimum_fee, max_distance_km }` |
| `GET /api/delivery/suggest?q=` | q 3–100 chars | `[{ id, label }]` |
| `POST /api/delivery/quote` | `{ address_id }` | `{ address, distance_km, fee, estimated_by }` |

- Errors: `400 validation_error` for bad `q`/`address_id`;
  `422 { error: "out_of_range" | "not_found" }`;
  `503 { error: "delivery_unavailable" }` (off / no origin);
  `503 { error: "address_lookup_down" }` (PDOK failed — checkout
  switches to the typed-address fallback, Decision 17).
- Rate limit: add `delivery_rate_limiter()` in `rate_limit.rs` (e.g.
  burst 30, refill 1 per second per IP) for `suggest` and `quote`.
- None of these responses contain origin fields (test asserts the JSON
  keys).

## Group 4 — API: orders with rentals and fees (Decisions 3, 4, 12, 13)

`apps/api/src/routes/orders.rs`:

- `CreateOrderItemRequest`: `menu_item_id: Option<i64>`,
  `rental_id: Option<i64>`, `price_option_id`, `quantity`. Validation:
  exactly one id (`items[i]` `invalid`); rental lines reject
  `price_option_id`; rental quantity 1–99.
- Load rentals `WHERE id = ANY($1) AND deleted_at IS NULL AND
  is_available` and price them; menu items unchanged (incl. Phase 36
  minimums).
- `CreateOrderRequest`: add `rental_date: Option<NaiveDate>`,
  `delivery_address_id: Option<String>`,
  `delivery_address_extra: Option<String>`; `delivery_address` is no
  longer read for delivery (kept optional for compatibility, ignored).
  Rental date rules per Decision 4, "today" via `chrono_tz::Europe::Amsterdam`
  if already a dependency, else `chrono::Local` with `TZ` set in the
  container — check `Cargo.toml` and match how events compute cutoffs.
- Delivery with `delivery_address_id`: call `DeliveryClient::quote`;
  map `OutOfRange` → field error `delivery_address_id` `out_of_range`,
  `Unavailable` → `delivery_type` `unavailable`, `NotFound` →
  `delivery_address_id` `not_found`. Store label (+ `, {extra}` when
  given), fee, distance, rate and method. Replace
  `Decimal::ZERO // Decision 9`.
- **Fee check (Decision 12):** `expected_delivery_fee: Option<Decimal>`
  is required with `delivery_address_id`; if it differs from the
  computed fee, return before any insert:
  `409 { "error": "delivery_fee_changed", "fee": "12.30", "distance_km": "12.3" }`.
- **Fallback (Decision 17):** delivery without `delivery_address_id`
  but with a typed `delivery_address` (≥ 5 chars): the API calls PDOK
  `free?q={typed}`; if PDOK answers at all → `400 delivery_address_id
  required` (lookup is up, use it); only on `LookupDown` does it accept
  the order with `delivery_fee = 0`, `delivery_fee_pending = true`,
  distance/rate/method null. `delivery_enabled = false` still rejects.
- `claim_payment`: `409 { error: "delivery_fee_pending" }` while the
  flag is set.
- `OrderResponse` / `load_order_by_token` / admin order queries: add
  `rental_date`, `delivery_distance_km`, `delivery_estimated_by`,
  `delivery_fee_pending`;
  order items add `kind: "menu" | "rental"`, stored in a new
  `order_items.kind TEXT NOT NULL DEFAULT 'menu' CHECK (kind IN ('menu', 'rental'))`
  column (add it in Group 1's migration) so the kind survives the
  rental being deleted and `rental_id` going null.
- Emails (`order_emails.rs`, `delivery_line`): add
  "Delivery ({km} km): €{fee}" and "Rental date: {date}" lines per
  locale; owner email and push text include the fee when > 0.
  Pending-fee orders: invoice says "Delivery fee: to be confirmed",
  total "€{subtotal} + delivery", no payment block; owner email/push
  say "Delivery fee needed". New email kind `delivery_fee_confirmed`
  (updated invoice with payment instructions), tracked in
  `email_deliveries` like the others.
- Regenerate `apps/api/.sqlx/`.

Tests (`#[sqlx::test]`, delivery client with a stub/mock trait so no
network): rental-only order, mixed order, rental without date → 400,
date today → `too_soon`, food-only with date → `not_allowed`,
unavailable rental → 400, delivery fee stored and in total,
Google-down fallback marks `straight_line`, delivery disabled → 400,
out-of-range → 400, wrong `expected_delivery_fee` → 409 and no order
row, typed address while PDOK is up → 400, typed address while PDOK is
down → 201 with `delivery_fee_pending`, payment claim on a pending
order → 409. Put the HTTP calls behind a small
`trait DeliveryBackend` so tests inject fixed coordinates/distances.

## Group 5 — API: admin delivery settings (Decisions 5, 6, 14, 15)

`apps/api/src/routes/admin/delivery.rs`:

- `GET /api/admin/delivery-settings` → all fields incl. origin and
  `updated_at`.
- `PUT /api/admin/delivery-settings` with `updated_at` (409 on
  mismatch). `origin_address_id` (PDOK id) re-geocodes via `lookup` and
  stores label + coordinates. Validation: rate 0–100, minimum 0–500,
  max distance empty or 1–500; `delivery_enabled = true` without origin
  coordinates → `400 delivery_enabled origin_required` (Decision 6).
- `PUT /api/admin/orders/{id}/delivery-fee`
  `{ fee, address_id? , updated_at }` (Decision 17 / risk 6): allowed
  only for delivery orders in `awaiting_payment` with no payment claim
  (`409 not_editable` otherwise). With `address_id` it recomputes via
  `quote` and stores label/distance/method; with only `fee` the method
  is `manual`. Recomputes `total`, clears `delivery_fee_pending`, sends
  `delivery_fee_confirmed` to the customer.
- `GET /api/admin/delivery/suggest?q=` and
  `POST /api/admin/delivery/quote { address_id }` (same as public, but
  returns origin-free results too — used by "Test an address" and the
  quote editor).
- Behind the existing admin auth + Origin check (Phase 33).

## Group 6 — Web: cart and rentals page (Decision 2)

- `lib/cart-context.tsx`: `CartLine.kind: "menu" | "rental"`;
  `isCartLine` accepts missing `kind` (→ `"menu"`); new
  `rentalLineKey(id) = \`rental:${id}\``. Keep storage key `v1`
  (backward compatible).
- `lib/api.ts` order payload: map lines to `{ menu_item_id, … }` or
  `{ rental_id, quantity }`.
- `app/[locale]/rentals/page.tsx`: replace per-card "Reserve on
  WhatsApp" with `AddToCartControls` (generalise it to take a
  `NewCartLine` with `kind`, or add a thin `AddRentalToCart` wrapper —
  reuse the Phase 30 fly/toast feedback). Keep the closing WhatsApp
  note.
- `components/cart-line-item.tsx`: show `RentalsPage.perEvent` after
  the unit price for rental lines.
- Cart page: under the subtotal, "Delivery fee is calculated at
  checkout from €{rate}/km" (from `/api/delivery/info`, hidden if
  disabled).

## Group 7 — Web: checkout address calculator and rental date (Decisions 4, 9, 11, 12, 17)

- `components/delivery-address-field.tsx` (client): accessible
  combobox (input + listbox, `aria-expanded`, `aria-activedescendant`,
  ↑/↓/Enter/Esc), 300 ms debounce, `AbortController` per request,
  calls `/api/delivery/suggest`; on pick calls `/api/delivery/quote`
  and reports `{ addressId, label, distanceKm, fee, estimatedBy }` up.
  States: idle, loading ("Calculating…"), result
  ("About 12.3 km · Delivery €12.30"), out of range, unavailable
  (message + WhatsApp link). Plus an optional "Apartment, floor,
  instructions" input (`delivery_address_extra`).
- `components/order-details-form.tsx`: replace the free-text address
  with the field; `DeliveryType` option hidden when `info.enabled` is
  false; validation requires a quoted address for delivery. Add the
  "Rental date" `<input type="date">` (min tomorrow, max +365) shown
  only when the cart has rental lines.
- `components/order-review.tsx` / summary: "Delivery" line — "Free"
  for pickup, the quoted fee, or "Calculated from your address";
  total = subtotal + fee.
- Order success, tracking (`order-view.tsx`, `order-invoice.tsx`):
  show delivery km and fee, rental date, and "per event" on rental
  lines.
- Map API errors `out_of_range`, `unavailable`, `rental_date.*`,
  `items[i].rental_id` to messages.
- **Fee changed (Decision 12):** send `expected_delivery_fee` with the
  order; on `409 delivery_fee_changed` update the field's quote and the
  summary from the response, show `CheckoutPage.feeChanged` above the
  submit button, focus it, and keep every other value. The next submit
  sends the new fee.
- **Lookup down (Decision 17):** when `suggest` or `quote` returns
  `503 address_lookup_down` (or the request fails/times out), the
  field switches to a plain text address input with
  `CheckoutPage.lookupDown`; the summary shows "Delivery: to be
  confirmed" and the total as "€{subtotal} + delivery". The order is
  sent with `delivery_address` and no id. If the API answers
  `400 delivery_address_id required` (PDOK came back), switch back to
  the combobox with the typed text pre-filled.
- Tracking/success for `delivery_fee_pending`: "Delivery fee: to be
  confirmed", `OrderPage.feePendingNote`, no payment instructions or
  "I've paid" button; `order-auto-refresh` picks up the fee once set.

## Group 8 — Web: admin (Decisions 6, 13, 14, 15, 17)

- Admin order list/detail: delivery fee + km + method badge
  ("Straight-line estimate" when `straight_line`, "Set by you" when
  `manual`, "Delivery fee needed" in red when pending), rental date,
  rental lines marked "Rental".
- Admin order detail: "Set delivery fee" panel
  (`components/admin-order-delivery-fee.tsx`) on delivery orders
  awaiting payment with no claim: the stored address, a PDOK lookup
  that pre-fills the calculated fee, a manual fee input, Save
  (conflict-aware). Hidden otherwise.
- Admin dashboard (`app/admin/(protected)/page.tsx`): while
  `delivery_enabled` is false, a notice "Delivery is off — check the
  business address in Settings and switch it on." linking to Settings.
  Orders awaiting a delivery fee count toward the PWA badge (Phase 27).
- `app/admin/(protected)/settings/page.tsx`: "Delivery" section
  (`components/admin-delivery-settings.tsx`): business address combobox
  (reuses `DeliveryAddressField` pointed at the admin routes) with the
  private hint, rate, minimum, max distance, on/off, Save with conflict
  handling, and "Test an address". The on switch is disabled with "Set
  the business address first" until the origin has coordinates.
- `components/admin-quote-editor.tsx`: "Calculate delivery" button per
  Decision 14, inserting/updating the line whose description starts
  with "Delivery (".

## Group 9 — Copy (Decision 16)

`messages/{en,nl,fr}.json` — new keys (English shown; nl/fr written
alongside, `pnpm check:messages` must pass):

| Key | en |
|---|---|
| `CheckoutPage.addressSearchLabel` | Delivery address |
| `CheckoutPage.addressSearchHint` | Start typing your street and house number |
| `CheckoutPage.addressExtraLabel` | Apartment, floor or instructions (optional) |
| `CheckoutPage.deliveryEstimate` | About {km} km · Delivery {fee} |
| `CheckoutPage.calculating` | Calculating delivery… |
| `CheckoutPage.deliveryPending` | Calculated from your address |
| `CheckoutPage.outOfRange` | We don't deliver this far yet. Choose pickup or message us on WhatsApp. |
| `CheckoutPage.deliveryUnavailable` | We can't calculate delivery right now. Choose pickup or message us on WhatsApp. |
| `CheckoutPage.noMatches` | No Dutch addresses match. Check the spelling or message us on WhatsApp. |
| `CheckoutPage.rentalDateLabel` | Rental date |
| `CheckoutPage.rentalDateHint` | The day of your event. We'll confirm pickup and return times with you. |
| `CartPage.deliveryFromRate` | Delivery fee is calculated at checkout from {rate}/km |
| `RentalsPage.perEvent` | per event |
| `OrderPage.deliveryLine` | Delivery ({km} km) |
| `OrderPage.rentalDate` | Rental date |
| `CheckoutPage.feeChanged` | The delivery fee changed to {fee}. Please check your total and place the order again. |
| `CheckoutPage.lookupDown` | We can't look up addresses right now. Place your order and we'll confirm the delivery fee before you pay. |
| `CheckoutPage.deliveryToConfirm` | To be confirmed |
| `CheckoutPage.totalPlusDelivery` | {subtotal} + delivery |
| `OrderPage.feePendingNote` | We're confirming your delivery fee. You'll get an email with the final total and payment details shortly. |

Email bodies get the same lines in `order_emails.rs` per locale.

## Group 10 — Specs and docs

- `docs/deployment.md`: `GOOGLE_ROUTES_API_KEY` — create a Google
  Cloud project under the client's account with billing, enable only
  the Routes API, create a key restricted to the Routes API (and the
  server's static IP), set the Routes API quota to 500 requests/day,
  add a €10/month budget alert to the owner's email, and store the key
  as a GitHub Actions secret / server env.
- `docs/privacy.md`: PDOK receives the typed address search text and
  Google receives the delivery coordinates (no name or address text) (processors list); the privacy page's
  processor section in en/nl/fr gets one line each.
- `specs/tech-stack.md` already updated in planning (Delivery fee
  section).
