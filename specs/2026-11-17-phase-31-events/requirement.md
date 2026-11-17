# Phase 31 — Events: Bookings, Plates & Past Events: Requirements

## Source

A new phase with no earlier roadmap entry. The user raised it on
2026-11-17:

> The client wants us to make it possible to add events on the website.
> So we need on the admin page to have a panel where events can be
> created and people can pay for these events and get a link, or … she
> can share the link to this event via a QR code, and they can register
> for the event on that link, add the amount of people … and make the
> payment similar to the steps we do for when we order for food. … she
> wants us to create a particular event for December, so we would need to
> seed this event on both production and staging. But for now we will
> seed it on staging for test purposes. … I would need a page where they
> would see previous events on the … regular website. Then in the CMS …
> the admin [should] be able to manage events and publish previous events
> and also show upcoming events … And we must keep that soup look and feel
> for the events.

The first event's details came from the client's planning notes, pasted
in the same message:

- **The Abula Experience.** A home-cooked plate of àmàlà, ewédú and
  gbẹ̀gìrì, and a shared table to meet people over the Christmas break.
- **Sunday 27 December 2026, 14:00**, a late lunch at one sitting, for
  12–16 guests.
- **Free entry, paid food.** Two mains: **Abula** (the headline) and
  **smoky jollof rice with salad**. Guests can also attend without a
  meal, and they still count towards capacity.
- **One form for RSVP and meals:** name, contact details, how many
  people, and a plate for each person.
- **One payment per booking**, due within **48 hours** before the places
  go to the waitlist. A free RSVP is confirmed separately.
- **The address is shared privately** with confirmed guests, along with
  the date, time and plates ordered.
- **Cancellation and refund terms** are on the form before the guest
  pays. **Meal orders close on 20 December.**
- Guest wording: "Entry is free and seats are limited. Please RSVP to
  save your place. If you'd like to eat, choose your plate when you sign
  up. We'll send you a payment request, and your meal is reserved once
  payment is received."

## Context

### Why now

The event is on 27 December and meal orders close on 20 December, so
guests need to be able to book by early December at the latest.

### What exists

- **Bank-transfer orders (Phases 20, 22).** `orders` has a
  `public_token`, an invoice with the business's bank details and a
  `ZTM-{id}` reference (`payment.rs`), the customer's "I've paid"
  (`claim_payment`), the admin's "Confirm payment" (`→ processing`), a
  tracking page at `/[locale]/order/[token]`, status emails, and
  `order_items` that snapshot names and prices.
- **Quotes turn into orders (Phase 23).** `accept_quote`
  (`routes/quotes.rs`) inserts an `orders` row plus `order_items` and
  hands the customer the order's token. It added a third fulfilment type,
  `on_site`, with its own status branch in `order_status::admin_next`.
  Events follow the same pattern.
- **Customer emails** are one email in en → nl → fr (Phase 24, see
  `order_emails.rs`). Owner emails and **web push** (Phase 27,
  `push.rs`) go out for every customer action.
- **Uploads** go to MinIO through `POST /api/admin/media`
  (`admin-image-upload.tsx`, Phase 14).
- **Draft/publish (Phase 25)** is used for site content, menu items and
  categories.
- **The "soup" look:** `liquid-card.tsx`, `liquid-surface.tsx`,
  `liquid-scroll.tsx`, `boiling-slideshow.tsx`, `bubble-loader.tsx`,
  `liquid-page-transition.tsx`, the bento grid (`menu-bento-grid.tsx`)
  and `page-hero.tsx`.
- **Spam protection (Phase 15):** `tower_governor` with the
  `X-Forwarded-For` key extractor, plus a honeypot field.
- **Seeding:** `apps/api/seed.sql` truncates and reseeds the menu. The
  "staging up" workflow runs it on every new staging instance.
  `seed-production.yml` runs it on production. Workflows find staging
  through `./.github/actions/staging-host`.
- **No background jobs** run in the API today.

## Decisions

