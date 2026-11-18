# Phase 35 — Event Booking: Copy, Plates Required & Flexible Options: Requirements

## Source

A new phase with no earlier roadmap entry. On 2026-11-18 the user asked:

> change this from "Entry is free and seats are limited. Please RSVP to
> save your place. If you'd like to eat, choose your plate when you sign
> up. You'll get a payment request straight away, and your meal is
> reserved once payment is received." to "YOUR SEAT AT THE TABLE IS
> WAITING
> Don't just hear about it, be part of the experience!"
> - remove the attending without a meal option.
> - option group flexible. … The admin should be able to set how many
>   options the person can select. … you can have an option group with 5
>   items and the user can only select 3 items, right? Then you can have
>   an option with 4 items, but the person can only select 1 item. So we
>   need to make it multi-select, but with limits from the admin.

Answers to the planning questions (2026-11-18):

- **After the meal cutoff:** bookings close. The meal cutoff becomes the
  booking cutoff, and waitlist offers stop too.
- **Limits:** the admin sets a **min and max** per option group. The
  default is exactly 1, so existing groups behave as today.
- **Enforcement:** the API rejects a guest without a plate, not just the
  form. Existing no-meal bookings stay valid and still display.

## Context

### Why now

The Abula Experience (27 December) is a meal event: the client wants
every guest to order a plate, and wants plates with "pick up to N"
choices. The guest wording above the booking form is being replaced with
shorter, warmer copy.

### What exists

- **Phase 31.** `event_booking_guests.plate_id` is nullable: `NULL`
  means "attending without a meal". The booking form
  (`apps/web/components/event-booking-form.tsx`) offers a `NO_MEAL`
  radio per guest. A booking with no meals is confirmed at once; one
  with meals gets an invoice (`events::start_payment`). After
  `meal_orders_close_at` guests can still RSVP without a meal until
  `rsvp_closes_at` (or `starts_at`).
- **Phase 32.** `event_plates.options` JSONB holds option groups; every
  group is pick-one and required (`apps/api/src/event_options.rs`,
  `resolve_choices`). The API takes `choices` as
  `{"group": "choice"}` per guest and snapshots
  `[{"group", "choice", "label"}]` in `event_booking_guests.choices`.
  Order lines, the cook counts and the CSV group by that snapshot.
  Out of scope there: "Optional or multi-select groups".
- **Phase 34.** Tikkie and the staging → production event copy. The
  copy workflow carries `options` as-is, so the new limits travel with
  it.
- The guest wording is `Events.guestWording` in
  `apps/web/messages/{en,nl,fr}.json`, rendered on
  `apps/web/app/[locale]/events/[slug]/page.tsx` under "Save your seat".

## Decisions

1. **New guest wording.** Above the booking form, a tagline
   **"YOUR SEAT AT THE TABLE IS WAITING"** and the line
   **"Don't just hear about it, be part of the experience!"** replace
   the old paragraph. New key `Events.guestTagline`, rewritten
   `Events.guestWording`:
   - en: "YOUR SEAT AT THE TABLE IS WAITING" / "Don’t just hear about it,
     be part of the experience!"
   - nl: "JE PLEK AAN TAFEL STAAT KLAAR" / "Hoor er niet alleen over,
     maak het zelf mee!"
   - fr: "VOTRE PLACE À TABLE VOUS ATTEND" / "Ne vous contentez pas d’en
     entendre parler, vivez l’expérience !"

   The tagline is stored in capitals (as the client wrote it) and
   rendered as a bold block line; the line follows in the muted style.
   No copy approval step (standing rule).
2. **Every guest orders a plate.** The form drops the "Attending without
   a meal" radio; each guest starts on the first available plate. The
   total is always a price, so the terms checkbox and payment note
   always show.
3. **The API requires a plate.** `POST /api/events/{slug}/bookings`
   rejects a guest with `plate_id: null` (or a missing one) with
   `400 validation_error`, field `guests[i].plate_id`, message
   `required`. No migration: the column stays nullable, so old no-meal
   bookings keep loading, and the booking page, emails, admin, CSV and
   `offer_seats`/`confirm` paths keep handling `NULL` for them.
4. **Bookings close at the meal cutoff.** `rsvp_open` becomes
   `now() < LEAST(meal_orders_close_at, rsvp_closes_at, starts_at)`
   (Postgres `LEAST` skips NULLs) everywhere it's computed; `meals_open`
   equals it and stays in the API response for compatibility. The
   form's "meals closed" state goes away. The event page's cutoff line
   reads "Bookings close on {date}." The admin form's hints for both
   cutoffs say so.
5. **Waitlist offers stop at the cutoff too.** `offer_seats` refuses
   once the event's bookings have closed, with a new
   `AppError::BookingClosed`-style `409` ("Bookings for this event have
   closed") shown in the admin bookings tab. (User's choice: the owner
   no longer cooks for late guests.)
