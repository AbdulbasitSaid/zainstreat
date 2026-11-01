# Phase 23 — Catering Quotations: Validation

Checked on 2026-11-01 unless noted. Where the implementation deviated from
`plan.md`, see its "Implementation notes" section.

## Pass/fail checklist

### Preconditions (Group 0)

- [x] Phase 22 is on `master`, and this branch is based on it: `ls
      apps/api/migrations/*order_tracking*` exists.
- [ ] Staging's API logs show no `email sending disabled` warning.
      *(Staging check, after merge.)*
- [x] The client approved `copy.md` (the date is noted at its top)
      before any customer strings were written. *(The project owner
      approved all copy, Dutch included, on 2026-11-01, after
      implementation. The strings already matched `copy.md`, so nothing
      changed.)*

### Migration (Group 1)

- [x] `sqlx migrate run`, then `sqlx migrate revert`, then `sqlx migrate
      run` succeeds on a fresh dev DB.
- [x] Existing `catering_enquiries` rows read `locale = 'en'`.
      `INSERT … locale = 'de'` fails on `catering_enquiries_locale_check`.
- [x] Inserting a `catering_quotes` row with `status = 'accepted'` and
      `order_id = NULL` fails on `catering_quotes_order_when_accepted`.
      A second quote for the same enquiry fails on the `UNIQUE`.
- [x] `SQLX_OFFLINE=true cargo build --release` succeeds.
- [x] `\d orders` shows `orders_delivery_type_check` allowing `on_site`
      and `orders_status_check` allowing `serving_on_site`. Existing
      orders are untouched.

### On-site lifecycle (Group 2b)

- [x] `cargo test order_status` passes:
      - `processing` + `on_site` → `serving_on_site`/`cancelled` only
      - `serving_on_site` → `completed`/`cancelled`
      - a customer-completed on-site order reopens to `serving_on_site`
      - `customer_can_complete("serving_on_site")` is true
- [x] `cargo test --test order_status_schema` passes with 8 statuses.
- [x] `cargo test --test admin_orders` passes the on-site walk:
      `ready_at` is set on `serving_on_site`.
- [x] `cargo test order_emails` covers `serving_on_site` in `en`/`nl`,
      and the invoice's "Served at" line for `on_site`.
- [x] `cargo test --test order_received` passes: `POST …/received` on a
      `serving_on_site` order → `200 completed` with `completed_by =
      'customer'`.
- [x] `cargo test --test order_submission` passes: a cart order with
      `delivery_type: "on_site"` is rejected and inserts nothing.
- [x] Live (dev stack): an accepted on-site order walked processing →
      serving_on_site (`ready_at` set, next `completed`/`cancelled`), then
      the customer's `received` → `completed`, `completed_by = customer`.

### Quote rules (Group 2)

- [x] `cargo test quote_status` passes: every status × expired/not for
      `customer_can_accept`, `customer_can_decline`, `admin_can_edit` and
      `display_status`.
- [x] `cargo test --test quote_status_schema` passes.

### Admin API (Group 3)

- [x] `cargo test --test admin_quotes` passes:
      - save, and save again with lines replaced
      - every validation case from plan 3.1 (`400 validation_error` with
        the named field)
      - send, then save → `draft`
      - `version` goes up on every save
      - send returns `email_queued: false` with SES unset
      - `404` for an unknown enquiry or a missing quote
      - `401` unauthenticated
- [x] `GET /api/admin/catering-enquiries/{id}` includes `locale` and
      `quote`. The list includes `quote_status`.
- [x] `GET /api/admin/orders/{id}` for an order from a quote has
      `catering_enquiry_id` set. For a cart order it's `null`.

### Public API (Group 4)

- [x] `cargo test --test quote_acceptance` passes:
      - `GET` by token never returns the email or phone
      - accepting a draft → `409`
      - a stale `version` on accept or decline → `409 quote_changed`,
        and no order is created
      - accepting an already-accepted quote with an old `version` →
        `200` with the same order
      - accept creates exactly one `awaiting_payment` order with the
        quote's lines, `delivery_fee = 0`, the enquiry's locale and
        `menu_item_id IS NULL`
      - repeat or concurrent accept → the same `order_token`, one order
      - expired → `409 quote_expired`
      - decline is idempotent; accepting after a decline → `409`
      - an accepted quote can't be declined or saved (`409`)
      - the created order completes through the Phase 22 flow
      - `429` when rate-limited
- [x] `curl -s -o /dev/null -w '%{http_code}'
      localhost:8080/api/quotes/by-token/not-a-uuid` → `404`.

### Emails and locale (Group 5)

- [x] `cargo test order_emails` passes, including `quote_email` in `en`
      and `nl`, with the full `en` text pinned to `copy.md` §1.
- [x] Every `Quote.*` and on-site `Order.*` string in
      `messages/{en,nl}.json` matches `copy.md` §2–3.
- [x] `cargo test --test catering_enquiries` passes: `nl` stored, missing
      → `en`, `de` → `400` with `field: "locale"`.
- [ ] With SES configured, on staging: *(Staging check; locally SES is
      off and `send` returned `email_queued: false`.)*
      - send → one customer email in the enquiry's language, linking to
        `/{locale}/quote/{token}`
      - accept → the customer's invoice email plus the owner's "Quote
        accepted" email
      - decline → the owner's email with the reason
      *(Yours to confirm.)*

