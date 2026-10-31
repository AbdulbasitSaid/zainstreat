# Phase 22 — Customer Order Tracking & Status Emails: Implementation Plan

**Ordering:**
- Group 0 is a hard gate. No repo group starts until Phase 20 is on
  `master`.
- API groups (1–4) come before web groups (5–7), because the web types
  mirror the new API fields.
- Run `cargo test` after Groups 2, 3 and 4. Run `/validate` after
  Group 8.

Paths refer to the tree after the Phase 20 merge.

## Group 0 — Preconditions (ops)

Depends on: nothing.

0.1. **Phase 20 is merged into `master`** (PR #35, 2026-10-31). Done.
0.2. **Merge Phase 21 Group 13.** It's on branch
     `2026-10-31-phase-21-group-13` (worktree `../zainstreat-merge-p20`),
     uncommitted:
     - `PUBLIC_ADMIN_URL` in `EmailConfig` and both compose files
     - owner email links set to `{admin_url}/orders/{id}`
     - `BANK_*` and `BUSINESS_KVK` in `_deploy.yml` and `.env.example`
     - Phase 20's phase references renumbered, and its roadmap paragraph
       restored
     - Phase 20's "needs action" `payment_claimed` dashboard tile
       restored (PR #35's conflict resolution dropped it)

     Review it, commit, open the PR and merge it. Then `git worktree
     remove ../zainstreat-merge-p20`. Production's `BANK_*` variables
     are still waiting on the client; staging's are set.
0.3. **Production email (requirement.md Decision 2).** Done during
     planning: the SES identity and DNS records, the IAM SMTP user with
     its key in SSM, `SES_SMTP_HOST` and `EMAIL_FROM_ADDRESS` on both
     Environments, and staging's notify address. Remaining:
     - Set `SES_SMTP_USERNAME`/`SES_SMTP_PASSWORD` on both Environments.
       The planning session gave the user a one-off script for this. It
       reads `/zainstreat/ses-smtp/*` from SSM, derives the SES SMTP
       password locally (SigV4 HMAC of `SendRawEmail`, version byte
       `0x04`), and pipes both into `gh secret set`. Check with `gh secret
       list --env production`.
     - `aws sesv2 get-email-identity --email-identity zainstreat.com
       --region eu-central-1` shows `"DkimAttributes": {"Status":
       "SUCCESS"}`.
     - Request SES production access (SES console → Account dashboard →
       Request production access: transactional, order confirmations and
       status updates, website `https://zainstreat.com`).
     - Set production's `EMAIL_BUSINESS_NOTIFY_ADDRESS` to the owner's
       inbox. Until it's set, `EmailConfig` keeps **all** production
       email off.
     - Send a test order on staging. In the sandbox the customer address
       must be a verified SES identity, so use `techsihir@gmail.com`
       after verifying it in SES.
0.4. **Rebase this branch onto the new `master`.**
     `git log master..HEAD` should show only the Phase 22 spec commits.
0.5. **Check that the base is right:** `ls
     apps/api/migrations/*order_payment_lifecycle*` exists, and
     `grep -rn '/admin/' apps/api/src | grep -v '/api/admin'` returns
     nothing.

## Group 1 — Migration (repo)

Depends on: Group 0.

1.1. **`sqlx migrate add -r order_tracking`** in `apps/api`. The up
     migration:

```sql
-- Phase 22: step timestamps for the tracking page, who completed the
-- order, and the customer's locale for status emails (requirement.md
-- Decisions 4, 5, 8).
ALTER TABLE orders
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en'
        CONSTRAINT orders_locale_check CHECK (locale IN ('en', 'nl')),
    ADD COLUMN ready_at TIMESTAMPTZ,
    ADD COLUMN completed_at TIMESTAMPTZ,
    ADD COLUMN cancelled_at TIMESTAMPTZ,
    ADD COLUMN completed_by TEXT
        CONSTRAINT orders_completed_by_check
        CHECK (completed_by IN ('customer', 'admin'));
```

     The down migration drops the five columns. That loses the data, but
     the result is valid.
1.2. **Update `.sqlx`:** after Groups 2–4 change queries, run `cargo sqlx
     prepare` so the offline release build still compiles.

## Group 2 — Status helpers and stamping (repo)

Depends on: Group 1.

2.1. **`apps/api/src/order_status.rs`:** add

```rust
/// Statuses from which the customer may confirm receipt (Phase 22
/// Decision 8).
pub const CUSTOMER_COMPLETABLE: &[&str] = &["ready_for_collection", "out_for_delivery"];

pub fn customer_can_complete(status: &str) -> bool {
    CUSTOMER_COMPLETABLE.contains(&status)
}
```

     Unit-test it next to the existing `admin_next` tests: true for both
     ready statuses, false for all others.
2.2. **Stamp in `update_order_status`** (`routes/admin/orders.rs`).
     Extend the existing `UPDATE` with `CASE` stamps alongside the
     `payment_confirmed_at` one:
     - `ready_at = now()` for `ready_for_collection` and
       `out_for_delivery`
     - `completed_at = now()` and `completed_by = 'admin'` for
       `completed`
     - `cancelled_at = now()` for `cancelled`

     Add `customer_email`, `public_token` and `locale` to `RETURNING`,
     using a separate row struct if `OrderSummaryRow` can't be extended
     cleanly. The response stays `OrderSummaryResponse`.
2.3. **`OrderResponse`** (`routes/orders.rs`): add
     `payment_claimed_at`, `payment_confirmed_at`, `ready_at`,
     `completed_at`, `cancelled_at` (`Option<DateTime<Utc>>`) and
     `completed_by: Option<String>`. Select them in `load_order_by_token`
     and in the create path.
2.4. **Admin order detail** (`GET /api/admin/orders/{id}`): also return
     `ready_at`, `completed_at`, `cancelled_at` and `completed_by`.
2.5. **Reopen a customer-completed order** (requirement.md Decision 8):
     - `admin_next(status, delivery_type, completed_by: Option<&str>)`:
       for `completed` with `completed_by == Some("customer")`, return
       the order's ready status (`ready_for_collection` or
       `out_for_delivery`). Otherwise it's unchanged (terminal). Update
       every caller, including the `next_statuses` in the admin detail
       response.
     - The locked `SELECT … FOR UPDATE` also reads `completed_by`.
     - Moving `completed` → ready status sets `completed_at = NULL,
       completed_by = NULL` and keeps `ready_at`.
     - The handler remembers that this move was a reopen and **skips**
       the customer email for it (Group 3.3).
     - Unit tests: a customer-completed order offers its ready status;
       an admin-completed order offers nothing.
2.6. **Tests:** extend `tests/admin_orders.rs`.
     - Walk one pickup order and one delivery order to `completed`, and
       assert each timestamp is set once and `completed_by = 'admin'`.
     - Cancel another order and assert `cancelled_at`.
     - Reopen: a customer-completed order PATCHed back to
       `ready_for_collection` returns `200` with `completed_*` cleared.
       The same PATCH on an admin-completed order returns `409`.

## Group 3 — Localized customer emails (repo)

Depends on: Group 2.

3.1. **New `apps/api/src/order_emails.rs`.** Register it in `lib.rs`.
     It's plain functions with no I/O:

```rust
#[derive(Clone, Copy)]
pub enum Locale { En, Nl }

impl Locale {
    pub fn parse(s: &str) -> Option<Self> { /* "en" | "nl" */ }
    pub fn as_str(self) -> &'static str { /* ... */ }
}

pub fn tracking_url(site_url: &str, locale: Locale, token: Uuid) -> String {
    format!("{site_url}/{}/order/{token}", locale.as_str())
}

/// Subject and body for the email sent on an admin status change, or
/// `None` for statuses that don't email the customer (Decision 6).
pub fn status_email(locale: Locale, order_id: i64, status: &str,
                    delivery_type: &str, tracking_url: &str)
    -> Option<(String, String)>;

/// Phase 20's invoice email, moved here and localized (Decision 7).
pub fn invoice_email(locale: Locale, /* existing inputs */) -> (String, String);
```

     Copy per status follows requirement.md Decision 6. Every body
     includes the order number (`#N`) and the tracking link. The
     cancellation body points to the WhatsApp number the site already
     uses (`NEXT_PUBLIC_WHATSAPP_NUMBER` isn't visible to the API, so
     either pass a `WHATSAPP_NUMBER` env var through `EmailConfig` or say
     "reply to this email". Pick one at implementation time and note it
     in the implementation notes).
3.2. **Move `invoice_email_body`** out of `routes/orders.rs` into
     `invoice_email`, keeping the English text exactly the same, and add
     the Dutch version. `create_order` uses the order's locale and
     `tracking_url(...)` instead of the unprefixed `/order/{token}`.
3.3. **Send from `update_order_status`.** Add `State(email):
     State<EmailConfig>`. After `tx.commit()`:

```rust
// A reopen (Group 2.5) moves back to a ready status silently.
if !reopened {
    let locale = Locale::parse(&row.locale).unwrap_or(Locale::En);
    let url = tracking_url(email.site_url(), locale, row.public_token);
    if let Some((subject, body)) =
        status_email(locale, row.id, &row.status, &row.delivery_type, &url)
    {
        email.spawn_send(row.customer_email.clone(), subject, body);
    }
}
```

3.4. **Unit tests** (`#[cfg(test)]` in `order_emails.rs`):
     - each emailing status × `en`/`nl` returns `Some`, and the body
       contains `#N` and the URL
     - `payment_claimed` and `awaiting_payment` return `None`
     - the `en` invoice text matches the pre-move text
     - `tracking_url` has no double slash

## Group 4 — Customer "received" endpoint (repo)

Depends on: Group 2.

4.1. **Handler `confirm_received`** in `routes/orders.rs`, modelled on
     `claim_payment`:

```sql
UPDATE orders
SET status = 'completed', completed_at = now(), completed_by = 'customer'
WHERE public_token = $1
  AND status IN ('ready_for_collection', 'out_for_delivery')
RETURNING id
```

     - A row comes back: return the reloaded order.
     - No row: load by token. Not found → `404`. `completed` → `200`
       with the order. Anything else → `AppError::InvalidStatusTransition`
       (`409`).
     - Bind the status list from `order_status::CUSTOMER_COMPLETABLE`
       (`= ANY($2)`) so it isn't written out twice.
     - No email.
4.2. **Route** in `routes/mod.rs`: `POST /orders/by-token/{token}/received`
     with `.route_layer(crate::rate_limit::enquiry_rate_limiter())`, the
     same way as `payment-claim`.
4.3. **Integration tests, `tests/order_received.rs`** (follow
     `tests/order_payment_claim.rs`):
     - pickup order moved to `ready_for_collection` → `200`, completed,
       `completed_by = 'customer'`
     - second call → `200`, `completed_at` unchanged
     - order in `processing` → `409`
     - cancelled → `409`
     - random UUID → `404`; `not-a-uuid` → `404`
     - six quick calls from one `X-Forwarded-For` → a `429`
4.4. **Locale on create** (`CreateOrderRequest.locale: Option<String>`):
     a missing value is `en`; an unsupported value returns the existing
     `400 validation_error` field-error shape with `field: "locale"`. Insert it into
     `orders.locale`. Add cases to `tests/order_submission.rs`: `nl` is
     stored, missing → `en`, `de` → `400`.

## Group 5 — Web: types and checkout locale (repo)

Depends on: Groups 2 and 4.

5.1. **`apps/web/lib/orders.ts`:**
     - Add `locale?: string` to `CreateOrderRequest`.
     - Add the new nullable timestamp fields and `completed_by: "customer"
       | "admin" | null` to `OrderResponse`. No `any`; narrow in the
       existing type guards if `OrderResponse` is validated there.
5.2. **Checkout** (the component that calls `submitOrder`): pass
     `locale: useLocale()`.
5.3. **`lib/admin-orders.ts`:** add the same fields to the admin order
     detail type.

## Group 6 — Web: tracking page (repo)

Depends on: Group 5.

6.1. **`components/order-progress.tsx`** is a server-compatible component
     (no state). It takes the order and builds the step list for its
     `delivery_type` (requirement.md Decision 3):
     - Each step is done, current or upcoming, with the time formatted
       via next-intl's `useFormatter().dateTime`.
     - Cancelled orders render a single cancelled state.
     - Use an `<ol>` with `aria-current="step"` on the current step.
6.2. **`components/order-received-button.tsx`** (`"use client"`), shaped
     like `payment-claim-button.tsx`:
     - It renders `null` unless the status is `ready_for_collection` or
       `out_for_delivery`.
     - The first click shows "Yes, I've got it" / "Cancel".
     - The second click POSTs `/api/orders/${token}/received`, then calls
       `router.refresh()`.
     - A `409` shows the "order has moved on, refresh" message.
6.3. **`app/api/orders/[token]/received/route.ts`:** a copy of the
     payment-claim proxy (`isOrderToken` guard, forwards
     `X-Forwarded-For`).
6.4. **`components/order-auto-refresh.tsx`** (`"use client"`):

```tsx
export function OrderAutoRefresh() {
  const router = useRouter();
  useEffect(() => {
    const tick = () => {
      if (document.visibilityState === "visible") router.refresh();
    };
    const id = window.setInterval(tick, 60_000);
    document.addEventListener("visibilitychange", tick);
    return () => {
      window.clearInterval(id);
      document.removeEventListener("visibilitychange", tick);
    };
  }, [router]);
  return null;
}
```

6.5. **`app/[locale]/order/[token]/page.tsx`:**
     - Replace the plain status line with `<OrderProgress>`.
     - Render `<OrderReceivedButton>` beside `<PaymentClaimButton>`.
     - Render `<OrderAutoRefresh />` only when the status is neither
       `completed` nor `cancelled`.
     - Keep `generateMetadata` (`noindex`, `no-referrer`) as it is.

## Group 7 — Web: admin auto-refresh (repo)

Depends on: Group 5.

7.1. **`components/admin-auto-refresh.tsx`** (`"use client"`). Props:
     `latestOrderId: number | null`, `claimedIds: number[]`.
     - `router.refresh()` every 30s whether or not the tab is visible.
       It also refreshes on `window` `focus`.
     - It keeps a baseline (`latestOrderId` and `claimedIds` from when
       the tab last had focus) in a `useRef`. On each new props value it
       computes `n = (#ids > baseline.latestOrderId) + (#claimedIds not in
       baseline)`.
     - It sets `document.title` to `(n) ${base title}` when `n > 0`. On
       `focus` it clears the badge and resets the baseline.
     - The baseline is mirrored to `sessionStorage`
       (`admin-badge-baseline`), with every read and write in
       `try/catch`, so a reload keeps the count (requirement.md Decision
       9). If storage fails, it falls back to the mount-time baseline.
     - It exposes the "new since baseline" ids through a
       `data-new-order-ids` attribute or a small context so the row
       components can highlight them. Pick the simpler one when
       implementing; a React context provider wrapping the list is the
       expected fit.
7.2. **Mount it** on `app/admin/(protected)/page.tsx` (dashboard) and
     `app/admin/(protected)/orders/page.tsx`. Pass the max id from the
     data the page already fetches. The claimed ids come from the
     dashboard's existing `payment_claimed` list, or from a
     `status=payment_claimed` filter on the orders list's existing query.
     If neither page has it cheaply, add `claimed_ids` to the existing
     `/api/admin/orders/summary` response rather than adding a new
     endpoint.
7.3. **Row highlight:** new rows get the same "needs action" accent
     Phase 20 uses for `payment_claimed`. No new colour token.
7.4. **Order detail** (`orders/[id]/page.tsx`): show the ready, completed
     and cancelled times next to the claimed/confirmed times Phase 20
     already shows, and "Completed by customer" or "Completed by admin".
     For a customer-completed order, `AdminOrderActions` renders the
     reopen entry from `next_statuses` as "Reopen (customer marked it
     received)", a secondary button with the same second-click confirm as
     cancel.

