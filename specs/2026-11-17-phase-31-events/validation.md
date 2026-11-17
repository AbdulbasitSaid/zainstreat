# Phase 31 — Events: Bookings, Plates & Past Events: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the new capacity, cutoff, waitlist, sweeper, confirm and
      address-privacy tests, and `tests/order_status_schema.rs`.
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] The migration applies on a fresh database and on a copy of the
      current staging data. The down migration reverts it.
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`.
- [ ] `qrcode.react` is the only new package in `apps/web/package.json`.
- [ ] `/validate` passes.

### Public API

- [ ] `curl /api/events?locale=en` lists the Abula Experience under
      `upcoming`, with `seats_left: 16`.
- [ ] `curl /api/events/the-abula-experience-2026` has plates and terms,
      and `grep -c address` on the response is 0.
- [ ] An unpublished event's slug gives `404`.
- [ ] Booking 7 guests (max party 6) gives `400`. Booking meals with a
      faked clock past the meal cutoff (test) gives `400`. Booking meals
      without `accept_terms` gives `400`.
- [ ] A filled honeypot gives a fake `201` and no row. Rapid repeated
      posts hit the rate limit (`429`).

### Booking flow (staging, en)

- [ ] On `/en/events`, the event card shows the date, "16 seats left" and
      Book a seat.
- [ ] Booking 3 guests (Abula, Jollof, no meal) shows a €30 total, needs
      the terms checkbox, and lands on the booking page with the invoice,
      `ZTM-…` reference, hold deadline and "I've paid". No address.
- [ ] The guest gets one email (en → nl → fr) with the invoice and the
      booking link. The owner gets an email and a push that opens the
      event's bookings tab. No separate "new order" email or push.
- [ ] The seats count drops to 13 on `/en/events`.
- [ ] Pressing "I've paid", then "Confirm payment" in admin Orders: the
      booking becomes Confirmed, the booking page shows the address, and
      the guest gets the confirmation email with the address and plates.
      The order's normal status email isn't sent.
- [ ] A 2-guest booking with no meals is confirmed at once with the
      address, without an invoice.
- [ ] With 1 seat left, a 2-guest booking shows "Only 1 seat left" and
      Join the waitlist. Joining gives a waitlisted booking page, emails,
      and no seats taken.
- [ ] In admin, Offer seats on that waitlisted booking fails with a
      message while it doesn't fit, and works after another booking is
      cancelled (new invoice, new 48-hour hold, "seats offered" email).
- [ ] Guest cancel from the booking page frees the seats and emails
      guest and owner. The button is gone after the meal cutoff (set the
      cutoff in the past on a test event).

### Hold sweeper

- [ ] On a test event with `payment_hold_hours = 1` and a booking's
      `hold_expires_at` set in the past by SQL: within 5 minutes the
      booking is cancelled (`expired`), its order is `cancelled`, the seats
      are free, and the guest and owner are emailed.
- [ ] The same with an order in `payment_claimed`: nothing changes.
- [ ] Restarting the `api` container mid-sweep releases nothing twice
      (one email per booking).

### Order integration

- [ ] Event orders show in admin Orders with the Event tag and title,
      and link to the booking. Their status choices after confirming are
      only Completed and Cancelled.
- [ ] Cancelling an event order in admin cancels its booking.
- [ ] `/en/order/{token}` for an event order links back to the booking
      page and shows no pickup/delivery steps.

### Admin Events panel

- [ ] Events is in the admin nav. The list shows date, status, seats,
      waitlist count and paid total.
- [ ] Creating an event with en only saves it as a draft. It's not on
      `/en/events`, and the Preview link renders it under the `cms.`
      preview host. Publishing shows it publicly, and nl/fr fall back to
      English text.
- [ ] The cover crop is 16:9. Plates can be added, reordered and made
      unavailable. The slug field locks after the first booking.
- [ ] The bookings tab shows the plate summary (confirmed vs held), and
      the CSV downloads with every guest.
- [ ] The share tab's QR code scans on a phone to the right locale's URL.
      Download PNG gives a 1024px image. Copy and WhatsApp work.
- [ ] On a past event, the recap and 3+ gallery photos (with captions,
      reordered) show on its public page with a working lightbox
      (keyboard and swipe), and the booking form is gone.
- [ ] Unpublishing a past event hides it from `/en/events`. Archiving
      hides it from the admin's default list.

### Look, locales, accessibility

- [ ] `/en/events`, `/nl/events`, `/fr/events` and the event page use the
      liquid cards, cover splash and pot-shaped seats meter, and look at
      home next to `/menu` in the device simulator, Mobile S to Desktop XL.
- [ ] With reduced motion, the meter and reveals are static.
- [ ] The header shows Events between Menu and Contact in all three
      locales, and `sitemap.xml` lists `/events` and the event in en/nl/fr.
- [ ] The form's errors are announced (`aria-invalid`/`aria-describedby`),
      and guest rows and plate radios can be used by keyboard only.

### Staging seed

- [ ] `seed-staging-events.yml` runs green on staging. Running it twice
      leaves one event and doesn't touch existing bookings.
- [ ] A fresh "staging up" seeds the event automatically.
- [ ] `https://<ip>.sslip.io/en/events/the-abula-experience-2026` shows 27
      December 2026, 14:00, two €15 plates, 16 seats and a meal cutoff of
      20 December.
- [ ] `seed-production.yml` and production are untouched.

## Definition of done

The owner can create, publish, share (link, WhatsApp, QR) and run an
event from the `cms.` admin, and add a recap and photo gallery
afterwards. Guests find upcoming and past events on `/events` in en, nl
or fr, in the site's liquid soup style. They book seats and a plate per
person in one form, get an instant bank-transfer invoice through the
existing order flow (or an instant confirmation with no meals), and see
the private address once confirmed. Capacity is never oversold, full
events take a waitlist the owner can offer seats to, unpaid holds are
released after 48 hours, and meal orders close at the cutoff. The
Abula Experience is seeded on staging with placeholder prices, tests,
lint, build, `check:messages` and `/validate` pass, and production is
untouched.
