# Phase 34 — Event Copy to Production & Tikkie Payment: Requirements

## Source

A new phase with no earlier roadmap entry. On 2026-11-18 the user said:

> the client updated the event on staging. that is exactly what they want
> on production. please help me copy the event data from the staging to
> production. next the client wants the payament tikkie added to the
> event payment flow: Please could you pay me for 'Abula Experience 2026'
> at https://tikkie.me/pay/dpmcb8p1hgl6338pqfn6
> This link is valid until 2 December
> don't use real tikkink on staging. The admin should be able to update
> the tikkie link.

Answers to the planning questions (2026-11-18):

- **Publish:** copy the event **and publish it** on production.
- **Tikkie amount:** the guest **types the amount**. One open-amount link
  serves every booking.
- **Placement:** **Tikkie first, bank transfer below** as the
  alternative.
- **Expiry:** admin stores a **"valid until" date** with the link. After
  it, guests see bank transfer only and admin shows a warning.

## Context

### Why now

The event is on 27 December and meal orders close on 20 December. The
client has finished the event on staging (real prices, address, text)
and wants to open bookings. Tikkie is how most Dutch guests pay each
other, so it lowers the barrier to paying for a meal.

### What exists

- **Phase 31/32 (merged).** `events`, `event_plates` (with `options`),
  `event_photos`, `event_bookings`. A booking with meals creates an
  order with `delivery_type = 'event'`, which follows the bank-transfer
  flow: invoice, IBAN + `ZTM-{id}` reference (`apps/api/src/payment.rs`),
  "I've paid" (`PaymentClaimButton`), owner confirms.
- **Production** has the event as a **draft**, seeded on 2026-11-17 by
  `seed-production-events.yml` (insert-only, €15 placeholders, empty
  address). Production is promoted to Phase 33 (promote run 2026-11-18).
- **Staging is ephemeral.** `staging-down.yml` deletes the instance and
  its data. `staging-up.yml` and `seed-staging-events.yml` run
  `seed-events.sql` with `upsert=true`, which **overwrites** the event's
  text, dates, settings and address with the seed values. Any of the
  three wipes the client's edits.
- Staging and production are separate GitHub Environments, each with its
  own `DEPLOY_SSH_KEY`. Staging's host comes from
  `.github/actions/staging-host`, production's from `vars.DEPLOY_HOST`.
- Uploaded images are stored in MinIO under a UUID key and saved as an
  absolute URL `{PUBLIC_API_URL}/api/media/{key}`
  (`routes/admin/media.rs`). A staging upload therefore points at
  staging's domain. The repo's own cover is a relative path,
  `/images/events/abula-experience.jpg`.
- Payment details reach the guest through `OrderResponse.payment`
  (`routes/orders.rs`, `load_order_by_token`), rendered by
  `components/order-invoice.tsx`, and through the payment block in
  `event_emails.rs`.

## Decisions

### A. Copying the event to production

1. **A new workflow, `copy-staging-event.yml`** (workflow_dispatch), with
   inputs `slug` (default `the-abula-experience-2026`) and `confirm`
   (typed phrase `COPY EVENT TO PRODUCTION`). A data copy, not a
   seed: the copy is whatever the client typed on staging, not a file in
   the repo.
2. **Three jobs.** `check` (no environment) verifies the phrase, so a
   typo never asks a reviewer. `export` (environment `staging`) reads
   the event and its images. `import` (environment `production`,
   reviewer-gated) writes them. The two Environments have different SSH
   keys, so one job can't reach both. `export` hands over a GitHub
   artifact kept for **1 day** (it holds the private address).
3. **What's copied:** the event's `content` (all locales), `starts_at`,
   `ends_at`, `area`, `address`, `capacity`, `max_party_size`,
   `meal_orders_close_at`, `rsvp_closes_at`, `payment_hold_hours` and
   `cover_image_url`; every plate (`position`, `content`, `options`,
   `price`, `is_available`); every photo (`position`, `image_url`,
   `caption`).
4. **What's never copied:** the slug (production is matched by it), the
   id, bookings, orders, `archived_at`, timestamps, and the **Tikkie
   link and date** (Decision 12). Staging only ever has a fake link.
5. **Production is matched by slug and overwritten.** Plates and photos
   are replaced outright. The event must already exist on production
   (it does, from Phase 32). A missing event fails with "Run Seed
   production events first."
6. **Refusals, all before anything is written:**
   - the event doesn't exist on staging, or staging isn't up;
   - production has **any booking** for the event (replacing plates
     would orphan guests' `plate_id`, and changing prices under a paid
     booking breaks trust);
   - the staging address is blank (production can't publish without
     one). Still having the seed's `Staging test address, Almere` is only
     a warning in the reviewer's summary: on 2026-11-18 the user said
     "don't worry too much about the address".
7. **It publishes.** `published_at = COALESCE(published_at, now())`, the
   same as the admin Publish button, whose only check (a non-blank
   address) Decision 6 already covers.