## Group 8 — i18n and validation (repo)

Depends on: Groups 6 and 7.

8.1. **`messages/en.json` and `nl.json`:** add an `Order.tracking`
     namespace: the step labels (pickup and delivery variants),
     "Cancelled", the received-button label for each delivery type, its
     confirm/cancel text, and the "moved on" error. Admin copy stays
     English inline, as in Phase 20.
8.2. **Run `cargo test`** (with `.sqlx` re-prepared) and `/validate`.
     Then tick `validation.md`.
8.3. **Client copy review on staging** (requirement.md Risk 4). Deploy
     to staging and walk one pickup order and one delivery order through
     every emailing status, in both `en` and `nl`. Forward the emails
     (and a tracking-page screenshot) to the client. Apply their wording
     changes in `order_emails.rs` and `messages/*.json` **before** the
     production promote.

## Implementation notes (2026-10-31)

- **Group 0.** `2026-10-31-phase-21-group-13` hadn't merged when work
  started, so its diff was applied to this branch's working tree too
  (`git apply --3way`, then unstaged). The changes are identical, so a
  rebase after that PR merges treats them as already applied.
- **Validation status code.** Field errors in this API are `400
  validation_error`, not `422`; the locale check follows the existing
  shape. requirement.md and this plan are corrected.
