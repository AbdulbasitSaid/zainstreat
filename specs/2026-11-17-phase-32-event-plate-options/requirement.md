# Phase 32 — Event Plate Options & Production Seed: Requirements

## Source

A new phase with no earlier roadmap entry. Phase 31 left production
seeding out of scope until the client's details were in. On 2026-11-17
the user asked:

> now lets seed the production event. use this image for as cover photo
> … note it needs to be croped to feet the website. please lets get as
> much information from this photo as possible … note we are not to add
> the photo to the website.

The first photo is a phone screenshot of the client's Abula bowl, with
the viewer's back and menu buttons circled. The second is the client's
event flyer, used for its information only. The flyer says:

- **ALMERE • DECEMBER 2026.** The Abula Experience. Tagline: *"A Taste of
  Home. A Table of Connection."*
- **Our menu**, "Freshly prepared with love":
  - **01 · Signature experience: The Classic Abula.** "Traditional Amala
    served with rich Gbegiri, fresh Ewedu and flavourful stew." **Choose
    your protein:** assorted meat or fish.
  - **02 · Party favourite: The Smoky Jollof Experience.** "Rich, smoky
    Nigerian party jollof rice served with your choice of side and
    protein." **Choose one side:** fried plantain, moin moin or fresh
    salad. **Choose your protein:** chicken or turkey.
- **Free entry • Meals by pre-order.** "Limited seats • Good food • Great
  conversations."
- "Official date, prices and reservations coming soon."

Answers to the planning questions (2026-11-17):

- **Prices:** seed production as a **draft** with the €15 placeholders.
  The owner sets the real prices in admin, then publishes.
- **Protein and side choices:** add **plate options** that guests pick
  per person.
- **Address:** the owner adds it in admin. The seed sets the area to
  Almere.

## Context

### Why now

The event is on 27 December and meal orders close on 20 December. The
event needs to be on production, ready for the owner to price and
publish, by early December. The flyer adds per-plate choices that Phase
31's plates can't take.

### What exists

- **Phase 31 (merged, PR #52).** `events`, `event_plates` (`content`
  JSONB per locale, `price`, `is_available`), `event_booking_guests`
  (`plate_id`, an English `plate_name` snapshot, `unit_price`), the
  booking form with one plate radio per guest, the booking page, emails,
  the admin plate editor, plate counts and the CSV. Plate names are
  snapshotted in English and translated on read
  (`GuestLine::name_in`).
- `start_payment` (`apps/api/src/events.rs`) builds the order's
  `order_items` by grouping guests by `plate_name, unit_price`.
- `apps/api/seed-events.sql` upserts the event **published**, with a
  placeholder area and address, and overwrites content and settings on
  every run. Plates are replaced only while the event has no bookings.
- The staging workflows run that file: `staging-up.yml` and
  `seed-staging-events.yml`. `seed-production.yml` truncates and reseeds
  the menu behind the `SEED PRODUCTION` phrase and the reviewer-gated
  `production` environment, after a `pg_dump` backup.
- Production changes only through `promote.yml`. Phase 31's migration
  reaches production with the next promote, so Phase 31's migration file
  must not be edited. Phase 32 adds its own.

## Decisions

1. **Plate options are JSON on the plate.** A new
   `event_plates.options JSONB NOT NULL DEFAULT '[]'` holds option
   groups:
   ```json
   [{"key": "protein",
     "label": {"en": "Protein", "nl": "Eiwit", "fr": "Protéine"},
     "choices": [{"key": "meat", "label": {"en": "Assorted meat", "nl": "…", "fr": "…"}},
                 {"key": "fish", "label": {"en": "Fish", "nl": "…", "fr": "…"}}]}]
   ```
   The plate is edited as a whole in admin, so a separate table adds
   joins without adding anything. English labels are required and nl/fr
   fall back to English, like the rest of the event content.
2. **Choices don't change the price.** The flyer has no surcharges.
   Pricing per choice can be added later if the client asks.
3. **Every group is required, with one choice each.** Guests pick one
   protein, and on jollof one side as well. A guest attending without a
   meal has no options. The API rejects a missing, extra or unknown
   group or choice with `400 validation_error`.
4. **Guests' choices are snapshotted.** A new
   `event_booking_guests.choices JSONB NOT NULL DEFAULT '[]'` holds
   `[{"group": "protein", "choice": "fish", "label": "Fish"}]`, in group
   order, with the **English** label. As with plate names (Phase 31
   implementation notes), guests see their own language while the plate
   still has that choice, and the English snapshot otherwise.
