# Phase 20 — Bank-Transfer Payment & Order Lifecycle: Implementation Plan

Groups 1–3 are API work, Groups 4–6 are web work, and Group 7 covers ops
and the deploy. Decision numbers refer to `requirement.md`.

## Group 0 — Branch

Branched `2026-10-25-phase-20-bank-transfer-payment` off `master`
(Phase 19 merged as `57a3a2a`).

## Group 1 — Migration & config (API)

Depends on: nothing.

1.1. `cd apps/api && sqlx migrate add -r order_payment_lifecycle`. Up:

```sql
ALTER TABLE orders DROP CONSTRAINT orders_status_check;

UPDATE orders SET status = CASE
    WHEN status = 'new' THEN 'awaiting_payment'
    WHEN status IN ('confirmed', 'preparing') THEN 'processing'
    WHEN status = 'ready' AND delivery_type = 'pickup' THEN 'ready_for_collection'
    WHEN status = 'ready' THEN 'out_for_delivery'
    ELSE status
END;

ALTER TABLE orders
    ALTER COLUMN status SET DEFAULT 'awaiting_payment',
    ADD CONSTRAINT orders_status_check CHECK (status IN (
        'awaiting_payment', 'payment_claimed', 'processing',
        'ready_for_collection', 'out_for_delivery', 'completed', 'cancelled')),
    ADD COLUMN public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    ADD COLUMN payment_claimed_at TIMESTAMPTZ,
    ADD COLUMN payment_confirmed_at TIMESTAMPTZ;
```

     Check the real constraint name first (`\d orders` in psql). Postgres
     names an inline column CHECK `orders_status_check`. Down: drop the
     three columns and the constraint, map the statuses back
     (`awaiting_payment`/`payment_claimed` → `new`, `processing` →
     `preparing`, `ready_for_collection`/`out_for_delivery` → `ready`),
     then restore the original CHECK and the `'new'` default.

1.2. `apps/api/Cargo.toml`: add `"uuid"` to `sqlx`'s features (risk 6).

1.3. New `apps/api/src/payment.rs`:
     - `BankDetails { account_name, iban, bic: Option<String> }` and a
       `PaymentConfig { bank: Option<Arc<BankDetails>> }` with
       `from_env()`. It uses the same `non_blank` closure pattern as
       `email.rs` and logs `warn!("bank details not set: invoices will show the WhatsApp fallback")`
       when the details are missing (Decision 5).
     - `pub fn payment_reference(order_id: i64) -> String { format!("ZTM-{order_id}") }`
       (Decision 6).
     - `#[derive(Serialize)] PaymentInstructions { account_name, iban, bic, reference }`
       and `PaymentConfig::instructions(order_id) -> Option<PaymentInstructions>`.

1.4. `PUBLIC_SITE_URL`: read it in `EmailConfig::from_env` as an
     *optional* field (`site_url()` falls back to `http://localhost:3000`),
     so missing it never turns email off (Decision 10).

1.5. `apps/api/src/lib.rs`: add `payment: payment::PaymentConfig` to
     `AppState` plus a `FromRef` impl. Build it in `main.rs` next to
     `EmailConfig::from_env()`. Update the test `AppState` construction in
     `apps/api/tests/common.rs`.

1.6. Env wiring:
     - `.env.example`: `BANK_ACCOUNT_NAME=`, `BANK_IBAN=`, `BANK_BIC=`,
       `PUBLIC_SITE_URL=http://localhost:3000` with a short comment.
     - `docker-compose.yml` `api.environment`: `BANK_*: ${BANK_*:-}`, the
       same as the SES vars, and `PUBLIC_SITE_URL: ${PUBLIC_SITE_URL:-http://localhost:3000}`.
     - `docker-compose.prod.yml` `api.environment`:
       `PUBLIC_SITE_URL: https://${DOMAIN}`.

## Group 2 — Lifecycle module (API)

Depends on: Group 1.

2.1. New `apps/api/src/order_status.rs`. This is the only source of truth
     for Decisions 1 and 3:

```rust
pub const ORDER_STATUSES: [&str; 7] = [
    "awaiting_payment", "payment_claimed", "processing",
    "ready_for_collection", "out_for_delivery", "completed", "cancelled",
];

/// Statuses an admin may move an order to next. `payment_claimed` is
/// never here: only the customer's "I've paid" sets it (Decision 3).
pub fn admin_next(status: &str, delivery_type: &str) -> &'static [&'static str] {
    match (status, delivery_type) {
        ("awaiting_payment" | "payment_claimed", _) => &["processing", "cancelled"],
        ("processing", "pickup") => &["ready_for_collection", "cancelled"],
        ("processing", _) => &["out_for_delivery", "cancelled"],
        ("ready_for_collection" | "out_for_delivery", _) => &["completed", "cancelled"],
        _ => &[],
    }
}
```

     Add `#[cfg(test)]` unit tests covering every pair in the
     requirement.md Decision 3 table, both allowed and rejected.

2.2. `apps/api/src/error.rs`: an `InvalidStatusTransition` variant →
     `409 {"error":"invalid_status_transition"}`, or reuse the existing
     `Conflict(json!(...))` if it already gives that shape.

## Group 3 — Endpoints & emails (API)

Depends on: Group 2.

3.1. `apps/api/src/routes/admin/orders.rs`:
     - Remove the local `ORDER_STATUSES` and use
       `order_status::ORDER_STATUSES`.
     - `update_order_status`: inside a transaction,
       `SELECT status, delivery_type ... FOR UPDATE`. Reject a value that
       isn't a known status (`422`, as today), and reject anything not in
       `admin_next(...)` (`409`). Then
       `UPDATE ... SET status = $2, updated_at = now(), payment_confirmed_at = CASE WHEN $2 = 'processing' THEN now() ELSE payment_confirmed_at END`.
     - Add `payment_reference`, `payment_claimed_at` and
       `payment_confirmed_at` to `AdminOrderResponse`. Add
       `payment_reference` to `OrderSummaryResponse`.
     - `orders_summary` seeds all seven counts.

3.2. `apps/api/src/routes/orders.rs`:
     - `OrderResponse` gains `public_token: Uuid`, `payment_reference`,
       and `payment: Option<PaymentInstructions>`. `create_order` gets
       `public_token` back from its `INSERT ... RETURNING`.
     - Replace the customer email body with the invoice (Decision 8) by
       calling a new helper, `invoice_email_body(...)`, so the claim email
       and any later email can't drift from it.
     - Add the owner new-order email (Decision 9). It's sent only when
       `email.business_notify_address()` is `Some`, the same pattern the
       enquiry emails use.
     - New `get_order_by_token(Path(token): Path<Uuid>)`: returns
       `OrderResponse` (items included). A malformed UUID is rejected by
       the extractor. Map that rejection to `404`, not `400`, so a bad
       token and an unknown one look the same.
     - New `claim_payment(Path(token))` (Decision 7):

```sql
UPDATE orders
SET status = 'payment_claimed', payment_claimed_at = now(), updated_at = now()
WHERE public_token = $1 AND status = 'awaiting_payment'
RETURNING id
```

       If a row came back, email the owner. If none did, load the order:
       `404` if it doesn't exist, `409` if it's `cancelled`, otherwise
       return it unchanged with `200`.

3.3. `apps/api/src/routes/mod.rs`:

```rust
.route("/orders/by-token/{token}", get(orders::get_order_by_token))
.route(
    "/orders/by-token/{token}/payment-claim",
    post(orders::claim_payment).route_layer(crate::rate_limit::enquiry_rate_limiter()),
)
```

3.4. Tests. Update `apps/api/tests/order_submission.rs` (new default
     status, `public_token`/`payment_reference`/`payment` present) and
     `apps/api/tests/admin_orders.rs` (seven statuses, the transition
     table over HTTP, `payment_confirmed_at` set on confirm, pickup order
     → `out_for_delivery` returns 409). Add
     `apps/api/tests/order_payment_claim.rs`: claim → 200 and
     `payment_claimed`; a second claim → 200 and unchanged; cancelled →
     409; random UUID → 404; `not-a-uuid` → 404.