6. **Publishing needs a plate.** The admin publish action also rejects
   an event with no available plate (`400 validation_error`, "Add a
   plate before publishing"), since no one could book it.
7. **Option groups get `min` and `max`.** Each group in
   `event_plates.options` gains `"min"` and `"max"` integers, read with
   serde defaults of `1` and `1`, so existing JSON (and the seed) needs
   no change. Rules, checked by `validate_groups`:
   `0 ≤ min ≤ max`, `1 ≤ max ≤ choices.len()`. Errors on
   `options[g].limits`: `invalid`.
8. **Guests pick a set per group.** The booking API takes
   `choices: {"group": ["a", "b"]}`. A bare string is still accepted as
   a one-item list, so a cached old form keeps working.
   `resolve_choices` checks per group: known group, known choices, no
   duplicates, `min ≤ count ≤ max`. Errors: `option_required` (fewer
   than `min`), `too_many_options` (more than `max`), `unknown_option`.
   A group with `min: 0` may be left empty.
9. **Snapshot shape is unchanged.** One `GuestChoice` per picked choice,
   ordered by group then by the plate's choice order (not click order),
   so equal picks always produce equal JSON. Order lines, cook counts
   and the CSV keep grouping by the snapshot; summaries read
   "Fried plantain, Moin moin, Chicken".
10. **Booking form controls follow the limits.** Exactly-one groups
    (min 1, max 1) keep the radio pills. Every other group uses checkbox
    pills in the same style; once `max` are picked, the rest are
    disabled. The group legend states the rule:
    - min 1, max 1: "Choose your {group}" (as today)
    - min = max = n: "{group}: choose {n}"
    - min 0: "{group}: optional, up to {max}"
    - otherwise: "{group}: choose {min} to {max}" (or "up to {max}" when
      min is 1)

    Plus a live "{count} of {max} chosen" hint on multi-pick groups.
    Client-side validation mirrors the API with the existing
    `aria-invalid`/`aria-describedby` pattern.
11. **The event page plate cards state the rule** with the same wording
    before listing the choices.
12. **Admin plate editor gets Min and Max** number inputs per option
    group (in `admin-plate-options.tsx`), defaulting to 1 and 1, with
    the rule shown as a preview ("Guests choose up to 3"). Lowering the
    limits on a plate with bookings is allowed; existing snapshots keep
    what was booked.
13. **No "free" in guest-facing copy.** (User, 2026-11-18: "remove the
    its free completely".) Every guest now pays for a plate, so:
    - Event page eyebrow `Events.freeEntry` ("Free entry · Paid meals")
      is replaced by `Events.eyebrow`: en "Limited seats · Meals by
      pre-order", nl "Beperkte plekken · Maaltijden op bestelling", fr
      "Places limitées · Repas sur commande".
    - Booking email `intro_awaiting` (`apps/api/src/event_emails.rs`)
      drops the "Entry is free" clause: en "Thanks for booking a seat at
      {title}! Your meals are reserved once payment is received.", nl
      "Bedankt voor je reservering voor {title}! Je maaltijden zijn
      gereserveerd zodra je betaling binnen is.", fr "Merci d'avoir
      réservé pour {title} ! Vos repas sont réservés dès réception de
      votre paiement."
    - The seeded story's last paragraph (`apps/api/seed-events.sql`,
      en/nl/fr) becomes: en "Seats are limited. Choose your plate when
      you book: we cook to order, so every plate is reserved once it's
      paid.", nl "Er zijn maar een paar plekken. Kies je bord bij het
      reserveren: we koken op bestelling, dus elk bord is gereserveerd
      zodra het betaald is.", fr "Les places sont limitées. Choisissez
      votre assiette en réservant : nous cuisinons sur commande, chaque
      assiette est donc réservée dès qu'elle est payée."
    - `EventBooking.totalFree` goes (Decision 2).

    The privacy policy's retention line ("free event RSVPs") stays: it
    describes data that still exists from old bookings. Internal
    comments and admin-only labels ("free-confirmed") are not
    guest-facing and stay.

## Out of scope

- **Price per choice** (still Phase 32 Decision 2).
- **Picking the same choice more than once** (quantities per choice).
- **Removing the `plate_id` NULL support** from the schema, emails or
  admin: old no-meal bookings must keep working.
- **Production rollout** (standing rule: staging is the target).

## Open risks flagged during planning

1. **Old no-meal bookings.** Staging (or production, if any exist) may
   hold bookings with a NULL plate. *Mitigation:* the display and
   admin paths keep their NULL handling and tests; only creation
   rejects it.
2. **The live event's story still says "Entry is free".** The story is
   database content: the staging seed re-run (upsert mode) refreshes it,
   but production's seed never overwrites, and the owner may have
   edited it. *Mitigation:* after deploying, re-run the staging seed or
   edit the story in admin on staging, then use Phase 34's
   `copy-staging-event.yml` for production when ready.
3. **Order lines multiply.** More combinations mean more distinct
   `order_items` lines on one invoice. *Mitigation:* acceptable for
   party sizes up to `max_party_size`; lines group identical picks.
4. **Closing bookings earlier.** Events whose `rsvp_closes_at` was later
   than the meal cutoff now close sooner. *Mitigation:* the admin hints
   explain it; the owner can move the meal cutoff.
