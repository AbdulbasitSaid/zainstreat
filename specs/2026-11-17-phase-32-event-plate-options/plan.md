# Phase 32 — Event Plate Options & Production Seed: Plan

Decisions refer to `requirement.md`.

## Group 0 — Preconditions

0.1. Branch `2026-11-17-phase-32-event-plate-options` from `master`
     (Phase 31 merged in PR #52).
0.2. Local stack up (`docker compose up -d`). Phase 31's migration is
     applied.

## Group 1 — Migration (API)

Decisions 1 and 4. A new migration; Phase 31's file stays untouched.

1.1. `apps/api/migrations/20261117180000_event_plate_options.up.sql`:
     ```sql
     -- Phase 32: per-plate option groups (protein, side) and each guest's
     -- choices. Choices don't change the price (Decision 2).
     ALTER TABLE event_plates
         ADD COLUMN options JSONB NOT NULL DEFAULT '[]'
             CHECK (jsonb_typeof(options) = 'array');
     -- [{"group", "choice", "label"}], label in English (Decision 4).
     ALTER TABLE event_booking_guests
         ADD COLUMN choices JSONB NOT NULL DEFAULT '[]'
             CHECK (jsonb_typeof(choices) = 'array');
     ```
1.2. `.down.sql` drops both columns.

## Group 2 — Domain types and validation (API)

2.1. `apps/api/src/events.rs`: serde types
     ```rust
     pub struct OptionGroup { pub key: String, pub label: LocalizedText, pub choices: Vec<OptionChoice> }
     pub struct OptionChoice { pub key: String, pub label: LocalizedText }
     /// A guest's pick, snapshotted with its English label (Decision 4).
     pub struct GuestChoice { pub group: String, pub choice: String, pub label: String }
     ```
     `LocalizedText` is `{en: String, nl: Option<String>, fr: Option<String>}`
     with `in_locale(locale)` falling back to `en`.
2.2. `fn resolve_choices(groups: &[OptionGroup], picked: &BTreeMap<String, String>) -> Result<Vec<GuestChoice>, AppError>`:
     one pick per group, in group order; a missing, extra or unknown
     key is `AppError::Validation` (Decision 3). Unit tests for each case.
2.3. `fn choices_summary(choices: &[GuestChoice], groups: Option<&[OptionGroup]>, locale) -> String`:
     "Fried plantain, Chicken", translated from the plate's groups when
     the choice still exists, else the English label.
2.4. `validate_option_groups(groups)` for admin saves (Decision 7):
     non-empty English labels, unique keys, at least two choices per
     group. Keys match `^[a-z0-9]+(-[a-z0-9]+)*$`.

## Group 3 — Public endpoints (API)

3.1. `routes/events.rs`: `PublicPlate` gains
     `options: Vec<PublicOptionGroup>` (`key`, `label`, `choices: [{key, label}]`,
     localized).
3.2. `CreateBookingRequest`: each guest becomes
     `{plate_id: Option<i64>, choices: BTreeMap<String, String>}`.
     For one release, also accept the old bare `Option<i64>` (an untagged
     enum), so a cached page can still book a plate without options.
     A guest with no plate must send no choices.
3.3. Insert `choices` into `event_booking_guests`.
3.4. `start_payment` groups by `plate_name, unit_price, choices` and
     names the line `"{title} — {plate} ({summary})"` when there are
     choices (Decision 5).
3.5. `PublicBooking`'s guests gain `choices: String` (the summary, in
     the booking's locale), and `load_booking` reads `choices` and the
     plate's `options`.

## Group 4 — Emails (API)

4.1. `event_emails.rs`: `GuestLine` gains `choices: Vec<GuestChoice>` and
     the plate's `options`; the guest list in every email shows
     "The Classic Abula — Fish". Tests: choices appear translated in nl
     and fr, and as English when the choice was removed.

## Group 5 — Admin endpoints (API)

5.1. `routes/admin/events.rs`: plates in the admin get/save carry
     `options`. Save runs `validate_option_groups`. Keys of existing
     groups and choices are kept as sent; new ones without a key get
     `slugify(label.en)`, made unique within the plate.
5.2. `list_bookings`: `plate_counts` become
     `{plate, choices, confirmed, held}` per plate and choice
     combination (Decision 6), and each guest row carries its summary.
5.3. `bookings_csv`: a `choices` column after `plate`, through
     `csv_field`.
5.4. `publish_event`: `400 validation_error` "Add the address before
     publishing" when `trim(address) = ''` (Decision 12). Test.
5.5. `cargo sqlx prepare --workspace -- --tests`.

## Group 6 — Tests (API)

6.1. `apps/api/tests/events.rs`:
     - booking with choices creates order lines per combination
       ("… — The Classic Abula (Fish)" ×1, "(Assorted meat)" ×2);
     - missing, unknown and extra choices give `400`;
     - choices on a no-meal guest give `400`;
     - the booking page shows the nl summary for an nl booking;
     - the old request shape still books a plate without options;
     - admin save rejects a group with one choice;
     - publish with a blank address gives `400`;
     - plate counts split by choices.
6.2. `cargo test`, `cargo clippy -- -D warnings`.

## Group 7 — Web: public

7.1. `lib/events.ts`: `PublicPlate.options`, `BookingGuest.choices`,
     the new request shape.
7.2. `components/event-booking-form.tsx`: per guest, after the plate
     radios, the chosen plate's groups as radio rows (pill style, a
     `fieldset` + `legend` per group). Changing plate clears that
     guest's choices. Client-side check that every group is picked,
     with the existing error pattern. Send `{plate_id, choices}`.
