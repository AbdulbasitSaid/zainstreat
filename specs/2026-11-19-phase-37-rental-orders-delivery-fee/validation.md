# Phase 37 — Home Tiles, Rental Orders & Delivery Fee: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the fee/haversine unit tests and the rental-order and
      delivery-order `#[sqlx::test]`s (no network calls in tests).
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] Exactly one new migration (`…_rental_orders_delivery`) with a
      `.down.sql`; `sqlx migrate run` → `revert` → `run` works on a dev
      database.
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`. No new npm or cargo packages.
- [ ] `/validate` passes.

### Home tiles

- [ ] On `/en`, `/nl`, `/fr`: Meals → `/{locale}/menu`; Snacks →
      `/{locale}/menu?category={Snacks id}` showing only Snacks;
      Catering → `/{locale}/contact?tab=catering` with the catering tab
      open; Catering Rentals → `/{locale}/rentals`.
- [ ] With the Snacks category archived, the Snacks tile goes to
      `/menu`.

### Rental orders

- [ ] `/en/rentals` cards have add-to-cart controls with the Phase 30
      feedback; no per-card "Reserve on WhatsApp"; the closing WhatsApp
      note remains.
- [ ] Cart holds a rental and a menu item together; rental line shows
      "per event"; a cart saved before this phase still loads.
- [ ] Checkout shows "Rental date" only when the cart has a rental;
      today is rejected, tomorrow accepted.
- [ ] `curl -X POST /api/orders` with `{ rental_id, quantity: 2 }` and
      a valid `rental_date` → `201`, line priced from the DB; without
      `rental_date` → `400 rental_date required`; food-only with
      `rental_date` → `400 not_allowed`; unavailable rental →
      `400 items[0].rental_id unavailable`; both ids on one line →
      `400`.
- [ ] Invoice email, tracking page and admin order show the rental
      date and the rental lines.

### Delivery fee

- [ ] `GET /api/delivery/info` → `{ enabled: false, rate_per_km: "1.00", minimum_fee: "0.00", max_distance_km: null }`
      on a fresh staging deploy, checkout offers pickup only, and the
      admin dashboard shows the "Delivery is off" notice; after the
      owner switches delivery on in Settings, `enabled: true`; no origin field in any public response
      (`grep -i origin` on the JSON of info/suggest/quote is empty).
- [ ] `delivery_settings.origin_address` on staging equals the current
      event's address after the migration.
- [ ] The Settings on switch is disabled until the origin has
      coordinates; `PUT` with `delivery_enabled: true` and no origin →
      `400 origin_required`.
- [ ] Checkout: choosing Delivery and typing "Stationsplein 1 Almere"
      shows suggestions; picking one shows "About {km} km · Delivery
      €{fee}" with fee = km × €1.00, and the summary total updates.
      Keyboard only (↑/↓/Enter/Esc) works; VoiceOver announces options.
- [ ] With `MAPBOX_ACCESS_TOKEN` set, the estimate shows "Driving
      distance by © Mapbox © OpenStreetMap" (links work) and the km is
      within ~5 % of a map app's driving distance for the same addresses.
- [ ] Placing that order stores the same fee,
      `delivery_rate_per_km = 1.00`, `delivery_estimated_by = route` and
      **no** `delivery_distance_km` (Mapbox terms); emails, tracking and
      admin show "Delivery: €{fee}" without km;
      `total = subtotal + fee` in DB, invoice email, tracking and admin.
- [ ] `curl -X POST /api/orders` with `delivery_type: delivery` and no
      `delivery_address_id` → `400`; a client-sent `delivery_fee` is
      ignored.
- [ ] With `MAPBOX_ACCESS_TOKEN` unset (or an invalid token), the quote
      still works and returns `estimated_by: "straight_line"`; admin
      shows "Straight-line estimate" and the order keeps its km; the API
      log line has the status only, never the Mapbox URL or token.
- [ ] `MAPBOX_ACCESS_TOKEN` is a GitHub Actions secret for staging and
      never appears in the web bundle (`grep -r pk\. apps/web/.next` is
      empty).
- [ ] Fee changed: open checkout, get a quote, change the rate in
      admin, place the order → `409 delivery_fee_changed`, no order row,
      checkout shows the new fee and message; placing again → `201` at
      the new fee.
- [ ] Lookup down (point the PDOK base URL at an unreachable host on a
      dev API): checkout switches to a typed address with the
      "can't look up addresses" note; the order is placed with
      `delivery_fee_pending = true`; tracking shows "Delivery fee: to be
      confirmed" and no payment instructions; `POST …/payment-claim` →
      `409 delivery_fee_pending`; owner email/push say "Delivery fee
      needed".
- [ ] With PDOK up, an order with a typed `delivery_address` and no id
      → `400 delivery_address_id required`.
- [ ] Admin "Set delivery fee" on that order (lookup or manual) sets
      the fee, recomputes the total, clears the flag, and the customer
      receives the "delivery fee is confirmed" email with payment
      details; the tracking page then shows payment instructions. The
      panel is hidden once a payment is claimed.
- [ ] Admin Settings → Delivery: change rate to 1.50 → new quotes use
      it; set minimum €5 → a 2 km address costs €5.00; set max 10 km →
      a farther address shows the out-of-range message and the API
      rejects the order; turn delivery off → checkout shows pickup only
      and the API rejects delivery; stale save → conflict message.
- [ ] Changing the business address in admin changes quotes for the
      same customer address (cache cleared).
- [ ] "Test an address" in admin shows distance and fee.
- [ ] Admin quote editor: "Calculate delivery" adds a
      "Delivery ({km} km)" line at the fee; pressing again updates it.
- [ ] Pickup orders and old orders display exactly as before (fee €0,
      no delivery line).
- [ ] Unauthenticated `GET/PUT /api/admin/delivery-settings` and
      `PUT /api/admin/orders/{id}/delivery-fee` → `401`.
- [ ] `docs/privacy.md` and the privacy page list PDOK and Google
      (coordinates only) as processors.
- [ ] Hammering `/api/delivery/suggest` past the limit → `429`.
- [ ] API logs contain no origin address or coordinates.

## Definition of done

The home collage's four tiles open the full menu, the Snacks category,
the catering quote form and the rentals page; rentals are added to the
same cart as food and ordered with a rental date through the existing
checkout and bank-transfer flow; every delivery order is charged a
server-calculated fee (€1/km of driving distance by default, from a
private business address seeded from the current event) that customers
see as an estimate as soon as they pick their address, and the owner
controls rate, minimum, maximum distance, on/off and origin in admin;
fee changes are confirmed by the customer before ordering, and when
address lookup is down orders still go through with the fee confirmed
by the owner before payment —
all with passing tests, `pnpm build` and `/validate` on staging.
