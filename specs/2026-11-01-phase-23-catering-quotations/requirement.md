# Phase 23 — Catering Quotations: Requirements

## Source

`specs/roadmap.md`, Phase 23:

> - Admin: turn a catering enquiry into a quotation (line items, total,
>   notes) and send it to the customer by email.
> - Customer accepts the quote via a link. From there it follows the same
>   bank-transfer → owner confirms → processing → pickup/delivery →
>   completed lifecycle as Phase 20/22.

`specs/client-notes/2026-10-18-client-call.md`, "Catering / events":
event orders don't go through the food-order flow. The customer requests
a quotation, the owner sends it, and if it's accepted the customer pays
the same way, the owner confirms, and it continues like an order
(including pickup/delivery).

## Context

### What exists

- **Catering enquiries (Phase 15).** `catering_enquiries` has the
  customer's name/phone/email, `event_type`, `event_date`,
  `guest_count`, `location`, `services_required` and `message`. It has
  **no status and no locale**. `POST /api/catering-enquiries` validates,
  inserts and emails the owner. Admin has a list
  (`/enquiries?type=catering`) and a read-only detail page
  (`app/admin/(protected)/enquiries/catering/[id]/page.tsx`) backed by
  `GET /api/admin/catering-enquiries/{id}`.
- **Order lifecycle (Phases 20 and 22).** `orders` has `public_token`,
  `locale`, the payment and step timestamps, and `status` driven by
  `order_status::admin_next`. `order_items.menu_item_id` is already
  nullable (`ON DELETE SET NULL`), so an item doesn't have to be on the
  menu. The tracking page `/{locale}/order/{token}`, the invoice email
  (`order_emails::invoice_email`), "I've paid", "received", the status
  emails and the admin auto-refresh badge all work from an `orders` row.
- **Email.** `EmailConfig::spawn_send` (fire-and-forget), `site_url()`
  and `admin_url()`. Customer copy lives in `order_emails.rs` as pure
  `en`/`nl` functions. Owner copy is English.
- **Payment reference.** `payment::payment_reference(order_id)` →
  `ZTM-{id}`.

## Decisions

1. **The quote is its own table; accepting it creates an order** (asked
   during planning). A new `catering_quotes` table covers the quote stage
   only: `draft` → `sent` → `accepted` or `declined`. When the customer
   accepts, the API creates a normal `orders` row in the same
   transaction: customer details and locale from the enquiry, the quote's
   delivery type and address, and one `order_items` row per quote line
   (`menu_item_id = NULL`). The quote stores the new `order_id`. From then
   on, everything from Phases 20 and 22 applies unchanged: invoice email,
   `ZTM-N` reference, "I've paid", admin "Confirm payment", tracking page,
   status emails, completion and the admin badge.
   - **Rejected:** a separate lifecycle on the quote. It would duplicate
     the order status machine, the tracking page and the emails.

2. **Line items are free text** (asked during planning). Each line is a
   description, a whole-number quantity and a unit price typed in by the
   owner. This covers food per guest, rentals, serving staff and the
   delivery fee, none of which are necessarily on the menu. The quote
   total is the sum of the line subtotals and is computed by the API.
   There is no separate delivery fee: the owner adds it as a line, and
   the order gets `delivery_fee = 0`, the same as Phase 10. No VAT is
   added (KOR, Phase 20).

3. **One quote per enquiry, editable until accepted, with a valid-until
   date** (asked during planning).
   - `catering_quotes.enquiry_id` is `UNIQUE`.
   - The owner can save it as often as they like while it isn't
     `accepted`. **Saving a `sent` or `declined` quote moves it back to
     `draft`**. A draft can't be accepted, so the customer never accepts
     a version they weren't sent. Their link shows "This quote is being
     updated" until the owner sends it again.
   - "Send" moves a `draft` (or re-sends a `sent`) quote to `sent`,
     stamps `sent_at`, and emails the customer. The link (same token) is
     always the latest version. A decline reason is cleared on send.
   - `valid_until` (a date) is required and must not be in the past when
     sending. After it passes (Europe/Amsterdam date), Accept returns
     `409 quote_expired` and the page says the quote has expired and to
     contact the business. "Expired" is computed, not stored. The owner
     extends it by editing and resending.
   - An `accepted` quote is read-only. Changes after acceptance go
     through the order (cancel it, and so on), not the quote.
   - **Rejected:** a new quote per revision (history, but more complex),
     and no expiry (prices could be weeks out of date).