5. **Order lines group by plate and choices.** `order_items` gets one
   line per distinct plate and choices, named
   `"The Abula Experience — The Classic Abula (Fish)"` or
   `"… — The Smoky Jollof Experience (Fried plantain, Chicken)"`, in
   English like today. The invoice, booking page and emails list each
   guest's plate with their choices.
6. **The owner cooks from the counts.** The bookings tab's plate summary
   breaks each plate down by choice combination (confirmed vs held). The
   CSV gets a `choices` column per guest.
7. **The admin plate editor gets option groups.** Under each plate: add,
   rename (per locale), reorder and remove groups, and the same for each
   group's choices. Group and choice keys are generated from the English
   label (`slugify`) when added and never change after, so snapshots and
   counts stay stable. A plate needs at least two choices per group to
   save. Removing a choice that booked guests picked is allowed: their
   snapshot keeps the English label.
8. **The booking form asks for options per guest.** When a guest picks a
   plate with options, the plate's groups appear under it as compact
   radio rows ("Protein: Assorted meat · Fish"), in the existing pill
   style. Errors use the form's existing `aria-invalid` and
   `aria-describedby` pattern. The event page's plate cards list the
   choices ("Choose your protein: assorted meat or fish").
9. **The seed matches the flyer.** Plates **The Classic Abula** (Abula
   with gbẹ̀gìrì, ewédú and stew; protein: assorted meat or fish) and
   **The Smoky Jollof Experience** (side: fried plantain, moin moin or
   fresh salad; protein: chicken or turkey), each still €15 as a
   placeholder. The area is **Almere**. The summary carries the tagline,
   and the story mentions the choices and drops "with salad". All in
   en/nl/fr.
10. **Cover photo.** The client's Abula photo, cropped to a clean 4:3
    frame without the phone's buttons and markings (1000×750, from
    x 145, y 185 of the 1320×996 screenshot). It replaces the
    placeholder at `apps/web/public/images/events/abula-experience.jpg`,
    so staging gets it too. 4:3 matches the event page cover, and the
    16:10 cards crop it lightly. The flyer is **not** added to the site.
11. **One seed file, two modes.** `seed-events.sql` reads two psql
    variables:
    - `publish`: `true` on staging (published, as today), `false` on
      production (a draft).
    - `upsert`: `true` on staging (re-runs refresh content and settings,
      as today), `false` on production (insert only; if the slug exists
      nothing changes, so a re-run never overwrites the owner's prices,
      address or text).

    The address is seeded empty on production and as a staging test
    address on staging, through a third variable, `address`.
12. **Publishing needs an address.** The admin publish action rejects an
    event whose address is blank (`400 validation_error`, "Add the
    address before publishing"). Confirmed guests can't receive an empty
    address.
13. **`seed-production-events.yml`** (workflow_dispatch) runs the seed on
    production. It reuses `seed-production.yml`'s safety measures: the
    `production` environment (reviewer-gated), a typed confirmation
    phrase (`SEED PRODUCTION EVENTS`), and a `pg_dump` backup before
    seeding. It runs `seed-events.sql` with `publish=false upsert=false
    address=''`, then prints the event's status. It fails with a clear
    message if the `events` table doesn't exist (production not yet
    promoted to Phase 31/32).
14. **Owner's steps after seeding** (in the workflow summary): open the
    event in the `cms.` admin, set the real plate prices and the
    address, check the copy, then Publish. The client's food-rules check
    with the gemeente (Phase 31 open risk 2) stays her decision before
    publishing.

## Out of scope

- **Price per choice** (for example fish costing more). Decision 2.
- **Optional or multi-select groups** ("add extra plantain"). Every
  group is pick-one and required.
- **Running the production seed and the promote.** Phase 32 adds the
  workflow. The user runs `promote.yml`, then
  `seed-production-events.yml`, when they're ready.
- **Final prices and the address.** The owner sets them in production
  admin.
- **The flyer as an image** on the site.

## Open risks flagged during planning

1. **Bookings made on staging before this phase** have no choices.
   *Mitigation:* `choices` defaults to `[]`, and every display treats an
   empty list as "no choices". Staging test bookings can also be
   cancelled.
2. **Plates replaced on staging re-seed.** The staging upsert replaces
   plates only while there are no bookings, so after a test booking the
   staging event keeps its old two plates without options.
   *Mitigation:* in upsert mode, when the event has bookings, the plate
   step updates the existing plates' `content` and `options` by
   position instead of replacing them, keeping their ids and prices.
3. **The owner forgets to set prices before publishing.** *Mitigation:*
   the workflow summary and the admin's draft banner say so. Publishing
   only checks the address, because €15 could be a real price.
4. **The promote ships Phase 31 and 32 together.** Both migrations run
   on production in one go. *Mitigation:* both are additive, and the
   seed workflow checks the tables exist first.