7.3. Event page plate cards: list each group, "Choose your protein:
     assorted meat or fish".
7.4. Booking page, confirmation: each guest's plate followed by the
     summary.
7.5. Messages en/nl/fr: `EventBooking.chooseOption` ("Choose your
     {group}"), `EventBooking.optionRequired`, `EventPage.chooseGroup`.
     Branches start with `#` where plural (check:messages).

## Group 8 — Web: admin

8.1. `lib/admin-events.ts`: `OptionGroupInput`, `newOptionGroup()`,
     `toInput` maps options.
8.2. `components/admin-event-form.tsx`: under each plate, an "Options"
     block: add group, label per locale (same locale tabs as the plate
     name), choices list with add, rename, reorder and remove, and
     remove group. Inline errors from the API's validation message.
8.3. `components/admin-event-bookings.tsx`: plate summary rows per
     combination ("The Classic Abula — Fish: 3 confirmed, 1 held"), and
     the guest list shows each guest's summary.
8.4. Publish button: shows the API's address error. The draft banner
     says "Set the real prices and the address, then publish."

## Group 9 — Cover photo

Decision 10.

9.1. Crop the client's screenshot to 1000×750 from x 145, y 185
     (`sips -c 750 1000 --cropOffset 185 145`), removing the phone's
     buttons and the green markings. Save over
     `apps/web/public/images/events/abula-experience.jpg`.
9.2. Drop the "PLACEHOLDER photo" comment from `seed-events.sql`.
     Phase 31's implementation notes stay as they are, as history.

## Group 10 — Seed file

Decisions 9 and 11.

10.1. `apps/api/seed-events.sql` reads psql variables with defaults
      kept in the workflows, not the file:
      ```sql
      -- psql -v publish=true -v upsert=true -v address='…' (staging)
      -- psql -v publish=false -v upsert=false -v address='' (production)
      ```
      - `published_at`: `CASE WHEN :'publish' = 'true' THEN now() END`.
      - `ON CONFLICT (slug) DO UPDATE … WHERE :'upsert' = 'true'`, so
        production re-runs change nothing.
      - Area `'Almere'`; address `:'address'`.
10.2. Content per Decision 9 (en/nl/fr): summary with the tagline
      "A taste of home, a table of connection.", the story naming both
      plates and the choices, terms unchanged.
10.3. Plates with options:
      - **The Classic Abula** — "Àmàlà with rich gbẹ̀gìrì, fresh ewédú
        and a flavourful stew." Protein: assorted meat, fish. €15
        (PLACEHOLDER).
      - **The Smoky Jollof Experience** — "Rich, smoky Nigerian party
        jollof rice with your choice of side and protein." Side: fried
        plantain, moin moin, fresh salad. Protein: chicken, turkey. €15
        (PLACEHOLDER).
10.4. Plate step: insert when the event has no plates; in upsert mode,
      replace them while there are no bookings, else update `content`
      and `options` by position (open risk 2). In insert-only mode,
      plates are only inserted for a newly created event.

## Group 11 — Workflows

11.1. `staging-up.yml` and `seed-staging-events.yml`: pass
      `-v publish=true -v upsert=true -v address='Staging test address, Almere'`.
11.2. New `.github/workflows/seed-production-events.yml` (Decision 13):
      ```yaml
      on:
        workflow_dispatch:
          inputs:
            confirm:
              description: 'Type SEED PRODUCTION EVENTS to confirm. Adds The Abula Experience as a draft; never overwrites it.'
              required: true
      jobs:
        seed:
          environment: production
          steps:
            - verify phrase
            - checkout; ssh-target with vars.DEPLOY_HOST / secrets.DEPLOY_SSH_KEY
            - check: psql -tAc "SELECT to_regclass('events') IS NOT NULL
                       AND EXISTS (SELECT 1 FROM information_schema.columns
                                   WHERE table_name = 'event_plates' AND column_name = 'options')"
              → fail with "Promote Phase 32 to production first."
            - pg_dump backup, as in seed-production.yml
            - psql -v ON_ERROR_STOP=1 -v publish=false -v upsert=false -v address='' < seed-events.sql
            - print slug, published_at, plates and prices
            - summary: the owner's next steps (Decision 14)
      ```

## Group 12 — Checks

12.1. `pnpm check:messages`, `pnpm lint`, `pnpm build` in `apps/web`.
12.2. Locally: run the seed in production mode on a fresh DB (draft,
      empty address, publish refused), then again (no change), then in
      staging mode (published, refreshed).
12.3. `/validate`.

## Implementation notes

2026-11-17, Groups 1–12:

- **Option logic lives in `apps/api/src/event_options.rs`** (types,
  `resolve_choices`, `choices_summary`, `normalize_groups`,
  `validate_groups`, with unit tests), not in `events.rs`.
- **The CSV puts choices inside the Plates column**
  (`Abula (Fish); Jollof (Moin moin, Turkey)`) instead of a separate
  `choices` column (5.3). The CSV has one row per booking, so a separate
  column couldn't line up with the guests' plates.
- **Error codes:** a missing pick is `option_required`; an unknown,
  extra or no-meal pick is `unknown_option`, both on
  `guests[i].choices`.
- **Group labels can differ per plate and language.** The seed's Dutch
  labels read naturally after "Kies je": "vlees of vis" on Abula and
  "vlees" on jollof. The page lowercases group labels after "Choose your".
- **The seed defaults to the safe production mode** when the psql
  variables are missing, and records whether the event was created in a
  temp table (`xmax = 0`), so insert-only runs never touch existing plates.
- **The admin draft banner** on the event page lists what to check before
  publishing and says when the address is still empty.