4. **The customer can accept or decline** (asked during planning). The
   public page `/{locale}/quote/{token}` shows the event details, lines,
   total, the owner's notes, the delivery type/address and the valid-until
   date. Each button needs a second click to confirm (no
   `window.confirm`), like `OrderReceivedButton`.
   - **Accept** (`POST /api/quotes/by-token/{token}/accept`): only from
     `sent` and not expired. It's idempotent: an already accepted quote
     returns `200` with the same order. Two clicks at once create one
     order (`SELECT … FOR UPDATE` on the quote). The response carries the
     order's `public_token`, and the page sends the customer to
     `/{locale}/order/{order_token}`, which shows the invoice and bank
     details. The customer gets the normal localized invoice email and
     the owner gets an English "Quote accepted" email linking to
     `{admin_url}/orders/{id}`.
   - **Decline** (`POST …/decline`, optional `reason` ≤ 500 chars): from
     `sent` (expired or not). Already declined → `200`. Accepted or draft
     → `409`. The owner gets an English email with the reason, linking to
     `{admin_url}/enquiries/catering/{enquiry_id}`. No customer email.
   - Both are rate-limited with `enquiry_rate_limiter()`. An unknown or
     malformed token is `404`.
   - Once accepted, the quote page shows "Accepted" and a link to the
     order page.

5. **The enquiry stores the customer's locale.** `catering_enquiries`
   gains `locale TEXT NOT NULL DEFAULT 'en' CHECK (locale IN ('en',
   'nl'))`, sent by the catering form from `useLocale()` and validated
   the same way as Phase 22's order locale (`400 validation_error`,
   `field: "locale"`). The quote email, its link and the order created
   on accept all use it. Older enquiries are `en`.

6. **Fulfilment is set on the quote: pickup, delivery or on-site** (Risk
   2, asked during planning). The owner picks one of three. Delivery and
   on-site both need an address, prefilled in the editor from the
   enquiry's `location`. The editor's default comes from the enquiry's
   `services_required`:
   - `on_site` if it includes `catering`
   - `delivery` if it includes `meal_delivery`
   - `pickup` otherwise

   These copy onto the order, so the order follows the branch for its
   type (Decision 12).

7. **The created order.** Customer name/email/phone come from the
   enquiry. `subtotal = total = quote total`, `delivery_fee = 0`, and
   `locale` comes from the enquiry. `notes` is
   `Catering quote #Q for {event_type} on {event_date}, {guest_count} guests`,
   so the context shows on the invoice and in admin without new order
   columns. The order has no column pointing back to the quote:
   `catering_quotes.order_id` (`UNIQUE`) is the link. Admin order
   queries `LEFT JOIN` it to show a "Catering" tag and a link back to the
   enquiry.

8. **Admin UI lives on the enquiry detail page.** The detail page gains
   a quote panel: a status badge (`Draft`, `Sent`, `Expired`, `Accepted`,
   `Declined` plus reason), and an editor (delivery type, address,
   valid-until, notes for the customer, line rows with add/remove and a
   live total) with "Save draft" and "Send to customer" (second-click
   confirm). It's read-only with a link to the order once accepted. The
   catering enquiry list shows each enquiry's quote status. The admin
   order list and detail show the "Catering" tag. Admin copy stays
   English inline.

9. **Customer copy is localized and lives with the other customer
   emails.** `order_emails.rs` gains `quote_email(locale, …)` returning
   `(subject, body)` in `en`/`nl`: greeting, event summary, lines,
   total, valid-until, notes and the link
   `{site_url}/{locale}/quote/{token}`. Owner emails (accepted, declined)
   are English, next to the existing owner alerts. The quote page copy
   goes in `messages/{en,nl}.json` under a new `Quote` namespace. All of
   the wording is in `copy.md` (Decision 15).

10. **Quote numbers.** Quotes are shown as `#{quote id}` (for example
    "Quote #12"). The `ZTM-N` payment reference only exists once the order
    is created, so the quote doesn't show bank details. The order page
    does.

11. **No new technology.** `tech-stack.md` is unchanged.