- **`status_email` signature.** It takes the customer's name and the
  payment reference instead of the order id and `delivery_type`. The
  status already tells collection from delivery apart, and the emails
  greet the customer and quote `ZTM-N` like the invoice.
- **Cancellation contact (3.1).** It links to `{site_url}/{locale}/contact`,
  which has the WhatsApp button. That needs no new env var; the
  WhatsApp number lives only in `apps/web/lib/whatsapp.ts`.
- **Reopen (2.5).** A new `order_status::is_reopen(from, to)` replaces a
  handler-local flag. The UPDATE takes it as `$3` to clear
  `completed_*` and keep `ready_at`.
- **Admin badge (7.1–7.2).** `/api/admin/orders/summary` gained
  `claimed_ids`, which both admin pages fetch (the orders list now also
  loads the summary). The newest order id comes from the summary's
  `recent`, so the badge counts at most the 5 newest orders since the
  baseline. That's fine for a "something arrived" signal. The baseline is
  read with `useSyncExternalStore` (the `order-view.tsx` pattern), with
  an in-memory copy when `sessionStorage` is blocked. Rows highlight
  through a small context and the `NewOrderHighlight` wrapper.
- **Tracking page layout.** Stepper first, then the "I've paid" and
  "received" buttons, then the invoice. The old plain status line is
  gone. The stepper's progress comes from the status, not the timestamps,
  so orders from before this phase still show the right step, just
  without times.
- **Results.** `cargo test` 159 passed, 0 failed (138 before). Clippy is
  clean, the offline release build passes, web lint and build pass, and
  the `/validate` live smoke test passes.