3.5. `cargo sqlx prepare` so the offline `.sqlx/` cache matches the new
     queries (the Docker build uses it).

## Group 4 — Shared types & invoice component (web)

Depends on: Group 3 (response shape).

4.1. `apps/web/lib/orders.ts`: add `PaymentInstructions`, and give
     `OrderResponse` the new fields `public_token: string`,
     `payment_reference: string`, `payment: PaymentInstructions | null`
     and `status: OrderStatus`. Move `ORDER_STATUSES`/`OrderStatus` here
     from `lib/admin-orders.ts` (public code now needs them too), and
     re-export them from `admin-orders.ts` so admin imports keep working.
     Add the TS mirror:

```ts
// Mirrors admin_next() in apps/api/src/order_status.rs. Only used to
// choose which buttons to show; the API enforces it (Phase 20 Decision 3).
export function adminNextStatuses(status: OrderStatus, deliveryType: DeliveryType): OrderStatus[] { ... }
```

4.2. New `apps/web/components/order-invoice.tsx` (server-compatible, no
     `"use client"`). It shows the order number and date, an
     `OrderLineSummary`, the total, pickup or delivery plus the address,
     then a "How to pay" block: account name, IBAN (grouped in fours),
     BIC if set, and the **reference** in a visible box. Below that is a
     `Notice` with the "only verified payments" line. If `payment` is
     `null`, it shows the WhatsApp fallback through `buildWhatsAppLink`.
     No VAT line.

4.3. New `apps/web/components/payment-claim-button.tsx` (`"use client"`).
     It POSTs to `/api/orders/{token}/payment-claim`. On success it shows
     "Thanks, we'll check your payment and confirm shortly", then
     `router.refresh()`. On error it shows a `Notice`. Show it only while
     the status is `awaiting_payment`. For `payment_claimed`, show the
     "We're checking your payment" text in its place.

4.4. New proxy route `apps/web/app/api/orders/[token]/payment-claim/route.ts`,
     copied from the shape of `app/api/orders/route.ts`, forwarding to
     `${API_BASE_URL}/api/orders/by-token/${token}/payment-claim`. Before
     forwarding, check the token against a UUID regex. Anything else gets
     a `404` without calling the API.

## Group 5 — Public pages (web)

Depends on: Group 4.

5.1. `apps/web/components/order-success.tsx`: rewrite it around
     `OrderInvoice` and `PaymentClaimButton`. The heading changes from
     "Order #{id} Confirmed" to "Order #{id} received — please pay to
     confirm". Add a "Save this link to check or pay later" line pointing
     to `/order/{public_token}` (the next-intl `Link`).
     `OrderView`/`sessionStorage` recovery stay as they are. Bump
     `LAST_ORDER_KEY` to `:v2` so a stored v1 order (no token) is never
     shown.

5.2. New `apps/web/app/[locale]/order/[token]/page.tsx` (server
     component): `fetch(`${API_BASE_URL}/api/orders/by-token/${token}`, { cache: "no-store" })`.
     Call `notFound()` on 404. Render `OrderInvoice`, a status line
     (`t("status.<value>")`), and `PaymentClaimButton`. Its
     `generateMetadata` returns `robots: { index: false, follow: false }`
     and `referrer: "no-referrer"` (Decision 4).

5.3. `apps/web/app/robots.ts`: add `"/*/order/"` to `disallow`. This
     matches `/en/order/<token>` but not the `/en/order` checkout page.
     Confirm the sitemap doesn't list token pages (it shouldn't, since
     they're dynamic).

