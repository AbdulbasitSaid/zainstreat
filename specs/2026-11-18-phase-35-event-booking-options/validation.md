# Phase 35 — Event Booking: Copy, Plates Required & Flexible Options: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the new `event_options` limit tests and the booking,
      offer and publish tests.
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] `git diff master -- apps/api/migrations/` is empty.
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`. No new packages.
- [ ] `/validate` passes.
- [ ] `grep -rn "NO_MEAL\|noMealHint\|mealsClosed\|totalFree" apps/web`
      returns nothing.

### Guest wording

- [ ] `/en/events/{slug}` shows "YOUR SEAT AT THE TABLE IS WAITING" in
      bold on its own line, then "Don’t just hear about it, be part of
      the experience!", under "Save your seat". The old paragraph is
      gone in en, nl and fr.
- [ ] `/nl/…` and `/fr/…` show the Decision 1 translations.

### No "free" copy

- [ ] The event page eyebrow reads "Limited seats · Meals by pre-order"
      (and the nl/fr versions).
- [ ] `grep -rniE "entry is free|free entry|toegang is gratis|gratis toegang|entrée (est )?(gratuite|libre)|freeEntry|totalFree" apps/web/messages apps/web/app apps/web/components apps/api/src apps/api/seed-events.sql`
      returns nothing.
- [ ] A new booking's "awaiting payment" email (en, nl, fr) has no
      "free" clause.
- [ ] After the staging seed re-run, the staging event story ends with
      "Seats are limited. Choose your plate when you book…" in all three
      languages.

### Plates required

- [ ] The booking form has no "Attending without a meal" option; each
      guest starts on the first plate; the total is a price and the
      terms checkbox shows.
- [ ] `curl` a booking with `"plate_id": null` → `400`,
      `guests[0].plate_id` `required`.
- [ ] An existing no-meal booking (staging, or a test fixture) still
      opens on `/en/events/booking/{token}` showing "Attending without a
      meal", and in admin and the CSV.
- [ ] With `meal_orders_close_at` in the past (and `rsvp_closes_at`
      later), the event page shows "Booking for this event has closed."
      and a `curl` booking returns `booking_closed`.
- [ ] Offer seats on a waitlisted booking after the cutoff shows the
      "Bookings for this event have closed" error; before it, it works
      as before.
- [ ] Publishing an event with no available plate fails with "Add a
      plate before publishing".

### Option limits

- [ ] In admin, a group shows Min 1 / Max 1 by default and existing
      Abula plates load as 1/1 without any data change.
- [ ] Saving Min 2 / Max 1, Max 0, or Max greater than the number of
      choices shows an inline limits error; Min 0 / Max 3 on a 5-choice
      group saves and survives reload.
- [ ] Booking form, 5-choice group "up to 3" (min 1, max 3): the
      legend reads "{Group}: choose up to 3", checkboxes show, the
      fourth is disabled after three picks, "3 of 3 chosen" shows;
      submitting with none picked shows the inline error.
- [ ] A 4-choice group with 1/1 still renders radio pills and
      "Choose your {group}".
- [ ] A min-0 group can be left empty and the booking succeeds.
- [ ] `curl` with 4 picks on a max-3 group → `too_many_options`;
      fewer than min → `option_required`; a duplicate or unknown key →
      `unknown_option`; a bare string for a 1/1 group still works.
- [ ] Two guests with the same picks in different click order share one
      invoice line, e.g. "The Abula Experience — The Smoky Jollof
      Experience (Fried plantain, Moin moin, Chicken)".
- [ ] The booking page, emails, admin cook counts and CSV list every
      picked choice.
- [ ] The event page plate cards show the rule wording before the
      choices.

## Definition of done

The event page shows the new tagline and line in all three languages;
every new booking has a plate per guest, and bookings (including owner
waitlist offers) close at the meal cutoff, while old no-meal bookings
still display everywhere; the owner can set a min and max per option
group, guests get radios or limited checkboxes accordingly, and the API
enforces the same limits — all with no migration, passing tests,
`pnpm build` and `/validate` on staging.
