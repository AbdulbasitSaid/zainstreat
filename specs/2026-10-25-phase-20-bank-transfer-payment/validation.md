# Phase 20 — Bank-Transfer Payment & Order Lifecycle: Validation

## Pass/fail checklist

### Migration (Group 1)

- [x] On a local DB holding one order in each old status (seed by hand
      before running the migration), the migration maps them as follows:
      `new`→`awaiting_payment`, `confirmed`/`preparing`→`processing`,
      `ready`+pickup→`ready_for_collection`, `ready`+delivery→`out_for_delivery`,
      and leaves `completed`/`cancelled` unchanged.
- [x] Every row has a distinct non-null `public_token`
      (`SELECT COUNT(DISTINCT public_token) = COUNT(*) FROM orders`).
- [x] `INSERT ... status = 'new'` is rejected by the CHECK.
- [x] `sqlx migrate revert` followed by `sqlx migrate run` both succeed.

### API (Groups 2–3)

- [x] `cargo test` passes, including the `order_status` unit tests and
      the new `order_payment_claim.rs`.
- [x] `POST /api/orders` → `201` with `status: "awaiting_payment"`, a
      UUID `public_token`, `payment_reference: "ZTM-<id>"`, and a
      `payment` object (or `payment: null` when `BANK_IBAN` is unset).
- [x] `GET /api/orders/by-token/<token>` → `200`. A random UUID → `404`.
      `not-a-uuid` → `404` (not 400).
- [x] `POST .../payment-claim` → `200` with `payment_claimed`. A second
      call → `200` with the status unchanged, and the API logs show only
      **one** owner email being sent.
- [x] Claiming a cancelled order → `409 {"error":"invalid_status_transition"}`.
- [x] Admin `PATCH .../status {"status":"processing"}` from
      `awaiting_payment` → `200` with `payment_confirmed_at` set.
- [x] Admin `PATCH` to `out_for_delivery` on a **pickup** order → `409`.
      `payment_claimed` from admin → `409`. Any change from
      `completed` → `409`. `"bogus"` → `422`.
- [x] `GET /api/admin/orders/summary` → `counts` has exactly the seven
      new keys.
- [ ] With SES configured locally (or the logs read in disabled mode),
      placing an order triggers a customer invoice email (bank details,
      reference, "only verified payments" line, `/order/<token>` link)
      **and** an owner new-order email.

### Public site (Groups 4–5)

- [ ] Placing an order on `/en/order` shows the invoice: items, total,
      account name, IBAN, reference `ZTM-<id>`, the verified-payments
      notice, no VAT line, and the "I've paid" button.
- [ ] Pressing "I've paid" shows the thanks message, and the button
      disappears after the refresh.
- [x] Opening `/en/order/<token>` in a fresh private window shows the same
      invoice and the current status. `/nl/order/<token>` shows it in Dutch.
- [x] `/en/order/00000000-0000-0000-0000-000000000000` and
      `/en/order/garbage` both render the 404 page.
- [x] The token page's HTML contains `<meta name="robots" content="noindex, nofollow">`
      and `<meta name="referrer" content="no-referrer">`.
- [x] `curl -s localhost:3000/robots.txt` lists `Disallow: /*/order/`.
      `/en/order` (checkout) still renders.
- [x] With `BANK_IBAN` unset, the invoice shows the WhatsApp fallback
      instead of empty bank fields.
- [ ] An old `zainstreat:last-order:v1` entry in sessionStorage is
      ignored (no crash, no token-less invoice).

### Admin (Group 6)

- [ ] The order detail of an `awaiting_payment` delivery order shows
      "Confirm payment" and "Cancel order". After confirming, it shows
      "Mark on the way", then "Mark completed", then no actions.
- [ ] A pickup order shows "Ready for collection" in place of "Mark on the way".
- [ ] "Cancel order" asks for a second click before it sends anything.
- [ ] The detail page shows the reference and the claimed/confirmed times.
- [ ] The dashboard shows seven tiles with `payment_claimed` first, and
      that tile is highlighted when its count is above 0. The list's
      filter offers all seven with readable labels.
- [ ] `grep -rn "preparing\|admin-order-status-select" apps/web apps/api/src`
      finds nothing.

### Sweep

- [x] `/validate` passes (lint, build, smoke test).

### Production, after deploy (Group 7)

- [ ] The server `.env` has `BANK_ACCOUNT_NAME`/`BANK_IBAN` (and
      `BANK_BIC` if wanted). A Lightsail snapshot was taken before the
      merge.
- [ ] The API container logs show the migration applied and **no**
      "bank details not set" warning.
- [ ] Place a real €-small test order on `https://zainstreat.com`. The
      customer email arrives with the right IBAN/reference and a working
      `https://zainstreat.com/order/<token>` link, and the owner gets the
      new-order email.
- [ ] Press "I've paid" and the owner gets the claim email. Confirm the
      payment in `/admin`, walk the order to completed, then cancel or
      delete the test order by hand if needed.

## Definition of done

A customer who places an order immediately sees, and receives by email,
an invoice with the business's bank details, a `ZTM-<id>` reference and a
clear statement that only verified payments are processed. They can come
back to it at any time through an unguessable link and press "I've paid".
The owner is emailed on every new order and every payment claim, confirms
the payment with one click (which moves the order to processing), and
then has only the valid next steps for that order's pickup or delivery
type, all enforced by the API. Every existing order has been moved onto
the new seven-status lifecycle.
