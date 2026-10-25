# Phase 20 — Bank-Transfer Payment & Order Lifecycle: Requirements

## Source

`specs/roadmap.md`, Phase 20 (from the 2026-10-18 client call,
`specs/client-notes/2026-10-18-client-call.md`):

> From the 2026-10-18 client call: food is paid **before** it's prepared,
> and only verified payments are processed. No payment provider yet — the
> customer pays by bank transfer and the owner verifies the bank alert by
> hand. No VAT is added (the business is under the KOR scheme).
>
> - Order lifecycle reworked around payment, with branches for pickup and
>   delivery: *awaiting payment* → *payment claimed* (customer says they
>   paid) → *confirmed / processing* (owner verified the payment; one
>   admin action, automatically moving the order to processing) → *ready
>   for collection* (pickup) or *on the way* (delivery) → *completed*, plus
>   *cancelled*. Replaces the current `new/confirmed/preparing/ready/
>   completed/cancelled` CHECK values via a migration that maps existing
>   orders.
> - An invoice for each order (order number, items, total, the business's
>   bank account details and payment reference), shown on the confirmation
>   page and emailed to the customer. States clearly that only verified
>   payments are processed.
> - An "I've paid" action for the customer on their order.
> - Admin: a "Confirm payment" action and status transitions that follow
>   the order's pickup/delivery type. The checkout's existing
>   pickup/delivery choice (Phase 10) drives the branch.
> - The owner is emailed on every new order and every "I've paid" claim.

## Context

- **Order data.** `orders.status` is a `TEXT` column with a CHECK of six
  values (`migrations/20261002162443_initial_schema.up.sql`). The same list
  is repeated in `ORDER_STATUSES` in `apps/api/src/routes/admin/orders.rs`
  and `apps/web/lib/admin-orders.ts`. `delivery_type` (`pickup`/`delivery`)
  has been captured at checkout since Phase 10.
- **Admin status changes.** Phase 13 added `PATCH
  /api/admin/orders/{id}/status`, which accepts any status from any other.
  The UI for it is the `AdminOrderStatusSelect` dropdown.
- **Confirmation.** After `POST /api/orders`, `OrderView` shows
  `OrderSuccess`, and `sessionStorage` keeps it across a refresh. Once the
  tab is closed, the customer has no way back to their order. The current
  copy says "We'll contact you by phone or WhatsApp shortly to confirm".
- **Email.** Phase 17's `EmailConfig::spawn_send` (fire-and-forget, plain
  text, SES over SMTP) sends the customer a short "we've received order
  #N" email. **The owner gets no email for orders today**, only for
  enquiries.
- **Accounts.** There are no customer accounts, and the mission keeps it
  that way, so the only way back to an order is a link that can't be
  guessed.
- **Launch.** The client wants to share the site publicly around
  2026-10-24/25, and this phase is what lets a real customer actually pay.

## Decisions

1. **Seven status values, stored as snake_case `TEXT` with a CHECK** (same
   pattern as today, no Postgres enum):
   `awaiting_payment` (default), `payment_claimed`, `processing`,
   `ready_for_collection`, `out_for_delivery`, `completed`, `cancelled`.
   The roadmap's "confirmed / processing" is **one** status,
   `processing`. The client wants confirming payment to move the order
   straight to processing, so a separate `confirmed` value would never be
   seen. When payment was verified is recorded in a new
   `payment_confirmed_at` column instead.

