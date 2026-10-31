# Phase 22 — Customer Order Tracking & Status Emails: Requirements

## Source

`specs/roadmap.md`, Phase 22:

> - A public order-status page reachable by an unguessable link (no
>   customer accounts), showing the order's progress through the Phase 20
>   lifecycle.
> - An email to the customer on each status change (payment confirmed,
>   ready for collection / on the way, completed), linking to that page.
> - Completion: the customer can confirm they received or collected the
>   order. The owner can also complete it from admin.
> - New orders appear in admin without a reload while it's open (polling
>   is enough; real push is Phase 27).

Phase 20's spec calls this "Phase 21". It was renumbered when the pipeline
refactor was inserted as Phase 21.

## Context

### Phase 20 is not on `master` yet

Phase 20 (bank-transfer payment and order lifecycle) is implemented on
`2026-10-25-phase-20-bank-transfer-payment`, but at planning time it had
not been merged. `master` already has Phase 21 on top. Phase 21's Group 13
("reconcile with Phase 20") is still open. It adds `PUBLIC_ADMIN_URL`,
switches owner email links to `{admin_url}/orders/{id}`, and adds the
`BANK_*` / `BUSINESS_KVK` vars to `_deploy.yml`'s env renderer. A
read-only `git merge-tree` shows two conflicts:
- the dashboard tile `<Link>` in `apps/web/app/admin/(protected)/page.tsx`
  (keep Phase 20's `className` and master's `/orders?status=` href)
- `specs/roadmap.md`

Everything below assumes that merge has landed. Paths and line numbers
refer to the Phase 20 branch.

### What Phase 20 already provides

- **Statuses.** `apps/api/src/order_status.rs` holds `ORDER_STATUSES`
  (`awaiting_payment`, `payment_claimed`, `processing`,
  `ready_for_collection`, `out_for_delivery`, `completed`, `cancelled`)
  and `admin_next(status, delivery_type)`, the only transition table. A
  test (`tests/order_status_schema.rs`) checks that the list matches the
  DB CHECK.
- **Token.** `orders.public_token UUID UNIQUE DEFAULT gen_random_uuid()`.
  The columns `payment_claimed_at` and `payment_confirmed_at` are also
  there.
- **Public API.** `GET /api/orders/by-token/{token}` and
  `POST /api/orders/by-token/{token}/payment-claim`. The claim endpoint is
  idempotent through a guarded `UPDATE … WHERE status =
  'awaiting_payment'` and rate-limited with `enquiry_rate_limiter()`. A
  malformed or unknown token returns 404.
- **Token page.** `app/[locale]/order/[token]/page.tsx` is an async server
  component. It is `noindex` and `no-referrer`, and shows the status as a
  plain label, `<OrderInvoice>` and `<PaymentClaimButton>`. It does not
  refresh on its own.
- **Admin status changes.** `update_order_status` in
  `apps/api/src/routes/admin/orders.rs` locks the row (`SELECT … FOR
  UPDATE`), checks `admin_next`, updates and commits. It stamps
  `payment_confirmed_at` on `processing`. It sends **no** email and does
  not have `customer_email` or `public_token` in scope.
  `AdminOrderActions` renders the API's `next_statuses`.
- **Email.** `EmailConfig::spawn_send(to, subject, body)` sends plain
  text without waiting (SES SMTP), and does nothing if SES isn't
  configured. `create_order` sends the customer an invoice email
  (`invoice_email_body`) and alerts the owner. All copy is English, and
  the order's locale isn't stored. There is no mailer fake in tests, but
  the body builders are pure `fn … -> String`.
- **Admin pages.** The dashboard and orders list are async server
  components under the `force-dynamic` protected layout. No page in
  `apps/web` polls; the only refresh pattern is `router.refresh()` after a
  mutation.

## Decisions

