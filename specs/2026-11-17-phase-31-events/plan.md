# Phase 31 — Events: Bookings, Plates & Past Events: Implementation Plan

**Ordering:** API first (Groups 1–5), then web (Groups 6–10), then
seeding (Group 11). Run `cargo test` after Group 5, and `/validate` after
Group 10 and again after Group 11.

Paths refer to `master` at `3ba70c2` (Phase 30 merged).

## Group 0 — Preconditions

0.1. Phase 30 is merged (PR #51). Done.
0.2. Staging can be brought up ("staging up" workflow), with the SES
     staging identities verified for the test email addresses.

## Group 1 — Migration (API)

Decisions 1–3, 6, 8, 9, 12, 13, 15.

1.1. `apps/api/migrations/20261117120000_events.up.sql`:
     ```sql
     -- Phase 31: events, plates, bookings (requirement.md Decisions 1-15).
     CREATE TABLE events (
         id BIGSERIAL PRIMARY KEY,
         slug TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
         -- {"en": {...}, "nl": {...}, "fr": {...}}; keys: title, summary,
         -- story, terms, recap. "en" required (Decision 15).
         content JSONB NOT NULL CHECK (content ? 'en'),
         starts_at TIMESTAMPTZ NOT NULL,
         ends_at TIMESTAMPTZ,
         area TEXT NOT NULL,              -- public
         address TEXT NOT NULL,           -- confirmed guests only (Decision 10)
         capacity INTEGER NOT NULL CHECK (capacity > 0),
         max_party_size INTEGER NOT NULL DEFAULT 6 CHECK (max_party_size > 0),
         meal_orders_close_at TIMESTAMPTZ,
         rsvp_closes_at TIMESTAMPTZ,
         payment_hold_hours INTEGER NOT NULL DEFAULT 48 CHECK (payment_hold_hours > 0),
         cover_image_url TEXT,
         published_at TIMESTAMPTZ,
         archived_at TIMESTAMPTZ,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
         updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
     );

     CREATE TABLE event_plates (
         id BIGSERIAL PRIMARY KEY,
         event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
         position INTEGER NOT NULL,
         content JSONB NOT NULL CHECK (content ? 'en'), -- name, description
         price NUMERIC(10, 2) NOT NULL CHECK (price >= 0),
         is_available BOOLEAN NOT NULL DEFAULT true,
         image_url TEXT
     );

     CREATE TABLE event_photos (
         id BIGSERIAL PRIMARY KEY,
         event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
         position INTEGER NOT NULL,
         image_url TEXT NOT NULL,
         caption TEXT
     );

     CREATE TABLE event_bookings (
         id BIGSERIAL PRIMARY KEY,
         event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
         public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
         status TEXT NOT NULL CHECK (status IN
             ('waitlisted', 'awaiting_payment', 'confirmed', 'cancelled')),
         cancel_reason TEXT CHECK (cancel_reason IN ('expired', 'admin', 'guest')),
         customer_name TEXT NOT NULL,
         customer_email TEXT NOT NULL,
         customer_phone TEXT NOT NULL,
         party_size INTEGER NOT NULL CHECK (party_size > 0),
         notes TEXT,
         locale TEXT NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'nl', 'fr')),
         order_id BIGINT UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
         hold_expires_at TIMESTAMPTZ,
         confirmed_at TIMESTAMPTZ,
         cancelled_at TIMESTAMPTZ,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
         CONSTRAINT event_bookings_cancel_reason
             CHECK ((status = 'cancelled') = (cancel_reason IS NOT NULL))
     );
     CREATE INDEX event_bookings_event_status ON event_bookings (event_id, status);
     CREATE INDEX event_bookings_hold ON event_bookings (hold_expires_at)
         WHERE status = 'awaiting_payment';

     -- One row per guest; plate_id NULL = attending without a meal.
     -- Name and price are snapshots (mission.md rule 2).
     CREATE TABLE event_booking_guests (
         id BIGSERIAL PRIMARY KEY,
         booking_id BIGINT NOT NULL REFERENCES event_bookings(id) ON DELETE CASCADE,
         position INTEGER NOT NULL,
         plate_id BIGINT REFERENCES event_plates(id) ON DELETE SET NULL,
         plate_name TEXT,
         unit_price NUMERIC(10, 2),
         UNIQUE (booking_id, position),
         CHECK ((plate_name IS NULL) = (unit_price IS NULL))
     );

     ALTER TABLE orders
         DROP CONSTRAINT orders_delivery_type_check,
         ADD CONSTRAINT orders_delivery_type_check
             CHECK (delivery_type IN ('pickup', 'delivery', 'on_site', 'event'));
     ```
     `.down.sql` drops the tables in reverse order and restores the
     three-value CHECK (after deleting `event` orders' items and orders).
1.2. Regenerate `apps/api/.sqlx/` with `cargo sqlx prepare` once the
     queries in Groups 2–5 exist.

## Group 2 — Domain module and order lifecycle (API)

Decisions 2, 3, 6, 15.

2.1. `apps/api/src/events.rs`:
     - `type Localized<T> = HashMap<String, T>` helpers:
       `pick(content, locale, key) -> Option<String>` with English
       fallback when the locale's value is missing or blank.
     - `seats_taken(tx, event_id) -> i64`:
       `SELECT COALESCE(SUM(party_size), 0) FROM event_bookings WHERE event_id = $1 AND status IN ('awaiting_payment', 'confirmed')`.
     - `is_upcoming(event, now)`, `meals_open(event, now)`,
       `rsvp_open(event, now)`.
     - `slugify(title) -> String` (lowercase ASCII, strip diacritics with a
       small `match` table for àáâäèéêëìíîïòóôöùúûüẹọṣñç, `-` for runs of
       anything else, trimmed).
2.2. `order_status.rs`: in `admin_next`, add
     `("processing", "event", _) => &["completed", "cancelled"]` **above**
     the catch-all `("processing", _, _)` arm. `completed` by customer
     never happens for `event` (no `CUSTOMER_COMPLETABLE` change). Add a
     unit test for the `event` branch.
2.3. `apps/web/lib/orders.ts`: add `"event"` to `FulfilmentType` and its
     label ("Event"), so the admin order pages render it.
2.4. In the admin order status handler (`routes/admin/orders.rs`) and
     `confirm payment`, after updating an `event` order, in the same
     transaction:
     ```sql
     UPDATE event_bookings SET status = 'confirmed', confirmed_at = now(), hold_expires_at = NULL
      WHERE order_id = $1 AND status = 'awaiting_payment';           -- → processing
     UPDATE event_bookings SET status = 'cancelled', cancel_reason = 'admin', cancelled_at = now()
      WHERE order_id = $1 AND status <> 'cancelled';                  -- → cancelled
     ```
     Then send the booking email (Group 4) instead of the order status
     email for `delivery_type = 'event'`.

## Group 3 — Public event and booking endpoints (API)

Decisions 4–11, 14, 17.

3.1. `apps/api/src/routes/events.rs`, registered in `routes/mod.rs`:

     | Route | Handler | Notes |
     |---|---|---|
     | `GET /api/events?locale=` | `list_events` | Published, not archived. `{ upcoming: [...], past: [...] }`, upcoming by `starts_at` ASC, past DESC. Each: slug, title, summary, starts_at, area, cover, seats_left, waitlist_open, has_gallery. |
     | `GET /api/events/{slug}?locale=` | `get_event` | Published only (404 otherwise). Adds story, terms, recap, plates (available only), photos, capacity, max_party_size, meals_open, rsvp_open, seats_left. **Never `address`.** |
     | `POST /api/events/{slug}/bookings` | `create_booking` | Rate-limited + honeypot (same layer as `/catering-enquiries`). |
     | `GET /api/events/bookings/{token}` | `get_booking` | Booking, event summary, guests, total, order token + payment instructions when it has an order, `address` only when `confirmed`. |
     | `POST /api/events/bookings/{token}/cancel` | `cancel_booking` | Guest cancel (Decision 14). |

3.2. `create_booking` request:
     ```rust
     struct CreateBookingRequest {
         customer_name: String, customer_email: String, customer_phone: String,
         guests: Vec<Option<i64>>,   // plate id per person, None = no meal
         notes: Option<String>,
         locale: Option<String>,
         accept_terms: bool,         // required when any guest has a meal
         join_waitlist: bool,        // set by the form's "Join the waitlist" retry
         website: Option<String>,    // honeypot
     }
     ```
     Validation (`400 validation_error` with `FieldError`s, like `orders.rs`): name/email/
     phone as in `orders::validate`; `1 <= guests.len() <= max_party_size`;
     every plate id belongs to the event and is available; meals only
     while `meals_open`; `accept_terms` when any meal; `rsvp_open`.
     Then, in one transaction:
     1. `SELECT … FROM events WHERE slug = $1 AND published_at IS NOT NULL AND archived_at IS NULL FOR UPDATE`.
     2. `seats_left = capacity - seats_taken`. If `guests.len() > seats_left`
        and `!join_waitlist` → `409 { error: "not_enough_seats", seats_left }`.
        If `join_waitlist` → insert as `waitlisted`.
     3. Otherwise insert the booking and guest rows (plate name in the
        booking's locale, falling back to en, as `plate_name`; price as
        `unit_price`). With no meals → `confirmed`. With meals → insert the
        `orders` row (`delivery_type = 'event'`, `notes` = "{event title},
        {date}" + the guest's note, totals from the plates, no delivery
        fee) and one `order_items` row per distinct plate
        (`item_name = "{event title} — {plate}"`, `quantity` = count),
        set `order_id`, status `awaiting_payment`, `hold_expires_at = now()
        + payment_hold_hours`.
     4. Commit, then fire-and-forget the emails and owner push (Group 4).
     Response `201 { token, status }`. The web redirects to the booking page.
3.3. `cancel_booking`: allowed while `status IN ('waitlisted',
     'awaiting_payment', 'confirmed')` and `meals_open` (or no meal
     cutoff set and the event hasn't started). Cancels the booking
     (`guest`) and its order if any (`cancelled_at`). `409` otherwise.
     Emails the guest and the owner, with push.
3.4. `claim_payment` (`routes/orders.rs`) needs no change: an event order
     is an ordinary order. `get_booking` reads the order's status, so the
     booking page shows "payment claimed" too.

## Group 4 — Emails and push (API)

Decision 19.

4.1. `apps/api/src/event_emails.rs`, following `order_emails.rs` (one
     email, en → nl → fr sections, joined subject, brand sender name,
     `Reply-To: contact@`, `email_deliveries` recorded). Templates:
     `booking_awaiting_payment` (includes the invoice block from
     `payment.rs`, the hold deadline and the terms), `booking_confirmed`
     (date, time, **address**, guests and plates), `booking_waitlisted`,
     `seats_offered` (then the awaiting-payment or confirmed content),
     `booking_released` ("contact us if you've already paid"),
     `booking_cancelled`. Each links to
     `{site_url}/{locale}/events/booking/{token}`.
4.2. Owner emails to `EMAIL_BUSINESS_NOTIFY_ADDRESS` (Reply-To the
     guest) for: new booking, waitlist entry, guest cancellation,
     auto-release.
4.3. `push.rs`: `event_booking(event_id, name, party, title)`,
     `event_waitlist(…)`, `event_cancelled(…)`, `event_released(…)`.
     URL `/events/{event_id}?tab=bookings` (passes `is_admin_path`).
4.4. Skip `order_emails` status mails when `delivery_type = 'event'`
     (Group 2.4 sends the booking mail instead). The new-order **owner**
     email and push are replaced by the booking ones, so the owner isn't
     notified twice.

## Group 5 — Admin endpoints and the hold sweeper (API)

Decisions 7, 8, 12, 13, 18, 20.

5.1. `apps/api/src/routes/admin/events.rs` under the protected
     `/api/admin` group:

     | Route | Purpose |
     |---|---|
     | `GET /events` | List with seats taken, waitlist count, paid total. |
     | `POST /events` | Create (draft). Slug from the en title, deduplicated with `-2`, `-3`. |
     | `GET /events/{id}` | Everything, including address, plates, photos. |
     | `PUT /events/{id}` | Update. `400` on slug change once a booking exists. Plates replaced as a list (existing ids kept, missing ones set `is_available = false` if referenced by a guest, deleted otherwise). |
     | `POST /events/{id}/publish`, `/unpublish`, `/archive`, `/unarchive` | Flags. |
     | `PUT /events/{id}/photos` | Replace the ordered gallery list (≤ 30). |
     | `GET /events/{id}/bookings` | Bookings with guests, order status, plus the plate summary (`confirmed` and `held` counts per plate and "no meal"). |
     | `GET /events/{id}/bookings.csv` | Name, email, phone, party, plates, status, paid, note. |
     | `POST /events/bookings/{id}/offer` | Waitlisted/cancelled → the create_booking step 3 path with a fresh hold, ignoring the meal cutoff; `409` if it doesn't fit. Emails `seats_offered`. |
     | `POST /events/bookings/{id}/cancel` | Admin cancel (and its order). |

5.2. `routes/admin/orders.rs` list/detail: join `event_bookings` →
     `events` so event orders carry `event: { id, title, booking_id }`.
5.3. **Sweeper** in `apps/api/src/events.rs`:
     ```rust
     pub fn spawn_hold_sweeper(state: AppState) {
         tokio::spawn(async move {
             let mut tick = tokio::time::interval(Duration::from_secs(300));
             tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
             loop {
                 tick.tick().await;
                 if let Err(error) = release_expired_holds(&state).await {
                     tracing::error!(%error, "event hold sweep failed");
                 }
             }
         });
     }
     ```
     `release_expired_holds` runs in one transaction:
     ```sql
     WITH expired AS (
       UPDATE orders o SET status = 'cancelled', cancelled_at = now()
         FROM event_bookings b
        WHERE b.order_id = o.id AND b.status = 'awaiting_payment'
          AND b.hold_expires_at < now() AND o.status = 'awaiting_payment'
       RETURNING o.id)
     UPDATE event_bookings SET status = 'cancelled', cancel_reason = 'expired', cancelled_at = now()
      WHERE order_id IN (SELECT id FROM expired)
     RETURNING id, public_token, event_id, customer_name, customer_email, locale, party_size;
     ```
     Then it sends `booking_released` and the owner email and push for each
     row. Called from `main.rs` after the router is built, before
     `axum::serve`.
5.4. Tests (`apps/api/tests/`): capacity (two bookings racing for the
     last seats, one gets `409`), the meal cutoff `400`, the waitlist path,
     the sweeper releasing an expired hold but not a `payment_claimed` one,
     confirm-payment flipping the booking to `confirmed`, the address
     missing from public responses until confirmed, and the `admin_next`
     `event` branch. Run `cargo test` and `cargo clippy -- -D warnings`.

## Group 6 — Shared web plumbing

6.1. `apps/web/lib/events.ts`: types (`EventSummary`, `EventDetail`,
     `EventPlate`, `EventPhoto`, `Booking`, `BookingStatus`) and fetchers
     (`getEvents(locale)`, `getEvent(slug, locale)`, `getBooking(token)`)
     using `API_BASE_URL` and `cache: "no-store"`, like `lib/quotes.ts`.
     No `any`.
6.2. `apps/web/lib/admin-events.ts` + `admin-events-api.ts`, like the
     menu-items pair.
6.3. Same-origin proxies, like `app/api/quotes/…`:
     `app/api/events/[slug]/bookings/route.ts` (POST, forwards
     `X-Forwarded-For`), `app/api/events/bookings/[token]/cancel/route.ts`,
     and `app/admin/api/events/**` through `admin-proxy.ts`.
6.4. `pnpm add qrcode.react@^4.2` in `apps/web`.

## Group 7 — Public pages (web)

Decisions 10, 11, 13, 16, 17.

7.1. `app/[locale]/events/page.tsx`: `generateMetadata`, `PageHero`,
     Upcoming as a bento of `EventCard` (`components/event-card.tsx`, built
     on `liquid-card.tsx`: cover via `SiteImage`, a date badge
     "27 DEC · 14:00" in the locale, title, summary, area, a seats chip
     "4 seats left" / "Waitlist open" / "Fully booked", and a "Book a seat"
     `ButtonLink`). Past events below in a quieter 3-column grid
     (`PastEventCard`: cover, date, title, "See photos" if any). Empty
     states: "No upcoming events right now — follow us on WhatsApp to hear
     about the next one" (with `buildWhatsAppLink`).
7.2. `app/[locale]/events/[slug]/page.tsx`: `generateMetadata` with the
     cover as the OG image. The cover in a hero with `LiquidSurface` on its
     lower edge, chips (date, time, area, "Free entry · Paid meals"), the
     story, a **plates** row of menu-style tiles (name, description,
     price), the **seats meter** (`components/event-seats-pot.tsx`: a pot
     outline whose liquid level is `taken / capacity`, animated up with
     GSAP on reveal, static under reduced motion, with a text label for
     screen readers), the guest wording (Decision 16), and the form. Past
     → recap + `EventGallery`.
7.3. `components/event-booking-form.tsx` (client): fields per Decision
     17. A `party` stepper (`quantity-stepper.tsx`) grows or shrinks the
     guest rows. Each row is a radio group of plates plus "Attending
     without a meal". Plates are disabled with a note after the meal
     cutoff. A live total, the terms in a `<details>` with the required
     checkbox, the honeypot, `aria-invalid`/`aria-describedby` like the
     order form. On `409 not_enough_seats` it shows "Only {n} seats left"
     with **Join the waitlist** (resubmits with `join_waitlist: true`) or
     "Change party size". On success `router.push` to the booking page.
7.4. `app/[locale]/events/booking/[token]/page.tsx`: status header
     (`order-progress.tsx`-style steps: Booked → Paid → Confirmed, or a
     waitlist/cancelled state), event summary, a guest/plate table with
     the total, then by status: `OrderInvoice` + `PaymentClaimButton`
     (order token) and the hold deadline; the **address** card with a map
     link (`https://maps.google.com/?q=…`) once confirmed; waitlist text.
     Cancel button per Decision 14 (`admin-confirm-dialog`-style native
     `<dialog>`). `OrderAutoRefresh` keeps it current.
7.5. `components/event-gallery.tsx` (client): a masonry-ish grid with a
     `<dialog>` lightbox (arrow keys, swipe, Esc), `next/image` with
     `sizes`.
7.6. `app/[locale]/order/[token]/page.tsx`: when the order is an `event`
     order, show "This payment is for your booking at {title}" with a link
     to the booking page, and hide the pickup/delivery step labels.
7.7. `site-header.tsx`: add `{ href: "/events", key: "events" }` between
     Menu and Contact. `app/sitemap.ts`: `/events` plus each published
     event slug, ×3 locales with hreflang alternates.

## Group 8 — Messages (web)

Decisions 15, 16.

8.1. Add `Events`, `EventPage`, `EventBooking` and `BookingPage` namespaces
     to `messages/{en,nl,fr}.json`, and `SiteHeader.events`. Claude writes
     all nl/fr copy. Key copy:

     | key | en | nl | fr |
     |---|---|---|---|
     | `SiteHeader.events` | Events | Evenementen | Événements |
     | `Events.upcoming` | Upcoming | Binnenkort | À venir |
     | `Events.past` | Past events | Eerdere evenementen | Événements passés |
     | `EventPage.freeEntry` | Free entry · Paid meals | Gratis toegang · Betaalde maaltijden | Entrée libre · Repas payants |
     | `EventPage.seatsLeft` | {count, plural, one {# seat left} other {# seats left}} | {count, plural, one {Nog # plek} other {Nog # plekken}} | {count, plural, one {Plus que # place} other {Plus que # places}} |
     | `EventBooking.noMeal` | Attending without a meal | Aanwezig zonder maaltijd | Présent sans repas |
     | `EventBooking.joinWaitlist` | Join the waitlist | Op de wachtlijst | Rejoindre la liste d'attente |

8.2. `pnpm check:messages` passes.

## Group 9 — Admin Events panel (web)

Decisions 12, 13, 18, 20.

9.1. `admin-nav.tsx`: add `{ label: "Events", href: "/events", enabled: true }`
     between Enquiries and Settings.
9.2. `app/admin/(protected)/events/page.tsx`: list (Decision 18) + "New
     event".
9.3. `app/admin/(protected)/events/new/page.tsx` and
     `events/[id]/page.tsx` with tabs (`?tab=details|bookings|share|past`):
     - `components/admin-event-form.tsx`: en/nl/fr tabs for the text
       fields, `datetime-local` inputs interpreted as Europe/Amsterdam,
       capacity, max party, hold hours, meal cutoff, RSVP close, area,
       address (labelled "Private — only shown to confirmed guests"), the
       cover via `admin-image-upload.tsx` with a new `aspect` prop
       (default `1`, `16/9` here), and `components/admin-event-plates-editor.tsx`
       (modelled on `admin-price-options-editor.tsx`).
     - `components/admin-event-bookings.tsx`: the plate summary, a table
       with status badges, hold countdown, order link, Offer seats /
       Cancel (`admin-confirm-dialog.tsx`), and "Download CSV".
     - `components/admin-event-share.tsx` (client): locale picker, the URL,
       Copy, WhatsApp, `QRCodeSVG` (256px, `level="M"`, `marginSize={2}`)
       and a hidden `QRCodeCanvas` at 1024px for **Download PNG**
       (`{slug}-{locale}-qr.png`).
     - `components/admin-event-gallery-editor.tsx`: multi-upload through
       `POST /api/admin/media`, drag to reorder (buttons as the keyboard
       fallback), caption, remove; then the recap tabs.
     - Publish/Unpublish/Archive buttons and a "Preview" link to
       `/preview/en/events/{slug}`.
9.4. `admin-catering-tag.tsx` pattern → `admin-event-tag.tsx` on the
     Orders list and detail, with the event title and a booking link.
9.5. Preview: make `getEvent` send the admin cookie and request
     `?draft=1` in draft mode (`lib/preview.ts`), and have `get_event`
     allow unpublished events only for an authenticated admin session.

## Group 10 — Checks (web)

10.1. `pnpm check:messages`, `pnpm lint`, `pnpm build` in `apps/web`.
10.2. `/validate`.

## Group 11 — Staging seed

Decision 21.

11.1. `apps/api/seed-events.sql`:
      ```sql
      -- Phase 31: The Abula Experience. Idempotent: upserts by slug, never
      -- truncates, never touches bookings. Staging only for now.
      -- PLACEHOLDER: plate prices (€15), area and address until the client confirms.
      INSERT INTO events (slug, content, starts_at, ends_at, area, address, capacity,
                          max_party_size, meal_orders_close_at, payment_hold_hours,
                          cover_image_url, published_at)
      VALUES ('the-abula-experience-2026', $json$ { "en": {...}, "nl": {...}, "fr": {...} } $json$,
              '2026-12-27 14:00 Europe/Amsterdam', '2026-12-27 18:00 Europe/Amsterdam',
              'A home lunch in [city] — address shared with confirmed guests', -- PLACEHOLDER
              'Placeholder Street 1, [city]',                                  -- PLACEHOLDER
              16, 6, '2026-12-20 23:59 Europe/Amsterdam', 48,
              '/images/events/abula-experience.jpg', now())
      ON CONFLICT (slug) DO UPDATE SET content = EXCLUDED.content, …, updated_at = now();
      -- Plates: delete-and-insert only when the event has no bookings yet.
      ```
      The en content: title "The Abula Experience", summary "A home-cooked
      plate of àmàlà, ewédú and gbẹ̀gìrì and a shared table over the
      Christmas break.", a story from the client's notes, terms (meal
      orders close 20 December; cancel before then for a full refund; after
      that meals are already bought and can't be refunded; your seat is
      held for 48 hours until payment), plus nl/fr translations. Plates:
      **Abula** ("Àmàlà with ewédú and gbẹ̀gìrì") €15, and **Smoky jollof
      rice with salad** €15.
11.2. Cover image: `apps/web/public/images/events/abula-experience.jpg`
      (the existing menu photos live in `public/images/menu/`). Use the
      client's Abula photo if one is on hand, otherwise an existing menu
      photo.
11.3. `.github/workflows/staging-up.yml`, seed job: after "Seed the menu",
      add "Seed events" running `apps/api/seed-events.sql` the same way.
11.4. `.github/workflows/seed-staging-events.yml`: `workflow_dispatch`,
      `environment: staging`, `./.github/actions/staging-host` to find the
      IP, `./.github/actions/ssh-target`, then the same `psql` line. No
      confirmation phrase, because it's idempotent and staging only.
11.5. Run it on staging and check the event at
      `https://<ip>.sslip.io/en/events/the-abula-experience-2026`.

## Implementation notes

2026-11-17, Groups 1–11:

- **Validation errors are `400`, not `422`.** The API's existing
  `AppError::Validation` returns `400 validation_error` everywhere, so
  bookings follow it. Two new variants: `NotEnoughSeats` (`409
  not_enough_seats` with `seats_left`) and `BookingClosed` (`409
  booking_closed`).
- **Plate names are snapshotted in English**, not the guest's language
  (3.2 said the booking's locale). With per-locale snapshots, the admin's
  plate counts and the order lines split one plate across three names.
  Guests still see their language: the booking page and emails translate
  from the plate's `content` while the plate exists, falling back to the
  English snapshot.
- **`OrderResponse` gained `event_booking_token`**, so `/order/{token}`
  links back to the booking page (7.6). Admin order rows and details carry
  `event_id` and `event_title`.
- **The admin "Offer seats" also works on cancelled bookings** (open risk
  4), and the guest's party and plates are kept from the original booking.
- **Server fetchers live in `lib/events-api.ts`.** `lib/events.ts` holds
  the types and the client helpers (`createBooking`, `eventDateParts`),
  because the booking form imports it and `next/headers` can't reach a
  client bundle.
- **Preview:** `/events` joins the device simulator's pages, and
  `normalizePreviewPath` allows `/events/{slug}`, so the admin's Preview
  link goes through `/api/preview/enable` like the other preview links.
- **The seed's cover photo** is a copy of `public/images/menu/ogbono-soup.jpg`
  at `public/images/events/abula-experience.jpg`, a placeholder until the
  client sends an Abula photo.
- **The pot meter** is an inline SVG (`components/event-seats-pot.tsx`): a
  clipped wave rises to the share of seats taken with a GSAP scroll
  trigger, plus rising bubbles; static under reduced motion.