8. **Images move with the event.** For `cover_image_url` and each
   photo, a URL ending in `/api/media/{key}` is downloaded from staging,
   uploaded to production's MinIO bucket under the **same key** with its
   content type, and rewritten to production's
   `{PUBLIC_API_URL}/api/media/{key}`. Production's base is read from
   the running `api` container, not hardcoded. Relative paths (the
   repo's `/images/...` cover) are copied as they are. Keys must match
   `^[0-9a-f-]{36}\.(jpg|jpeg|png|webp)$`; anything else fails the run.
9. **Safety, same as the other production workflows:** the reviewer-
   gated `production` environment, the typed phrase, and a `pg_dump`
   backup before writing. The whole write is one transaction. Images
   are uploaded before it and use new UUID keys, so they can't overwrite
   anything.
10. **The reviewer sees what's coming before approving.** `export`
    writes a job summary: title (en), dates, area, the address, capacity,
    each plate with its price and option groups, and the image count.
    The summary is only visible to repo collaborators, and the reviewer
    has to check the address is the real one.
11. **Warn about staging.** The workflow's header comment and
    `docs/deployment.md` say: don't run Staging down, Staging up or Seed
    staging events before copying, because each one wipes or overwrites
    the client's edits.

### B. Tikkie payment for events

12. **The link lives on the event.** New columns
    `events.tikkie_url TEXT` and `events.tikkie_valid_until DATE`, both
    null or both set (a `CHECK`). Edited in the admin event form and
    saved with the event. It's per event, because a Tikkie request is
    named after what's being paid for ("Abula Experience 2026").
13. **Only real Tikkie links are accepted:**
    `^https://tikkie\.me/pay/[A-Za-z0-9_-]+$`, after trimming. Anything
    else is `400 validation_error` on `tikkie_url` (`invalid`). This
    keeps a typo or a look-alike domain away from guests. A date in the
    past is allowed (so the admin can save other fields), but admin
    warns (Decision 18).
14. **"Valid until" means the end of that day in Amsterdam.** The link
    is shown while `now() < (tikkie_valid_until + 1 day) at Europe/Amsterdam`.
    Computed in SQL, like the event's other cut-offs.
15. **The guest types the amount, so we tell them exactly what.** The
    API returns, on an event order, `tikkie: {url, valid_until, amount}`
    where `amount` is the order total. It's a new field on
    `OrderResponse`, separate from `payment` (bank details), so Tikkie
    still shows when bank details aren't configured. `null` when the
    order isn't an event order, the event has no link, the link has
    expired, or the order's status isn't `awaiting_payment` (once the
    guest presses "I've paid" or the owner confirms, there's nothing
    left to pay).
16. **Tikkie first, bank transfer below.** On the booking page and the
    order tracking page, when `tikkie` is set, the invoice's payment
    section opens with a **Pay with Tikkie** button (opens the link in a
    new tab, `rel="noopener noreferrer"`), the amount to enter
    ("Enter €45.00 in Tikkie"), and "Valid until 2 December". Below it,
    "Or pay by bank transfer" with the existing IBAN and reference. The
    "I've paid" button stays as it is, for both methods. Without
    `tikkie`, the page is unchanged.
17. **Payment emails carry it too.** The awaiting-payment and offered
    emails put a Tikkie line before the bank details while the link is
    valid when the email is sent: "Pay with Tikkie: {url} (enter
    €45.00, valid until 2 December)". Emails sent later don't include
    an expired link.
18. **Admin warns about expiry.** In the event form, next to the field:
    "Expired on 2 December: paste the client's new link" once it's past,
    "Expires in N days" within 3 days. The events list shows a badge
    ("Tikkie expired") on an upcoming, published event whose link has
    expired. No email or push: the owner is in admin daily.
19. **No reference to match on.** Tikkie payments don't carry the
    `ZTM-` reference, so the owner matches them by name and amount. The
    bookings tab already shows each booking's name and total, and the
    page asks the guest to press "I've paid", which flags the order.
    Nothing changes in how the owner confirms.
20. **Staging never gets the real link.** `seed-events.sql` sets
    `tikkie_url` only through a new psql variable `tikkie_url`, passed
    by the staging workflows as `https://tikkie.me/pay/staging-not-a-real-link`
    with a date a year ahead. Production's seed passes nothing (stays
    null) and the copy never touches it. The real link
    (`https://tikkie.me/pay/dpmcb8p1hgl6338pqfn6`, valid until
    2 December 2026) is entered by the owner in production admin, and
    appears nowhere in the repo's code, seeds or workflows (only in
    this spec, as the source quote).
21. **Copy in en/nl/fr**, written by Claude with no approval step.

## Out of scope

- **Tikkie for food orders** (the cart). Only events; the client asked
  for the event flow. The column lives on the event, so a site-wide link
  would be a separate decision.
- **Fixed-amount or per-booking Tikkie links**, and Tikkie's API
  (automatic payment confirmation). The owner still confirms by hand;
  iDEAL by webhook stays in "Later — iDEAL Payments".
- **Copying in the other direction**, other events, or bookings.
- **Running the workflows and the promote.** The user runs
  `copy-staging-event.yml`, promotes, and sets the Tikkie link in admin.
- **Reminders** to guests when the link changes.

## Open risks flagged during planning

1. **Staging wiped before the copy.** *Mitigation:* Decision 11. Until the
   copy has run, nobody runs the three staging workflows, including
   Staging up to deploy this phase (it re-seeds the event on every run).
2. **The client's address on staging is still the test one.**
   *Mitigation:* the reviewer's summary warns (Decision 6), and the
   owner fixes it in production admin after the copy.
3. **`mc` may not be in the `alpine/minio` image.** *Mitigation:*
   plan Group 0 checks it. Fallback: a pinned `alpine/mc` container on
   the Compose network.
4. **The link expires on 2 December, before meal orders close.**
   *Mitigation:* Decisions 14 and 18: guests fall back to bank transfer
   and the admin is warned to paste the client's new link.
5. **Guests send the wrong amount** in an open-amount Tikkie.
   *Mitigation:* the exact amount is on the page and in the email. The
   owner sees the shortfall when matching and contacts the guest, as
   with a short bank transfer today.
6. **The promote ships the migration after the copy.** The copy doesn't
   read or write the Tikkie columns, so it works before and after.