1. **Phase 20 must be merged first** (asked during planning). Phase 20 is
   merged into `master` and Phase 21 Group 13 is done before any
   implementation group starts. This branch is then rebased onto that
   `master`. Phase 20 was merged on 2026-10-31 (PR #35). Phase 21
   Group 13 is on `2026-10-31-phase-21-group-13` (see Risk 1).

2. **Production email must actually work before this phase ships**
   (found during planning, see Risk 2). Every Phase 22 feature except
   the tracking page is an email, but production had no email at all.
   Group 0 requires:
   - the SES domain `zainstreat.com` verified (DKIM)
   - SES production access granted
   - `SES_SMTP_*`, `EMAIL_FROM_ADDRESS` and
     `EMAIL_BUSINESS_NOTIFY_ADDRESS` set on both GitHub Environments.
     `EmailConfig` turns **all** email off if any of the five is
     missing, including the notify address.
   - **Rejected:** branching off the Phase 20 branch so both phases ship
     together. That would tie two reviews and two migrations together.

3. **The tracking page is Phase 20's token page, extended.** Same URL
   (`/{locale}/order/{token}`), same token, no new mechanism. The plain
   status label becomes a **progress stepper** that follows the order's
   branch:
   - pickup: Order placed → Payment sent → Payment confirmed → Ready for
     collection → Completed
   - delivery: Order placed → Payment sent → Payment confirmed → On the
     way → Completed

   Each reached step shows its timestamp. "Payment sent" is shown as done
   once `payment_confirmed_at` is set, even if the customer never pressed
   "I've paid", but it has no time unless `payment_claimed_at` is set. A
   cancelled order shows a single "Cancelled" state with its time instead
   of the stepper. The invoice stays below the stepper. `noindex` and
   `no-referrer` stay.

4. **Step timestamps are columns, not an events table.** A migration
   adds `ready_at`, `completed_at`, `cancelled_at` (all `TIMESTAMPTZ`) and
   `completed_by TEXT CHECK (completed_by IN ('customer', 'admin'))`. One
   `ready_at` covers both `ready_for_collection` and `out_for_delivery`,
   since an order only ever takes one branch. The admin PATCH stamps these
   in the same `UPDATE` it already runs. `OrderResponse` (by-token and
   create) gains `payment_claimed_at`, `payment_confirmed_at`, `ready_at`,
   `completed_at`, `cancelled_at` and `completed_by`. Existing orders are
   not backfilled: their earlier steps simply show no time.
   - **Rejected:** an `order_status_events` table. The lifecycle is
     linear with one branch, and nobody asked for an audit history.

5. **The order stores the customer's locale** (asked during planning).
   `orders.locale TEXT NOT NULL DEFAULT 'en' CHECK (locale IN ('en',
   'nl'))`. `CreateOrderRequest` gains an optional `locale`. The checkout
   sends next-intl's `useLocale()`. A missing value defaults to `en`; an
   unsupported value returns a `400 validation_error` field error,
   like other field validation. Phase 24 (French) extends the CHECK and the validation.

6. **Customer status emails** (asked during planning). The customer is
   emailed when the admin moves the order to:
   - `processing` ("Payment confirmed, we're preparing your order")
   - `ready_for_collection` ("Your order is ready for collection")
   - `out_for_delivery` ("Your order is on the way")
   - `completed` by the admin ("Your order is complete, thank you")
   - `cancelled` ("Your order has been cancelled", with WhatsApp contact
     for questions)

   Every email links to `{PUBLIC_SITE_URL}/{locale}/order/{token}`. No
   email goes out when the customer completes the order themselves
   (Decision 8): they just pressed the button.

   Mechanics:
   - Sent from `update_order_status` **after `tx.commit()`**, the same
     "send after the write is final" pattern as `create_order` and
     `claim_payment`. The handler takes `State<EmailConfig>`, and its
     `RETURNING` also returns `customer_email`, `public_token` and
     `locale`. The PATCH response shape doesn't change.
   - Plain text, sent fire-and-forget through the existing `spawn_send`.
     A failed send is logged and never fails the status change.

7. **Customer emails are localized; owner emails stay English** (asked
   during planning). A new `apps/api/src/order_emails.rs` holds all
   customer email copy as pure functions that return `(subject, body)`
   for an `en` or `nl` locale. The Phase 20 invoice email moves there too,
   so it's localized as well. Owner emails (new order, payment claimed)
   stay where they are and stay English. Unit tests assert on the built
   strings, because there is no mailer fake.
   - **Rejected:** adding a template crate (`askama`, `tera`) for two
     locales of plain text. Phase 24 adds a third locale arm.

8. **The customer can complete the order.**
   `POST /api/orders/by-token/{token}/received`:
   - From `ready_for_collection` or `out_for_delivery`: sets `completed`,
     `completed_at = now()` and `completed_by = 'customer'`, then returns
     the order. A guarded `UPDATE … WHERE public_token = $1 AND status IN
     ('ready_for_collection', 'out_for_delivery')` decides the outcome, so
     two clicks at once complete the order once.
   - Already `completed` (by anyone): `200` with the order unchanged.
   - Any other status: `409 invalid_status_transition`.
   - Unknown or malformed token: `404`.
   - Rate-limited with `enquiry_rate_limiter()`, the same as
     payment-claim.
   - No owner email. Admin order detail shows "Completed by customer".

   `order_status.rs` gains `customer_can_complete(status) -> bool` next
   to `admin_next`. Admin completion still goes through `admin_next` and
   stamps `completed_by = 'admin'`.

   **The admin can reopen an order the customer completed** (Risk 3,
   asked during planning). For an order where `status = 'completed'` and
   `completed_by = 'customer'`, `admin_next` also offers going back to
   `ready_for_collection` (pickup) or `out_for_delivery` (delivery). That
   needs `completed_by` as a third argument. Reopening clears
   `completed_at` and `completed_by`, keeps `ready_at`, and sends **no**
   customer email; the customer can confirm again later. An order the
   admin completed stays terminal. In admin the action reads "Reopen
   (customer marked it received)".
   - **Rejected:** letting only pickup customers confirm. Delivery
     customers are exactly the ones who know when the food arrived.

   Web: a proxy route `app/api/orders/[token]/received/route.ts`, a copy
   of the payment-claim route. Also a client `OrderReceivedButton` that
   only renders in the two "ready" statuses. It reads "I've collected my
   order" or "I've received my order", depending on `delivery_type`, and
   uses an inline second-click confirm like `AdminOrderActions`' cancel
   (no `window.confirm`).

9. **Admin refreshes itself, with a tab-title badge** (asked during
   planning). A client component `AdminAutoRefresh` is mounted on the
   dashboard and the orders list:
   - It calls `router.refresh()` every **30s**, **including while the tab
     is hidden**, because a background tab is exactly when the badge
     matters. It also refreshes at once when the tab regains focus.
   - The server page passes it the highest order id on the page and the
     ids currently in `payment_claimed`. The component keeps a baseline
     (from mount, then from the last time the tab had focus). The tab
     title becomes `(N) <title>`, where N counts new orders plus new
     claims since that baseline. The badge clears when the tab gets focus.
   - Rows that are new since the baseline get a highlight until the next
     focus.
   - The baseline is kept in `sessionStorage` (wrapped in `try/catch`),
     so reloading the tab doesn't lose the count (Risk 6). It's per tab
     by design.
   - No sound.
   - **Rejected:** SSE/WebSocket push. That's Phase 27, and polling is
     fine for one or two admins.

10. **The tracking page refreshes itself** (asked during planning). A
   client `OrderAutoRefresh` calls `router.refresh()` every **60s** while
   `document.visibilityState === 'visible'`, and once at once when the tab
   becomes visible again. The page doesn't render it for `completed` or
   `cancelled` orders, so polling stops once the order is finished.

11. **No new technology.** `tech-stack.md` is unchanged. Everything uses
    the existing Axum, sqlx, `EmailConfig`, next-intl and
    `router.refresh()`.

## Out of scope

- Real push (SSE, Web Push) and the installable admin: **Phase 27**.
- WhatsApp or SMS messages to customers. Phase 28 is owner WhatsApp
  notifications.
- French copy for the tracking page and emails: **Phase 24**.
- Catering quotations using this page: **Phase 23**.
- HTML email templates and PDF invoices (still deferred from Phase 17 and
  Phase 20).
- Reopening an order the admin completed, or a cancelled one. Only
  customer-completed orders can be reopened (Decision 8).
- Automatically cancelling unpaid orders, and reminder emails for unpaid
  orders.
- An admin-side status history or audit log beyond the Decision 4
  timestamps.

## Open risks flagged during planning

1. **Phase 20 and Phase 21 must be reconciled first.** Two known merge
   conflicts (see Context) plus Phase 21's Group 13 steps.
2. **Production email doesn't work at all.** At planning time, SES had
   no verified identity in any region, the account was in the sandbox,
   and none of the SES/email variables were set on either GitHub
   Environment, so `EmailConfig` switched email off without any error.
   There was also no MX record, so `orders@zainstreat.com` can't receive
   mail.
3. **A customer can mark an order completed too early,** for example
   before a delivery actually arrives, and `completed` was terminal.
4. **Customer-facing email copy, especially Dutch, should be checked by
   the client** before production.
5. **Polling load** is two requests a minute per open admin tab and one a
   minute per open tracking page, each a single indexed query.
6. **The badge baseline is per tab and resets on reload.**

### Resolution (2026-10-31, during planning)

| Risk | Handled by |
|---|---|
| 1. Phase 20/21 reconciliation | Phase 20 merged into `master` (PR #35, 2026-10-31), and this branch was moved onto it. Phase 21 Group 13 is on branch `2026-10-31-phase-21-group-13` (worktree `../zainstreat-merge-p20`), waiting for the user to commit: `PUBLIC_ADMIN_URL` with owner links set to `{admin_url}/orders/{id}`, `BANK_*` in `_deploy.yml`, phase renumbering, and restoring Phase 20's "needs action" dashboard tile, which the PR's conflict resolution dropped. `cargo test` 138/138, web lint and build pass. Decision 1 and Group 0. |
| 2. No production email | Partly done; Decision 2 and Group 0 gate the rest. **Done:** SES identity `zainstreat.com` (eu-central-1, Easy DKIM 2048) with 3 DKIM CNAMEs and `_dmarc` (`p=none`) in Lightsail DNS; IAM user `zainstreat-ses-smtp` (`ses:SendRawEmail` only), its key in SSM `/zainstreat/ses-smtp/*`; `SES_SMTP_HOST` and `EMAIL_FROM_ADDRESS` on both Environments; staging `EMAIL_BUSINESS_NOTIFY_ADDRESS`. DKIM verified the same day. **Open:** set the SMTP secrets (`set-ses-secrets.sh`), request SES production access, set production's `EMAIL_BUSINESS_NOTIFY_ADDRESS` (the owner's real inbox). MX for `@zainstreat.com` is not needed, since owner alerts go to that inbox. |
| 3. Early completion | Decision 8: the admin can reopen an order the customer completed. |
| 4. Email copy review | The client reviews real emails on staging before production is promoted (Group 8). Copy lives in one file, `order_emails.rs`. |
| 5. Polling load | Accepted: negligible on the current Lightsail box; Phase 27's push replaces it. |
| 6. Badge baseline | Decision 9: the baseline is kept in `sessionStorage`, so it survives a reload. Per tab by design. |