2. **Migration mapping for existing orders:**
   `new` → `awaiting_payment`; `confirmed`, `preparing` → `processing`;
   `ready` → `ready_for_collection` (pickup) / `out_for_delivery`
   (delivery); `completed`, `cancelled` unchanged. The down migration
   reverses it (it loses detail, but it's valid). Existing orders are not
   backfilled with `payment_confirmed_at`: those orders came before this
   payment flow.

3. **Transitions are guided in the UI and enforced by the API** (asked
   during planning). A single pure function `allowed_next(status,
   delivery_type)` in Rust is the only source of truth. TypeScript has a
   mirror of it that is used only to choose which buttons to show:

   | From | To |
   |---|---|
   | `awaiting_payment` | `payment_claimed` (customer only), `processing`, `cancelled` |
   | `payment_claimed` | `processing`, `cancelled` |
   | `processing` | `ready_for_collection` (pickup) **or** `out_for_delivery` (delivery), `cancelled` |
   | `ready_for_collection` / `out_for_delivery` | `completed`, `cancelled` |
   | `completed`, `cancelled` | — (terminal) |

   Admin may confirm payment straight from `awaiting_payment`, for when a
   customer paid but never pressed "I've paid". An invalid admin
   transition returns `409 {"error":"invalid_status_transition"}`. Admin
   can't set `payment_claimed`, because that's the customer's claim. The
   existing `PATCH .../status` endpoint stays: "Confirm payment" is just
   `PATCH {status:"processing"}`, which also stamps
   `payment_confirmed_at`. No new admin endpoint.

4. **Unguessable per-order link now** (asked during planning). A new
   `orders.public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid()`
   (built into Postgres 17). A minimal public page,
   `/{locale}/order/{token}`, shows the invoice, the current status (as a
   plain label), and the "I've paid" button. The invoice email links to
   it. Phase 21 turns this same page and token into the full tracking
   page, so no new mechanism is needed later. The page is `noindex`,
   disallowed in `robots.txt`, and sets `referrer: no-referrer` so the
   token isn't leaked to `wa.me` and other outbound links.

5. **Bank details come from API env vars** (asked during planning):
   `BANK_ACCOUNT_NAME`, `BANK_IBAN` (both required to turn it on) and
   `BANK_BIC` (optional), handled the same way as `EmailConfig` (blank
   counts as unset). Order responses carry a `payment` object with the
   account name, IBAN, BIC and reference, or `null` when the details
   aren't configured. When it's `null`, ordering still works: the invoice
   asks the customer to contact the business on WhatsApp for payment
   details, and the API logs a startup `warn`. Orders keep working even if
   the config is missing.

6. **Payment reference: `ZTM-{order id}`** (asked during planning), e.g.
   `ZTM-1042`. It's worked out from the id by one Rust helper,
   `payment_reference(id)`, and not stored, so it can't drift. It's
   returned in the order responses so the frontend never builds it
   itself.

7. **"I've paid" is a public, token-scoped, idempotent endpoint.**
   `POST /api/orders/by-token/{token}/payment-claim`:
   - `awaiting_payment` → `payment_claimed`, stamps `payment_claimed_at`,
     emails the owner, and returns the order.
   - Already `payment_claimed` or further along: returns `200` with the
     order unchanged and **no second email** (protects against double
     clicks).
   - `cancelled`: `409 invalid_status_transition`.
   - Unknown or malformed token: `404`.
   - Rate-limited with the existing `enquiry_rate_limiter()`.
   The `UPDATE ... WHERE status = 'awaiting_payment'` is guarded, so two
   requests at the same moment produce one email, not two.

8. **Invoice content** (confirmation page, token page and email are the
   same): order number, date, items with options/quantity/line totals,
   total, pickup or delivery (with the address for delivery), the bank
   details, the payment reference, and the line *"We only start preparing
   your order once your payment has been verified."* **No VAT line**
   (KOR). Plain-text email, English only. Both choices continue Phase 17
   (HTML email and per-locale emails stay deferred; French arrives in
   Phase 23). No PDF: the page can be printed, and a PDF crate isn't worth
   it yet.

9. **Emails in this phase** (all fire-and-forget through the existing
   `spawn_send`):
   - Customer, new order: replaces the current one-liner with the
     Decision 8 invoice and the link to the token page.
   - Owner, new order: order number, customer name/phone/type, total,
     reference, and a link to `/admin/orders/{id}`.
   - Owner, payment claimed: "Customer says they've paid order ZTM-N
     (€X). Check the bank alert, then confirm payment in admin," plus the
     admin link.
   Status-change emails to the customer belong to Phase 21.

10. **New API env var `PUBLIC_SITE_URL`** (`https://${DOMAIN}` in prod
    compose, `http://localhost:3000` in dev) for links in emails.
    Customer links have no locale prefix (`/order/{token}`). The existing
    next-intl proxy sends them to the visitor's locale.

11. **Completing an order is admin-only in this phase.** Customer
    confirmation of receipt/collection is Phase 21.

12. **Admin UI.** The status dropdown is replaced by `AdminOrderActions`:
    one primary button for the next step ("Confirm payment", "Ready for
    collection" / "Mark on the way", "Complete") and a secondary "Cancel
    order" with an inline second-click confirm (no `window.confirm`).
    Order detail adds the payment reference and the claimed/confirmed
    times. Statuses get human-readable labels (`STATUS_LABELS`) instead of
    raw values. On the dashboard, `payment_claimed` comes first and is
    marked as needing action. The list's status filter shows all seven.

## Out of scope

- The full order-tracking page, customer status emails, customer
  "received/collected" confirmation, and admin auto-refresh: **Phase 21**.
- iDEAL or any payment provider, and the VAT treatment that comes with it
  (client call: settle when iDEAL is built).
- Automatically cancelling unpaid orders after some time.
- Delivery fees (still `0`, Phase 10 Decision 9).
- An admin-editable settings page for bank details (Decision 5 uses env
  vars; Phase 24 is the admin-content phase).
- Catering quotations paying through this flow: **Phase 22**.
- PDF invoices and HTML email templates.

## Open risks flagged during planning

Each risk ends with how it was handled during implementation
(2026-10-25).

1. **The real bank details are needed from the client** (account holder
   name, IBAN, and BIC if wanted) before production. Until they're in the
   server `.env`, the invoice shows the WhatsApp fallback.
   *Mitigated, client input still needed.* `BANK_IBAN` is normalised
   (spaces removed, upper-cased) and checked against the ISO 13616 mod-97
   checksum at startup (`payment::normalize_iban`). An invalid value is
   treated as unset and logs `ERROR BANK_IBAN is not a valid IBAN`, so a
   typo can never show customers a wrong account.
2. **Is this a legal invoice?** KOR participants are exempt from Dutch
   invoicing requirements, so a "payment request" with no VAT line should
   be fine. The client may still want their KvK number on it. Confirm
   with the client. Adding it later is a one-line copy change.
   *Mitigated, client input still needed.* A new optional env var,
   `BUSINESS_KVK`, prints "KvK: …" under the bank details on the invoice
   page and in the email. Whatever the client answers, it's a config
   change, not a code change.
3. **SES sandbox.** Phase 18 asked for SES production access. If it
   hasn't been granted, customer invoice emails reach only verified
   addresses. The confirmation page and token page still show the
   invoice either way.
   *Open: check before deploy.* `aws sesv2 get-account --region
   eu-central-1` must show `"ProductionAccessEnabled": true`. This
   couldn't be checked during implementation because the local AWS
   session had expired.
4. **Status list now in three places** (CHECK constraint, Rust
   `ORDER_STATUSES` + `allowed_next`, TS mirror). This drift risk was
   already noted in Phase 13 (open risk 7). An API integration test that
   walks every allowed and disallowed transition limits it.
   *Resolved.* The TS transition mirror is gone. `GET
   /api/admin/orders/{id}` returns `next_statuses` (from `admin_next`),
   and `AdminOrderActions` renders exactly that list. A new
   `tests/order_status_schema.rs` reads `orders_status_check` back with
   `pg_get_constraintdef` and fails if it differs from `ORDER_STATUSES`.
   The only list left unchecked is the TS `ORDER_STATUSES` used for
   labels; there's no web test runner, and the badge's
   `Record<OrderStatus, …>` types make a missing label a compile error.
5. **Existing production orders.** The Decision 2 mapping is
   mechanical. Any real `new` orders become `awaiting_payment` even if
   they were already handled by phone. Check the production order list
   before deploying and tidy up by hand if needed.
   *Open: ops step before merge.* plan.md 7.3 now has a read-only
   preview query that shows each open order's old and new status. Tidy
   up in `/admin` before the merge, while the old dropdown still allows
   any move.
6. **`sqlx` needs its `uuid` feature** to read the `UUID` column. The
   `uuid` crate is already a dependency (Phase 14), so this only enables
   a feature and isn't a new technology decision. `tech-stack.md` is
   unchanged.
   *Resolved.* Enabled in `Cargo.toml` (plus `uuid`'s `serde` feature),
   and the offline `.sqlx` release build passes.
