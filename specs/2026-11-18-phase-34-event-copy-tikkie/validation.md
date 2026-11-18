# Phase 34 — Event Copy to Production & Tikkie Payment: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the Tikkie URL, admin save, order `tikkie` and email
      tests.
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] The new migration applies on top of Phase 33 and its down
      migration reverts it. Earlier migration files are unchanged
      (`git diff master -- apps/api/migrations/` only adds the two
      `20261118140000_event_tikkie` files).
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`. No new packages.
- [ ] `/validate` passes.
- [ ] `grep -rn dpmcb8p1hgl6338pqfn6 . --exclude-dir=specs --exclude-dir=node_modules`
      returns nothing.

### Copy workflow

- [ ] `copy-staging-event.yml` with a wrong phrase fails in `check`,
      without asking for a reviewer.
- [ ] A slug that doesn't exist on staging fails in `export` with "No
      event with that slug on staging."
- [ ] With a blank staging address, `export` fails and nothing reaches
      production. With `Staging test address, Almere`, the summary warns
      and the copy goes ahead.
- [ ] The `export` job summary shows the title, dates, area, address,
      capacity, plates with prices and options, and image count, before
      the production approval is asked.
- [ ] The artifact is kept for 1 day.
- [ ] `import` refuses when the production event is missing ("Run Seed
      production events first.") or has any booking.
- [ ] `import` takes a `pg_dump` backup before writing, and the backup
      file is non-empty.
- [ ] After a successful run, production's event matches staging's:
      `content` (en/nl/fr), dates, area, address, capacity, max party,
      cut-offs, hold hours, plates (content, options, price,
      availability), photos. Production's `tikkie_url` and
      `tikkie_valid_until` are unchanged.
- [ ] The event is published: `https://zainstreat.com/en/events/the-abula-experience-2026`
      returns 200 and the event is on `/en/events`.
- [ ] Every image that was on staging MinIO loads from production's
      `api.` domain; a relative `/images/...` cover is unchanged.
- [ ] `docs/deployment.md` has the copy section and the staging warning;
      the three staging workflows carry the warning comment.

### Tikkie: admin

- [ ] Saving the event with `https://tikkie.me/pay/abc123` and a date
      stores both; the form shows them after reload.
- [ ] `http://tikkie.me/pay/x`, `https://tikkie.me.evil.com/pay/x`,
      `https://tikkie.me/pay/x?y=1` and `https://example.com` are
      rejected with an inline error on the link.
- [ ] A link without a date, or a date without a link, is rejected on
      the missing field. Clearing both saves and removes Tikkie.
- [ ] A past date shows "Expired on …", a date within 3 days shows
      "Expires in N days", and the events list shows the "Tikkie
      expired" badge on that published event.

### Tikkie: guests

- [ ] Booking 3 meals at €15 shows, on the booking page, **Pay with
      Tikkie** first, "Enter €45.00 in Tikkie", "Link valid until …",
      then "Or pay by bank transfer" with the IBAN and `ZTM-` reference.
      The button opens the link in a new tab.
- [ ] `/order/{token}` for the same order shows the same.
- [ ] The awaiting-payment email shows the Tikkie line before the bank
      details, in the booking's language (en, nl and fr checked).
- [ ] With the date set to yesterday, the page and a new email show bank
      transfer only. With the date set to today, Tikkie still shows.
- [ ] After "I've paid", Tikkie is gone from the page.
- [ ] A cart (non-event) order's API response has `"tikkie": null` and
      its invoice is unchanged.
- [ ] With no bank details configured, Tikkie still shows, above the
      WhatsApp fallback.
- [ ] Keyboard and screen reader: the Tikkie button is a link with a
      clear name and a visible focus ring.

### Staging

- [ ] After `seed-staging-events.yml` (run only after the production copy
      is done), the staging event's link is
      `https://tikkie.me/pay/staging-not-a-real-link`, never the real one.
- [ ] `seed-production-events.yml` leaves `tikkie_url` null on a new
      event.

## Definition of done

The client's finished event has been copied from staging to production
by a reviewer-gated workflow that backs up first, carries its images
across, refuses bookings or a test address, never copies the Tikkie
link, and publishes it. Events have an admin-editable open-amount Tikkie
link with a "valid until" date: while it's valid, guests see **Pay with
Tikkie** with the exact amount first and bank transfer below, on the
booking page, tracking page and payment emails, in en/nl/fr. Expired or
unset, only bank transfer shows and admin warns. Staging only ever has a
fake link, and the real one lives only in production admin. Tests, lint,
build, `check:messages` and `/validate` pass.