### Web (Groups 6–9)

- [x] `pnpm lint` and `pnpm build` in `apps/web` pass, with no `any`.
- [x] `curl -s localhost:3000/en/quote/<token>` → `200`, containing
      `noindex` and the line table.
      `curl -s -o /dev/null -w '%{http_code}' localhost:3000/en/quote/not-a-token`
      → `404`.
- [x] `/nl/quote/<token>` is fully Dutch.
- [x] Submitting the catering form on `/nl/contact` stores `locale = 'nl'`.
      *(Checked through the `/api/catering-enquiries` proxy with `locale:
      "nl"`, plus a missing locale → `en`. The form passes `useLocale()`.)*
- [x] Admin enquiry detail:
      - with no quote, the editor shows one empty line, the address
        prefilled from the location, and the valid-until default
      - the live total matches the API total after saving
      - Send is disabled while there are unsaved edits and needs two
        clicks, and its confirmation shows the total
      - a total of €10,000 or more shows the "Large total" warning but
        can still be saved
      - after sending, the share panel shows the quote link, Copy link
        works, and "Send via WhatsApp" opens `wa.me/31…` with the
        `copy.md` §4 message in the enquiry's language
      - an unusable phone number hides the WhatsApp button
      - with email off (local dev, no SES), sending shows "Email is
        switched off. Share the link…"
      *(Browser check, 2026-11-01: all of the above in Chrome on the dev
      stack. The live total read €10.250,00 for 60 × 12,50 + 9.500, and
      the API returned the same. The confirm button showed "Send quote for
      €10.250,00 to …", and "Copy link" turned into "Copied". The
      WhatsApp link was `wa.me/31612345678` with the Dutch §4 message.
      `toWhatsAppNumber` was checked directly: `06…`, `+31…` and `0031…`
      give `316…`, `+44…` stays `44…`, and `12345`/`n/a` give `null`,
      which hides the button. Found and fixed: the button style uppercased
      the email in the confirmation.)*
- [x] Stale tab: with the quote page open, revise and resend the quote
      in admin, then press Accept on the old tab. It shows "This quote
      has changed", and no order is created. *(Browser check, 2026-11-01.)*
- [x] The quote page in each state shows the right content: sent shows
      the buttons, draft shows "being updated", expired shows a contact
      link, declined shows its message, accepted links to the order.
      *(Sent, draft and expired were checked in the browser; expired has no
      Accept button. Declined and accepted were checked by curl. Found and
      fixed: the sent-state line said "below" while sitting under the
      table; it's now "above" / "hierboven" in `copy.md` and both message
      files.)*
- [x] Accepting on the quote page lands on `/{locale}/order/{token}` with
      the invoice and bank details. The admin badge counts the new order
      within about 30s, and the order shows the "Catering" tag linking
      back to the enquiry. *(Browser check: the background dashboard tab's
      title read "(1) Dashboard" right after the accept. The bank details
      showed the WhatsApp fallback, because dev's `BANK_IBAN` is invalid.)*
- [x] The catering enquiry list shows a Quote column with the badges.
- [x] The quote editor defaults to On-site for an enquiry whose
      services include `catering`, and requires an address for it.
      *(Also checked: Delivery for `meal_delivery`, and valid-until falls
      back to the event date when it's sooner than 14 days.)*
- [x] An accepted on-site order:
      - its tracking page stepper reads "… → Catering in progress →
        Completed" (`nl`: "Catering bezig")
      - its invoice shows "Served at: {venue}"
      - admin offers "Mark catering started", then "Complete"
      - in `serving_on_site`, the customer sees "Our event has been
        catered", and two clicks complete the order
      *(Browser check: "Confirm payment" → "Mark catering started" →
      the customer's "Our event has been catered" → "Yes, all done". All
      five steps were ticked with times.)*
- [x] The cart checkout still offers only Pickup and Delivery.

### Sweep (Group 10)

- [x] `cargo test` (all), `cargo clippy` and `/validate` pass. *(194
      passed, 0 failed, up from 159. Clippy clean. Web lint/build pass.
      Live smoke test on the dev stack passed; the smoke data was deleted
      afterwards.)*
- [ ] Staging walk-through from plan 10.2 is done. *(Yours to confirm.)*
- [x] The client has reviewed the quote email, the quote page and the
      on-site tracking page and status email in `en` and `nl`, and their
      wording changes are applied. *(Covered by the owner's approval of
      `copy.md` on 2026-11-01; no separate client review needed.)*

## Definition of done

An owner can open a catering enquiry in admin, build a quotation from
free-text lines with a valid-until date and pickup or delivery, and
email it to the customer in the language they enquired in, or share the
same link on WhatsApp when email is off. The wording is what the client
approved in `copy.md`. The customer
opens a private link, sees the quote, and accepts or declines it. Editing
a sent quote pulls it back to draft until it's resent, a page showing an
older version can't accept the newer one, and an expired quote can't be
accepted. Accepting creates a normal order, once, even
with repeated clicks, and from there the customer gets the usual invoice
with bank details, and the order follows the Phase 20/22 lifecycle:
I've paid, payment confirmed, then ready for collection, on the way, or
(for catering served at the venue) catering in progress, then completed,
with the tracking page and status emails. Only quotes can create on-site
orders. The owner is emailed on accept and on
decline. There's no new technology, and all API tests, lint, build and
`/validate` pass.
