# Phase 35 — Event Booking: Copy, Plates Required & Flexible Options: Plan

No migration, no new packages. Decisions refer to `requirement.md`.

## Group 0 — Guest wording (Decision 1)

- `apps/web/messages/{en,nl,fr}.json`, `Events`: add `guestTagline`
  above `guestWording` and replace `guestWording` with the texts in
  Decision 1.
- `apps/web/app/[locale]/events/[slug]/page.tsx`, under the
  `bookHeading` h2:

  ```tsx
  <p className="m-0 text-lg text-text-muted">
    <strong className="block font-semibold tracking-wide text-text">{t("guestTagline")}</strong>
    {t("guestWording")}
  </p>
  ```
- `specs/2026-11-17-phase-31-events/requirement.md` is history: leave it.

## Group 1 — Option limits in the API (Decisions 7–9)

`apps/api/src/event_options.rs`:

- `OptionGroup` gains
  ```rust
  #[serde(default = "one")] pub min: u8,
  #[serde(default = "one")] pub max: u8,
  ```
  (`fn one() -> u8 { 1 }`). `normalize_groups` keeps them as sent.
- `validate_groups`: push `(format!("options[{g}].limits"), "invalid")`
  unless `min <= max && max >= 1 && usize::from(max) <= choices.len()`.
- New input type for a guest's picks:
  ```rust
  #[derive(Deserialize)]
  #[serde(untagged)]
  pub enum Picked { One(String), Many(Vec<String>) }
  ```
  with `fn keys(&self) -> &[String]`-style access.
- `resolve_choices(groups, &BTreeMap<String, Picked>)`: unknown group →
  `unknown_option`; per group collect picks, reject unknown or duplicate
  keys (`unknown_option`), `count < min` → `option_required`,
  `count > max` → `too_many_options`; emit `GuestChoice`s in the plate's
  choice order, groups in plate order. A missing group counts as 0.
- Update the module doc comment (no longer "every group is pick-one").
- Tests: keep the existing ones (now with `Picked::One`), add: up-to-3
  of 5 accepts 1–3 and rejects 4; exactly-2 rejects 1 and 3; min 0
  accepts empty; duplicates rejected; output order follows the plate
  regardless of input order; `validate_groups` rejects min > max,
  max 0, max > choices; defaults parse to 1/1 from Phase 32 JSON.

`apps/api/src/routes/events.rs`:

- `GuestInput::WithChoices.choices: BTreeMap<String, Picked>`; adjust
  `choices()` and `PlateOnly`.
- Expose `min`/`max` on the public option group (`PublicOptionGroup`).
- Map `too_many_options` like the other field errors.

`apps/api/src/routes/admin/events.rs`: the admin plate payload carries
`min`/`max` through (it reuses `OptionGroup`); check the admin
read/write round-trip test.

## Group 2 — Plates required, bookings close at meal cutoff (Decisions 2–6)

`apps/api/src/routes/events.rs`:

- In `create_booking`'s guest loop, `None => errors.push(field_error(&format!("guests[{index}].plate_id"), "required"))`.
  Drop the `wants_meals && !meals_open` branch (unreachable now) but
  keep the booking-closed check.
- Every `rsvp_open` SQL expression (public list, event detail, booking
  create) becomes
  `now() < LEAST(e.meal_orders_close_at, e.rsvp_closes_at, e.starts_at)`;
  `meals_open` uses the same expression.

`apps/api/src/routes/admin/events.rs`:

- `offer_seats`: load the same `rsvp_open` expression with the event
  row; if false return a `409` (add `AppError::BookingClosed` in
  `apps/api/src/error.rs` if no fitting variant exists, mapped to
  `{"error": "booking_closed"}`). Update its doc comment ("Ignores the
  meal cutoff" no longer holds).
- `set_flags` publish path: besides the address check, reject when the
  event has no `is_available` plate, field `plates`, message
  "Add a plate before publishing".

Regenerate `apps/api/.sqlx/` (`cargo sqlx prepare`).

Tests: a booking with a null plate → 400 `guests[0].plate_id`
`required`; after `meal_orders_close_at` a booking → `booking_closed`;
`offer_seats` after the cutoff → 409; publish without plates → 400.
Existing no-meal display tests (`event_emails.rs`, booking page) stay.

## Group 3 — Booking form (Decisions 2, 4, 10)

`apps/web/lib/events.ts`: `EventPlate.options[]` groups gain
`min: number; max: number`; the booking request's
`choices: Record<string, string[]>`; `plate_id: number`.

`apps/web/components/event-booking-form.tsx`:

- Remove `NO_MEAL`, the no-meal radio, `mealsOpen` prop and the
  disabled-plate state, `wantsMeals` (terms/payment note always shown,
  total always a price). Default plate: first plate.
- `GuestState.picks: Record<string, string[]>`.
- `toggle(index, group, choice)`: radio groups (min 1, max 1) replace
  the list; others add/remove, ignoring adds beyond `max`.
- `unpicked()` → per-group problems: `count < min`. Error text
  `errors.option_required` for exactly-one groups, else
  `errors.option_range` with min/max.
- Render: radio pills as today for 1/1; checkbox pills (same classes,
  `type="checkbox"`, `disabled` when at max and unchecked, with a muted
  disabled style) otherwise, plus `t("pickedCount", { count, max })`.
- Legend via a small helper `ruleLabel(t, group)` implementing
  Decision 10; share it with the event page (put it in
  `apps/web/lib/events.ts` taking the translator, or a tiny
  `option-rule.ts`).
- `errorKey`: add `too_many_options`.

`apps/web/app/[locale]/events/[slug]/page.tsx`: stop passing
`mealsOpen`; plate cards use the rule label (Decision 11); the cutoff
line uses `bookingCutoff` and shows whenever bookings are open.

Messages (`en`, `nl`, `fr`), `EventBooking`: remove `noMealHint`, `mealsClosed`, `totalFree`, `errors.meals_closed`; add
`chooseExactly` ("{group}: choose {count}"), `chooseUpTo`
("{group}: choose up to {max}"), `chooseRange`
("{group}: choose {min} to {max}"), `chooseOptional`
("{group}: optional, up to {max}"), `pickedCount`
("{count} of {max} chosen"), `errors.option_range`
("Please choose {min} to {max} for {group}." / an exactly-n variant if
min = max), `errors.too_many_options`. `Events`: `mealCutoff` →
`bookingCutoff` ("Bookings close on {date}."), plus the matching
`chooseGroup` variants for the cards. The booking page's
`EventBooking.noMeal` stays: the booking page
(`app/[locale]/events/booking/[token]/page.tsx:139`) still uses it for
old no-meal bookings. Run `pnpm check:messages`.

## Group 4 — Admin (Decisions 4, 5, 12)

- `apps/web/lib/admin-events.ts`: `OptionGroupInput` gains `min`,
  `max`; new groups default to `1`/`1`.
- `apps/web/components/admin-plate-options.tsx`: per group, two small
  number inputs "Min" and "Max" (`min={0}`, `max={choices.length}`),
  a preview line using the rule label in English, and the
  `options[g].limits` error inline.
- `apps/web/components/admin-event-form.tsx`: the hint at line ~235
  becomes "Bookings close at the earlier of this and the meal-order
  cutoff."; the meal-cutoff hint says "Bookings close then too.";
  line ~265 drops "Guests without a meal take a seat too."; line ~322
  becomes "Guests pick one per person. The first plate is the headline."
- `apps/web/components/admin-event-bookings.tsx`: show the
  `booking_closed` error from Offer seats.

## Group 5 — Remove "free" from guest-facing copy (Decision 13)

- `apps/web/messages/{en,nl,fr}.json`: rename `Events.freeEntry` to
  `Events.eyebrow` with the Decision 13 texts; update
  `apps/web/app/[locale]/events/[slug]/page.tsx:74` to `t("eyebrow")`.
  `grep -rn freeEntry apps/web` must return nothing afterwards.
- `apps/api/src/event_emails.rs`: the three `intro_awaiting` strings per
  Decision 13; update any test asserting the old text.
- `apps/api/seed-events.sql`: replace the last story paragraph in en,
  nl and fr per Decision 13. Min/max need no seed change (defaults).
- Re-run the staging seed (`seed-staging-events.yml`) or edit the story
  in staging admin so the live staging event loses the old paragraph.

## Group 6 — Docs and roadmap

- `specs/roadmap.md`: Phase 35 entry (done in planning).