5.4. `apps/web/messages/en.json` and `nl.json`, `Order` namespace: the
     new success heading/body, `howToPay`, `accountName`, `iban`, `bic`,
     `reference`, `referenceHint` ("Use this exact reference so we can
     match your payment"), `verifiedOnly`, `noBankDetails`, `iPaid`
     ("I've paid"), `claiming`, `claimThanks`, `claimError`, `saveLink`,
     and `status.{seven values}`. Keep the Dutch natural ("Ik heb betaald",
     "Betalingskenmerk"). Remove the old `successBody` ("we'll contact
     you by phone…").

## Group 6 — Admin (web)

Depends on: Group 4.1.

6.1. `apps/web/lib/admin-orders.ts`: add `STATUS_LABELS: Record<OrderStatus, string>`
     ("Awaiting payment", "Payment claimed", "Processing", "Ready for
     collection", "On the way", "Completed", "Cancelled"). Add
     `payment_reference` to `OrderSummary`, and `payment_reference`,
     `payment_claimed_at` and `payment_confirmed_at` to `AdminOrder`.

6.2. `apps/web/components/admin-status-badge.tsx`: give all seven
     statuses classes. `payment_claimed` uses the most eye-catching one
     (`bg-accent-light text-primary-dark`), and the badge shows
     `STATUS_LABELS[status]` without `capitalize`.

6.3. Replace `admin-order-status-select.tsx` with
     `apps/web/components/admin-order-actions.tsx`. It renders
     `adminNextStatuses(status, deliveryType)` as buttons, all using the
     existing `PATCH /api/admin/orders/{id}/status` proxy. Labels per
     target: `processing` → "Confirm payment", `ready_for_collection` →
     "Ready for collection", `out_for_delivery` → "Mark on the way",
     `completed` → "Mark completed". `cancelled` is a secondary button
     that first changes to "Confirm cancel?" (Decision 12). A `409`
     shows "This order has moved on, refresh the page". Terminal statuses
     show no actions. Delete the old select component and update its
     import in `orders/[id]/page.tsx`.

6.4. `apps/web/app/admin/(protected)/orders/[id]/page.tsx`: show the
     payment reference, and the "Payment claimed" and "Payment confirmed"
     timestamps when they're set, with the existing Europe/Amsterdam
     formatter.

6.5. `apps/web/app/admin/(protected)/page.tsx` (dashboard): order the
     tiles as `payment_claimed`, `awaiting_payment`, `processing`, …, and
     add a "needs action" style to `payment_claimed` when its count is
     above 0. `orders/page.tsx`'s filter uses `STATUS_LABELS`.

## Group 7 — Validate, configure, deploy

Depends on: Groups 1–6.

7.1. `/validate` (lint, build, smoke test), plus `cargo test` in
     `apps/api`.

7.2. Get the bank details from the client (risk 1), along with whether
     they want their KvK number on the invoice (risk 2).

7.3. Before merging: look at production's current orders (risk 5). This
     read-only query previews exactly what the migration will do to each
     order. Run it on the server through
     `docker compose -f docker-compose.yml -f docker-compose.prod.yml exec postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB"`:

```sql
SELECT id, created_at::date, customer_name, delivery_type, total, status AS old_status,
       CASE
           WHEN status = 'new' THEN 'awaiting_payment'
           WHEN status IN ('confirmed', 'preparing') THEN 'processing'
           WHEN status = 'ready' AND delivery_type = 'pickup' THEN 'ready_for_collection'
           WHEN status = 'ready' THEN 'out_for_delivery'
           ELSE status
       END AS new_status
FROM orders
WHERE status NOT IN ('completed', 'cancelled')
ORDER BY id;
```

     Any `new` order that was already handled by phone should be moved to
     `completed` or `cancelled` in `/admin` **before** the merge, while
     the old free dropdown still allows it. After the merge, the guided
     buttons can't jump straight there.

     Add `BANK_ACCOUNT_NAME`/`BANK_IBAN`/`BANK_BIC` (and `BUSINESS_KVK`
     if the client wants it, risk 2) to the server's `.env`
     (`PUBLIC_SITE_URL` comes from prod compose). Take a manual Lightsail
     snapshot, because this migration rewrites data.

7.4. Merge to `master` (CI deploys, and the API runs migrations at
     startup). Then run validation.md's production checks.
