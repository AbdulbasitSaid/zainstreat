# Phase 32 — Event Plate Options & Production Seed: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the new choices, publish-address and plate-count tests.
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] The new migration applies on top of Phase 31 and its down
      migration reverts it. Phase 31's migration file is unchanged
      (`git diff master -- apps/api/migrations/20261117120000_events.*`
      is empty).
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`. No new packages.
- [ ] `/validate` passes.

### Plate options

- [ ] `curl /api/events/the-abula-experience-2026?locale=nl` lists The
      Classic Abula with a protein group (2 choices) and The Smoky Jollof
      Experience with side (3) and protein (2), labels in Dutch.
- [ ] The event page's plate cards show the choices in en, nl and fr.
- [ ] In the form, picking a plate shows its groups; booking without a
      pick shows an announced error. Switching plate clears the picks.
      Everything works by keyboard.
- [ ] Booking 3 guests (Abula + fish, Abula + assorted meat, Jollof +
      fried plantain + chicken) gives a €45 invoice with three order
      lines named with their choices.
- [ ] Missing, unknown or extra choices, or choices on a no-meal guest,
      give `400`.
- [ ] The booking page and emails show each guest's choices in the
      booking's language.
- [ ] The admin bookings tab splits counts by choice ("The Classic
      Abula — Fish: 1 held"), and the CSV lists each plate with its choices.
- [ ] In admin, a plate's groups and choices can be added, renamed per
      locale, reordered and removed. A group with one choice won't save.
- [ ] Bookings made before this phase still show without choices.

### Cover photo

- [ ] `apps/web/public/images/events/abula-experience.jpg` is the
      client's Abula bowl at 1000×750, with no phone buttons or green
      markings.
- [ ] It looks right on the event page (4:3), the event card (16:10) and
      as the Open Graph image. The flyer image is nowhere in the repo.

### Seed

- [ ] On a fresh local DB, the production-mode seed
      (`publish=false upsert=false address=''`) creates the event as a
      draft with area Almere, an empty address and the two plates with
      options. It's not on `/en/events`.
- [ ] Publishing it in admin is refused until an address is set.
- [ ] Running the production-mode seed again after editing a price and
      the address in admin changes nothing.
- [ ] The staging-mode seed publishes it and refreshes content, and on
      an event with bookings updates plate content and options without
      touching prices or bookings.
- [ ] `seed-staging-events.yml` runs green on staging, and the staging
      event shows the new plates, choices and cover photo.
- [ ] `seed-production-events.yml` refuses a wrong confirmation phrase,
      and stops with "Promote Phase 32 to production first." when the
      tables or column are missing.

## Definition of done

Event plates have option groups the owner manages in admin, guests pick
a protein (and on jollof a side) per person, and the choices carry
through to the invoice, booking page, emails, cook counts and CSV in
en/nl/fr. The Abula Experience matches the client's flyer (Almere, The
Classic Abula, The Smoky Jollof Experience) with the client's own Abula
photo as cover. A reviewer-gated `seed-production-events.yml` adds it to
production as a draft without ever overwriting the owner's edits, and
an event can't be published without an address. Tests, lint, build,
`check:messages` and `/validate` pass.