1. **Events are their own tables, and meal payments are normal orders.**
   New tables `events`, `event_plates`, `event_photos`, `event_bookings`
   and `event_booking_guests`. A booking with at least one meal creates
   an `orders` row with the new `delivery_type = 'event'` and one
   `order_items` line per plate type (for example "The Abula Experience —
   Abula" × 2). That reuses the invoice, payment reference, "I've paid",
   "Confirm payment", the owner's order list, push and emails unchanged.
   A booking with no meals has no order. *Why:* payment is "similar to
   the steps we do for food" (the user's words), and Phase 23 already
   showed that turning something into an order is the cheapest way to get
   the whole payment flow.
2. **The event order lifecycle:** `awaiting_payment` → `payment_claimed`
   → `processing` (payment confirmed, shown to guests as "Confirmed") →
   `completed` (after the event), or `cancelled`. In `admin_next`,
   `processing` with `event` goes to `completed` or `cancelled`. There's
   no ready, delivery or on-site step. The booking's status follows its
   order in the same transaction: `processing` confirms the booking, and
   `cancelled` cancels it and frees its seats.
3. **Booking statuses:** `waitlisted`, `awaiting_payment` (meals booked,
   seats held), `confirmed`, `cancelled`. `cancel_reason` is `expired`,
   `admin` or `guest`.
4. **Instant invoice** (asked during planning). The site checks capacity
   itself, so a booking that fits gets its invoice at once. The guest
   doesn't wait for the owner to "send a payment request". The client's
   "after checking availability" step is done by the software. The
   guest-facing wording is adjusted to match (Decision 16).
5. **A free RSVP is confirmed at once** if it fits. It needs no payment,
   so there's nothing for the owner to approve. The owner can still
   cancel it from admin.
6. **Capacity is counted in seats.** Seats taken = the sum of
   `party_size` over bookings that are `awaiting_payment` or
   `confirmed`. Guests without a meal take a seat too. The check runs in
   the booking transaction under `SELECT … FROM events … FOR UPDATE`, so
   two guests can't both take the last seats. Each event also has a
   `max_party_size` (default 6).
7. **Waitlist.** If a party doesn't fit, the form says how many seats are
   left and offers to join the waitlist with the same details and plates.
   A waitlisted booking holds no seats and has no order. In admin, the
   owner presses **Offer seats** on a waitlisted booking. If it fits now,
   it becomes `awaiting_payment` (creating its order and a new 48-hour
   hold) or `confirmed` (no meals), and the guest is emailed. Offering
   ignores the meal cutoff, because the owner chose to cook for them.
   There's no automatic promotion. The owner decides who gets the seats.
8. **48-hour hold, released automatically** (asked during planning). A
   booking in `awaiting_payment` gets `hold_expires_at = now() + 48h`
   (`events.payment_hold_hours`, default 48). A sweeper task in the API
   runs every 5 minutes. It cancels expired bookings whose order is
   still `awaiting_payment`, cancels that order too, and emails the guest
   and the owner (with push). A booking whose guest pressed "I've paid"
   (`payment_claimed`) is **never** auto-released, because the money may
   be on its way. The owner verifies it as usual. The sweeper is a
   `tokio::spawn`ed `tokio::time::interval` loop started in `main.rs`.
   Each run is one SQL `UPDATE … RETURNING`, so it's safe to run again
   after a restart.
9. **Meal cutoff.** `events.meal_orders_close_at` (for the Abula
   Experience, 20 December 2026 23:59 Europe/Amsterdam). After it, the
   form only offers "attending without a meal", and the API rejects
   bookings with meals (`400`). RSVPs close when the event starts
   (`starts_at`), or earlier if the owner sets `rsvp_closes_at`.
10. **The address is private.** `events.address` is never in a public
    API response for the event. The public page shows `area` instead
    (e.g. "A home in [city] — address shared with confirmed guests"). The
    address is in the booking API response and on the booking page
    **only while the booking is `confirmed`**, and in the confirmation
    email.
11. **A booking page** at `/[locale]/events/booking/[token]` (its own
    `public_token`) is the guest's link. It shows the status, the event,
    each guest's plate, the total, and either the invoice with the "I've
    paid" button (reusing `order-invoice.tsx` and
    `payment-claim-button.tsx` with the order's token), the address once
    confirmed, or the waitlist message. It has a **Cancel my booking**
    button until the meal cutoff (Decision 14). The order tracking page
    for an event order links back to it.
12. **Events are published by hand.** An event has `published_at`
    (nullable) and `archived_at`. Unpublished events are drafts, visible
    only in admin. "Upcoming" and "past" are worked out from `starts_at`
    against the current time, never stored. A past event stays published
    after it happens, and the owner adds the recap and photos to it. To
    hide one, she unpublishes it. Events don't use the Phase 25 draft
    JSONB and Changes page. A single "Published" switch is enough for one
    event at a time, and the event page has its own preview link (below).
13. **Past events: cover, recap and gallery** (asked during planning).
    Each event has a cover photo, a recap text per locale (written after
    the event), and an ordered photo gallery (`event_photos`, MinIO
    uploads with an optional caption, up to 30 per event). Past events
    with a recap or photos show the gallery on their page with a
    lightbox. Booking is closed on past events.
14. **Cancellation by the guest.** Until the meal cutoff, the guest can
    cancel from the booking page (with a confirm dialog). An unpaid
    booking just ends. For a paid one, the page and the owner email say a
    refund will be arranged by the owner, according to the event's terms.
    Refunds stay manual bank transfers, and there's no refund status in
    the database. After the cutoff, the page shows the terms and a
    WhatsApp link instead.
15. **Content per locale.** Title, summary, story, the cancellation
    terms, the recap and each plate's name and description are stored as
    JSONB `{ "en": …, "nl": …, "fr": … }`. English is required, and nl/fr
    fall back to English when empty. The admin form has en/nl/fr tabs.
    The admin stays English-only (Phase 3). Claude writes the nl/fr seed
    copy (`feedback_no_copy_approval`).
16. **Public pages in the soup style.**
    - `/[locale]/events` has a `page-hero` header, an **Upcoming** bento
      section of `liquid-card` tiles (cover, date badge, title, "x seats
      left" or "Waitlist", "Book" button), and a **Past events** grid
      below (cover, date, title, "See photos"). Empty states for both.
    - `/[locale]/events/[slug]` has a full-width cover with the liquid
      surface treatment, the date/time/area chips, the story, the plates
      as menu-style tiles with prices, a seats-left meter styled as a
      filling pot (`liquid-scroll`-style fill), and the booking form. For
      past events, the recap and gallery replace the form.
    - Reveals use the existing GSAP `reveal.tsx`, and everything respects
      reduced motion.
    - The header nav gets an **Events** link (en "Events", nl
      "Evenementen", fr "Événements") between Menu and Contact. The
      sitemap lists `/events` and every published event in all three
      locales.
    - The guest wording, adjusted for instant invoices: "Entry is free
      and seats are limited. Please RSVP to save your place. If you'd
      like to eat, choose your plate when you sign up. You'll get a
      payment request straight away, and your meal is reserved once
      payment is received."
17. **The booking form.** Name, email, phone (all required, as on the
    order form), party size (1 to `max_party_size`), then one row per
    person: "Guest 1 (you)", "Guest 2", … each with a radio choice of the
    event's plates or "Attending without a meal". A live total, the
    cancellation terms with a required "I've read the terms" checkbox
    (only shown when there's a meal), an optional note (dietary needs,
    allergies), and the honeypot. It submits through a same-origin proxy
    (`app/api/events/…`) to a rate-limited `POST
    /api/events/{slug}/bookings`. The guest's locale is stored for emails.
18. **The admin Events panel** (`/events` on `cms.`, a new nav item
    between Enquiries and Settings):
    - **List**: upcoming then past, with date, status (Draft, Published,
      Archived), seats taken/capacity, waitlist count and money received.
    - **Create/edit**: all fields above, the plates editor (name and
      description per locale, price, order, available), the cover upload
      (`admin-image-upload.tsx`, 16:9 crop rather than square), and the
      Published switch. The slug is generated from the English title and
      is editable until the first booking, then locked.
    - **Bookings tab**: each booking with its guests, plates, status, hold
      expiry and linked order. Actions: Offer seats (waitlisted), Cancel,
      and "Open order" for payment actions on the existing order page. A
      plate count summary for cooking ("Abula × 9, Jollof × 4, no meal ×
      2"), counting confirmed bookings and, separately, unpaid holds. A
      CSV export of the guest list.
    - **Share panel**: the public link (`https://{DOMAIN}/{locale}/events/{slug}`,
      with an en/nl/fr picker), Copy link, Share on WhatsApp
      (`buildWhatsAppLink`), and the **QR code** (`qrcode.react`) with a
      PNG download for printing on flyers.
    - **Past-event tab**: the recap per locale and the gallery uploader
      (multi-upload, reorder, caption, remove).
    - **Preview**: unpublished events can be viewed at their public URL by
      a logged-in admin through the Phase 25 draft-mode preview host
      (`cms.<host>/preview/{locale}/events/{slug}`).
19. **Notifications.** Owner email and push for: a new booking (with meals
    or not), a new waitlist entry, a guest cancellation and an
    auto-release. Push opens the event's bookings tab. Customer emails (en
    → nl → fr, like Phase 24): booking received with invoice, RSVP
    confirmed with address, payment confirmed with address, waitlisted,
    seats offered, released after 48 hours, and cancelled. The existing
    order status emails are **not** sent for `event` orders. The booking
    emails replace them, so guests don't get two emails per step.
20. **Order list.** Event orders show in admin Orders with an "Event" tag
    (like Phase 23's catering tag), the event title in the customer
    column, and a link to the booking. The dashboard counts include them,
    and so does the app badge (Phase 27), because they're payments to
    verify.
21. **Seeding: staging now, production later.** `apps/api/seed-events.sql`
    upserts The Abula Experience by slug (`the-abula-experience-2026`),
    published, with its two plates at **€15 each (placeholders,** asked
    during planning), capacity 16, `max_party_size` 6, the 20 December
    meal cutoff, a 48-hour hold, placeholder area and address, en/nl/fr
    copy and terms. It never truncates and never touches bookings, so
    re-running it is safe. The "staging up" workflow runs it after
    `seed.sql`, and a new `seed-staging-events.yml` (workflow_dispatch)
    runs it on a staging instance that's already up. Production seeding
    waits for the real prices and address (Out of scope).
22. **One new library**, `qrcode.react` (`tech-stack.md`, "Event QR
    Codes"). Everything else uses what's there.

## Out of scope

- **Production seeding and rollout** of The Abula Experience. It needs
  the real plate prices, area and address from the client. Running
  `seed-events.sql` on production (or creating the event in production
  admin) is a later step, per `project_production_deferred`.
- **Online payment (iDEAL)** stays the "Later — iDEAL Payments" item.
  Events use bank transfer like food orders.
- **Refund tracking.** Refunds are manual bank transfers, noted by the
  owner. No refund status or amount is stored.
- **Several sittings per event**, ticket tiers, paid entry or discount
  codes. The client suggested two sittings only for 20–30 guests, and the
  owner can create two events instead.
- **Automatic waitlist promotion.** The owner offers seats by hand
  (Decision 7).
- **Reminder emails** before the event, and calendar (`.ics`) files.
- **Check-in on the day.** The CSV guest list covers it.
- **Allergen fields per plate.** Allergies go in the booking note, and
  plate descriptions can mention allergens.
- **WhatsApp owner notifications** (Phase 28) for bookings. They'll follow
  whatever Phase 28 does for orders.

## Open risks flagged during planning

1. **Placeholder prices, area and address on staging.** The seed uses
   €15 plates and a fake address. *Mitigation:* the seed file marks them
   with a `-- PLACEHOLDER` comment, and validation checks the staging
   event only, never production.
2. **The local food rules for a home event.** The client's notes say to
   check the setup with the gemeente before taking payments. That's the
   client's job, not code, but bookings shouldn't open to the public on
   production before she has. *Mitigation:* noted for the production
   seeding step. The event stays unpublished until she says so.
3. **Seat races and the sweeper.** Two bookings for the last seats, and
   the sweeper releasing a hold at the same moment as the guest pressing
   "I've paid". *Mitigation:* booking takes the event row lock
   (Decision 6). The sweeper's `UPDATE` only matches orders still in
   `awaiting_payment`, and `claim_payment` only matches the same status,
   so whichever commits first wins, and the other changes nothing.
4. **A late transfer after auto-release.** A guest may pay without
   pressing "I've paid", after their seats were released. *Mitigation:*
   the release email says to contact the owner if they've already paid.
   The owner can re-offer seats from the cancelled booking ("Offer seats"
   also works on `cancelled` / `expired` bookings) or refund.
5. **Order statuses are mirrored in three places** (Phase 20 open risk
   4). Adding the `event` delivery type touches the CHECK, `admin_next`
   and `apps/web/lib/orders.ts`. *Mitigation:*
   `tests/order_status_schema.rs` already catches drift, and a new test
   covers `admin_next` for `event`.
6. **Email volume.** One booking can send several emails. SES is still
   in the sandbox on staging, so test addresses must be verified
   identities there.
7. **Event payments share the `ZTM-{id}` reference** with food orders.
   That's fine for matching bank transfers, but the owner sees one
   numbering for both. *Mitigation:* the invoice and the admin order list
   show the event title, so they're easy to tell apart.