12. **On-site orders get their own lifecycle branch** (Risk 2, asked
    during planning). Catering served at the venue is neither collection
    nor delivery, so it gets a third branch:
    - `orders.delivery_type` gains `on_site`. The column keeps its name;
      renaming it would touch every order query for no behaviour change.
      `catering_quotes.delivery_type` allows the same three values.
    - A new order status, `serving_on_site`, reads "Catering in progress"
      (Dutch: "Catering bezig"). The on-site branch is: awaiting payment →
      payment claimed → processing → **catering in progress** →
      completed, plus cancelled.
    - `admin_next`:
      - `processing` + `on_site` → `serving_on_site` or `cancelled`
      - `serving_on_site` → `completed` or `cancelled`
      - a customer-completed `on_site` order reopens to `serving_on_site`
    - `serving_on_site` stamps `ready_at`, like the other "handed over"
      statuses.
    - `serving_on_site` joins `CUSTOMER_COMPLETABLE`, so the customer
      can confirm afterwards. The button reads "Our event has been
      catered". The admin can still complete it themselves.
    - **Only quotes can create `on_site` orders.** The cart's
      `DeliveryType` enum stays `Pickup | Delivery`, so a cart order
      with `on_site` is rejected at deserialization as it is today for
      any unknown value.
    - Customer-facing changes:
      - tracking stepper: Order placed → Payment sent → Payment
        confirmed → Catering in progress → Completed
      - a status email for `serving_on_site` ("Our team is at your
        event")
      - the invoice line "Served at: {address}"
      - the admin label "Mark catering started"
    - `ORDER_STATUSES` has to change in all three places: the DB CHECK,
      `order_status.rs` and `lib/orders.ts`
      (`tests/order_status_schema.rs` catches drift).
    - **Rejected:** reusing `delivery` with catering-specific labels.
      The internal status would still be "out for delivery" for food
      served at a venue, and every label would need to branch on
      "is this a catering order".

13. **Accept and decline are tied to the version the customer saw**
    (Risk 1). `catering_quotes.version INTEGER NOT NULL DEFAULT 1` goes up
    by one on every admin save. The public `GET` returns it, and the page
    sends it back in the accept and decline bodies (`{ "version": n }`).
    If it doesn't match the current version, the API returns `409
    {"error": "quote_changed"}`, and the page says "This quote has changed,
    refresh". Revert-to-draft (Decision 3) alone isn't enough: if the owner
    saves and resends while the customer's page is still open, an accept
    from that stale page would otherwise go through for a version the
    customer never saw. An already-accepted quote still returns `200`
    for any version, so a retried request stays idempotent.

14. **Quotes work even when email is off** (Risk 3, asked during
    planning).
    - `EmailConfig` gains `is_enabled()`. `POST …/quote/send` returns
      `{ quote, email_queued: bool }`.
    - Once a quote is `sent`, admin always shows a share panel: the
      quote link with "Copy link", and "Send via WhatsApp". The WhatsApp
      button opens `wa.me/{customer phone}` with the prefilled message
      from `copy.md` §4, in the enquiry's language.
    - When `email_queued` is false, admin shows "Email is switched off.
      Share the link with the customer instead."
    - The customer's phone is turned into a `wa.me` number by
      `toWhatsAppNumber()`: keep digits only, drop a leading `00`, and
      replace a leading `0` with `31`. If that leaves fewer than 8
      digits, the WhatsApp button is hidden and Copy link remains.
    - Accept and decline still email the owner when email is on. When
      it's off, the admin badge still picks up the new order, and the
      enquiry list shows the quote status.
    - **Rejected:** relying on email alone. Email can be off (no SES
      configured, as in local dev), and some customers answer WhatsApp
      faster than email.

15. **Customer copy is written in the spec first** (Risk 4, asked during
    planning). `copy.md` in this folder holds every customer-facing
    string for the phase in English and Dutch: the quote email, the quote
    page, the on-site texts and the WhatsApp message. It goes to the
    client **before** implementation, and the code uses it word for
    word. The staging review (plan Group 10) is a final check, not the
    first time the client sees the wording.

16. **Guards against typo totals** (Risk 5).
    - The API caps (plan 3.1): at most 50 lines, quantity at most 10,000,
      unit price at most €100,000.00 with 2 decimals, and a total above 0.
    - The admin Send confirmation shows the total: "Send quote for
      €{total} to {email}?".
    - The editor shows a non-blocking warning, "Large total, please
      double-check", when the total is €10,000 or more.

## Out of scope

- Deposits or partial payments: the full total is paid up front, as with
  food orders. Revisit if the client asks.
- A menu picker for quote lines. Free text only (Decision 2).
- Quote version history, PDF quotes and HTML email.
- French copy: **Phase 24** adds the `fr` arm to `quote_email` and the
  `Quote` messages.
- WhatsApp notification of accepted or declined quotes: **Phase 28**.
- A status or "handled" flag on enquiries without a quote, and enquiry
  archiving.
- Reminder emails before a quote expires.
- Editing an accepted quote. Post-acceptance changes go through the order.
- `on_site` fulfilment for cart orders. Only quotes offer it
  (Decision 12).

## Open risks flagged during planning

1. **Owner edits while the customer is looking.** Saving reverts a sent
   quote to draft (Decision 3). But if the owner saves and resends while
   the customer still has the old page open, an Accept from that page
   would accept a version the customer never saw.
2. **Pickup and delivery don't fit on-site catering.** Catering served
   at the venue is neither collection nor delivery, so "On the way" and
   "I've received my order" would be wrong for it.
3. **Email deliverability.** Quotes reach the customer only by email,
   and `EmailConfig` switches all email off when SES isn't fully
   configured.
4. **Client copy review.** If the client first sees the quote email and
   page wording (especially Dutch) on staging, any change comes after
   implementation.
5. **Large totals.** Catering totals can be large, and a typo (an extra
   zero) could go out to a customer.

### Resolution (2026-11-01, during planning)

| Risk | Handled by |
|---|---|
| 1. Stale accept | Decision 13: a `version` column, sent back by the page. A mismatch returns `409 quote_changed`. |
| 2. On-site catering | Decisions 6 and 12: an `on_site` fulfilment type for quotes only, with a "Catering in progress" (`serving_on_site`) status. |
| 3. Email off | Decision 14: the admin share panel (Copy link, Send via WhatsApp) and an `email_queued` flag, so quotes work without email. Production rollout is out of scope for this phase. |
| 4. Copy review | Decision 15: `copy.md` goes to the client before implementation; staging is the final check. |
| 5. Typo totals | Decision 16: API caps, the total in the Send confirmation, and a warning at €10,000 or more. `NUMERIC(10,2)` (up to 99,999,999.99) is ample. |
