# Phase 22 — Customer Order Tracking & Status Emails: Validation

Checked on 2026-10-31 unless noted. Where the implementation deviated from
`plan.md`, see its "Implementation notes" section.

## Pass/fail checklist

### Preconditions (Group 0)

- [x] Phase 20 is merged into `master` (PR #35), and this branch is
      moved onto it: `ls apps/api/migrations/*order_payment_lifecycle*`
      exists *(2026-10-31)*.
- [ ] Phase 21 Group 13 is done:
      `grep -rn '/admin/' apps/api/src | grep -v '/api/admin'` returns
      nothing, and `PUBLIC_ADMIN_URL` is set in both compose files.
      *(True in this branch's working tree, where the Group 13 diff was
      applied; still open until `2026-10-31-phase-21-group-13` merges.)*
- [x] SES identity `zainstreat.com` exists in eu-central-1, and its 3
      DKIM CNAMEs and `_dmarc` TXT are in Lightsail DNS *(done
      2026-10-31, during planning)*.
- [x] `aws sesv2 get-email-identity --email-identity zainstreat.com
      --region eu-central-1` shows DKIM `SUCCESS` and
      `VerifiedForSendingStatus: true` *(2026-10-31)*.
- [ ] `aws sesv2 get-account --region eu-central-1` shows
      `"ProductionAccessEnabled": true`. *(Requested 2026-10-31 via `aws
      sesv2 put-account-details`, mail type `TRANSACTIONAL`; review
      `PENDING`.)*
- [x] Staging `BANK_ACCOUNT_NAME`/`BANK_IBAN` are `STAGING – do not pay`
      / `NL91ABNA0417164300` *(2026-10-31)*. Production's are still
      waiting on the client.
- [x] `gh secret list --env production` and `--env staging` both list
      `SES_SMTP_USERNAME` and `SES_SMTP_PASSWORD` *(2026-10-31)*.
- [ ] `gh variable list` shows `SES_SMTP_HOST`, `EMAIL_FROM_ADDRESS` and
      `EMAIL_BUSINESS_NOTIFY_ADDRESS` on both Environments, with
      production's set to the owner's real inbox. *(Only production's
      notify address is missing; until it's set, all production email
      stays off.)*
- [ ] After a deploy, the API logs no `email sending disabled` warning on
      staging or production.

### Migration (Group 1)

- [x] `sqlx migrate run` then `sqlx migrate revert` then `sqlx migrate
      run` succeeds on a fresh dev DB.
- [x] `INSERT … locale = 'de'` fails on `orders_locale_check`, and
      existing orders read `locale = 'en'`.
- [x] `SQLX_OFFLINE=true cargo build --release` succeeds (the `.sqlx`
      cache is up to date).

### Status stamping (Group 2)

- [x] `cargo test order_status` passes, including the
      `customer_can_complete` cases.
- [x] `cargo test --test admin_orders` passes:
      - pickup and delivery orders walked to `completed` have
        `ready_at`, `completed_at` and `completed_by = 'admin'`
      - a cancelled order has `cancelled_at`
      - a customer-completed order can be reopened to its ready status
        (`completed_*` cleared, no customer email); an admin-completed
        order can't (`409`)
- [x] `GET /api/orders/by-token/{token}` returns all six new fields
      (`null` where not reached).

### Emails (Group 3)

- [x] `cargo test order_emails` passes: every emailing status in `en` and
      `nl`, `None` for non-emailing statuses, and the `en` invoice text
      unchanged.
- [ ] With SES configured locally or on staging, confirming payment,
      marking ready/on the way, completing and cancelling each produce one
      customer email in the order's language, linking to
      `/{locale}/order/{token}`. *(Yours to confirm.)*
- [x] A customer completing the order themselves produces **no** email.
- [x] With SES unset, status changes still return `200`. Only a debug
      "email disabled" log appears.

### Customer "received" (Group 4)

- [x] `cargo test --test order_received` passes: `200`, idempotent `200`,
      `409` from `processing` and from `cancelled`, `404` for unknown and
      malformed tokens, and `429` when rate-limited.
- [x] `cargo test --test order_submission` passes: `nl` stored, missing →
      `en`, `de` → `400 validation_error` with `field: "locale"`.

### Tracking page (Groups 5–6)

- [x] `curl -s localhost:3000/en/order/<token>` returns `200`, includes
      `noindex`, and the stepper HTML (`aria-current="step"`).
- [x] `curl -s -o /dev/null -w '%{http_code}' localhost:3000/en/order/not-a-token`
      returns `404`.
- [ ] A pickup order shows "Ready for collection"; a delivery order shows
      "On the way". Reached steps show times. A cancelled order shows
      only the cancelled state.
- [ ] Changing the status in admin updates an open tracking tab within
      about 60s without a reload. A completed or cancelled order's page
      makes no further requests (DevTools Network). *(Yours to confirm.)*
- [ ] The received button appears only in the two ready statuses. Its
      label matches the delivery type, it needs two clicks, and
      afterwards the page shows Completed. *(Yours to confirm.)*
- [x] The `/nl/order/<token>` page is fully Dutch.

### Admin auto-refresh (Group 7)

- [ ] With the dashboard open in a background tab, a new order placed in
      another browser shows up as `(1) …` in the tab title within about
      30s. Focusing the tab clears it, and the new row is highlighted.
      *(Yours to confirm.)*
- [ ] Pressing "I've paid" on an order increments the badge the same way.
- [ ] Reloading a background admin tab that shows `(1)` still shows
      `(1)` (the `sessionStorage` baseline).
- [ ] Order detail shows the ready/completed/cancelled times and
      "Completed by customer" / "Completed by admin". A customer-completed
      order shows "Reopen (customer marked it received)", which needs two
      clicks.

### Sweep (Group 8)

- [x] `pnpm lint` and `pnpm build` in `apps/web` pass, with no `any`.
- [x] `cargo test` passes (all API tests): 159 passed, 0 failed.
- [x] `/validate` passes. The live dev-stack smoke test placed an `nl`
      order through the web proxy (`locale = nl` stored), loaded its
      `en`/`nl` tracking pages, got `409` from `received` before it was
      ready and `200` (`completed`, `customer`, then an idempotent `200`)
      after; the test order was then deleted.
- [ ] After merge, on staging (`<ip>.sslip.io` and `cms.<ip>.sslip.io`):
      the full flow works: order → I've paid → confirm → ready → customer
      confirms receipt, with emails and badge. *(Yours to confirm.)*
- [ ] The client has reviewed the staging emails and tracking page in
      `en` and `nl`, and their wording changes are applied before the
      production promote. *(Yours to confirm.)*

## Definition of done

A customer who places an order gets a private link, in their own
language, to a page that shows where their order is: paid, confirmed,
ready for collection or on the way, and completed. The page updates by
itself while it's open, and when the order is ready they can confirm
they've collected or received it. Each step the owner takes emails the
customer in English or Dutch with that link, and a cancellation tells
them too. The owner can leave the admin open in a background tab, and
new orders and payment claims show up there and in the tab title without
a reload. Everything sits on top of Phase 20's lifecycle with no new
technology, and all API tests, lint, build and `/validate` pass.
