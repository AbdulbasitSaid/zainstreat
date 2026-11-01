# Phase 23 — Catering Quotations: Implementation Plan

**Ordering:**
- API groups (1–5, including 2b) come before web groups (6–9, including
  6b), because the web types mirror the new API fields.
- Run `cargo test` after Groups 3, 4 and 5. Run `/validate` after
  Group 10.

Paths refer to `master` at `64c453a` (Phase 22 merged).

## Group 0 — Preconditions (ops)

Depends on: nothing.

0.1. **Phase 22 is on `master`** (PR #36). Done.
0.2. **Email works on staging** (Phase 22 Group 0): no `email sending
     disabled` warning in staging's API logs. While SES is in the sandbox,
     the test enquiry's email must be a verified identity.
0.3. **Client copy sign-off** (requirement.md Decision 15). Send
     `copy.md` to the client. Apply their changes to `copy.md` before
     Groups 2b, 5, 7 and 9 write any strings. Note the date it was
     approved at the top of `copy.md`. *Done 2026-11-01: the project
     owner approved all copy, Dutch included.*

## Group 1 — Migration (repo)

Depends on: Group 0.

1.1. **`sqlx migrate add -r catering_quotes`** in `apps/api`. Up:

```sql
-- Phase 23: catering quotations (requirement.md Decisions 1-7).
ALTER TABLE catering_enquiries
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en'
        CONSTRAINT catering_enquiries_locale_check CHECK (locale IN ('en', 'nl'));

CREATE TABLE catering_quotes (
    id BIGSERIAL PRIMARY KEY,
    enquiry_id BIGINT NOT NULL UNIQUE REFERENCES catering_enquiries(id) ON DELETE RESTRICT,
    public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'sent', 'accepted', 'declined')),
    delivery_type TEXT NOT NULL CHECK (delivery_type IN ('pickup', 'delivery', 'on_site')),
    delivery_address TEXT,
    notes TEXT,
    total NUMERIC(10, 2) NOT NULL CHECK (total > 0),
    valid_until DATE NOT NULL,
    -- Bumped on every admin save; accept/decline must echo it (Decision 13).
    version INTEGER NOT NULL DEFAULT 1,
    sent_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ,
    declined_at TIMESTAMPTZ,
    decline_reason TEXT,
    order_id BIGINT UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT catering_quotes_order_when_accepted
        CHECK ((status = 'accepted') = (order_id IS NOT NULL))
);

CREATE TABLE catering_quote_items (
    id BIGSERIAL PRIMARY KEY,
    quote_id BIGINT NOT NULL REFERENCES catering_quotes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    description TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10, 2) NOT NULL CHECK (unit_price >= 0),
    subtotal NUMERIC(10, 2) NOT NULL,
    UNIQUE (quote_id, position)
);

-- On-site fulfilment for catering orders (requirement.md Decision 12).
-- The original delivery_type CHECK was inline, so Postgres named it
-- orders_delivery_type_check; confirm with \d orders before relying on it.
ALTER TABLE orders
    DROP CONSTRAINT orders_delivery_type_check,
    ADD CONSTRAINT orders_delivery_type_check
        CHECK (delivery_type IN ('pickup', 'delivery', 'on_site')),
    DROP CONSTRAINT orders_status_check,
    ADD CONSTRAINT orders_status_check CHECK (status IN (
        'awaiting_payment', 'payment_claimed', 'processing',
        'ready_for_collection', 'out_for_delivery', 'serving_on_site',
        'completed', 'cancelled'));
```

     Down: drop both tables, then restore the two `orders` CHECKs to
     their Phase 20 lists, then drop `catering_enquiries.locale`. Restoring
     the CHECKs fails if any `on_site` or `serving_on_site` orders exist.
     That's acceptable: a down migration is for dev databases.
1.2. **`.sqlx`:** run `cargo sqlx prepare` after Groups 2–5 change
     queries.

## Group 2 — Quote module (repo)

Depends on: Group 1.

2.1. **New `apps/api/src/quote_status.rs`** (register in `lib.rs`),
     pure functions next to `order_status.rs`:

```rust
/// Mirrors the CHECK on `catering_quotes.status`.
pub const QUOTE_STATUSES: [&str; 4] = ["draft", "sent", "accepted", "declined"];

/// The admin may save (and so revert to draft) anything not accepted
/// (Phase 23 Decision 3).
pub fn admin_can_edit(status: &str) -> bool { status != "accepted" }

/// `sent` and not past `valid_until` (Europe/Amsterdam date).
pub fn customer_can_accept(status: &str, valid_until: NaiveDate, today: NaiveDate) -> bool;

pub fn customer_can_decline(status: &str) -> bool { status == "sent" }

/// What the admin and the quote page show: the stored status, or
/// `expired` for a `sent` quote past `valid_until`.
pub fn display_status(status: &str, valid_until: NaiveDate, today: NaiveDate) -> &'static str;
```

     Unit tests cover every status × expired/not.
2.2. **"Today" in Amsterdam.** Get it from the DB in the same statement
     as the lock, as `orders_summary` does:
     `(now() AT TIME ZONE 'Europe/Amsterdam')::date AS "today!"`.
     Don't add `chrono-tz`.
2.3. **Add `AppError::QuoteExpired`** → `409 {"error": "quote_expired"}`
     and **`AppError::QuoteChanged`** → `409 {"error": "quote_changed"}`
     (Decision 13). Other illegal moves reuse `InvalidStatusTransition`
     (`409 invalid_status_transition`).
2.3b. **`EmailConfig::is_enabled(&self) -> bool`** (`email.rs`):
     `self.settings.is_some()`. It's used by `send_quote` (Decision 14).
2.4. **Add `tests/quote_status_schema.rs`**, a copy of
     `order_status_schema.rs` that checks `QUOTE_STATUSES` against the
     CHECK.

## Group 2b — On-site fulfilment in the order lifecycle (repo)

Depends on: Group 1. Groups 3 and 4 depend on it.

2b.1. **`order_status.rs`** (requirement.md Decision 12):
      - Add `"serving_on_site"` to `ORDER_STATUSES` (now `[&str; 8]`),
        after `out_for_delivery`.
      - `CUSTOMER_COMPLETABLE` becomes `["ready_for_collection",
        "out_for_delivery", "serving_on_site"]`.
      - Add these arms to `admin_next`, before the `_` delivery
        fallbacks:

```rust
("processing", "on_site", _) => &["serving_on_site", "cancelled"],
("ready_for_collection" | "out_for_delivery" | "serving_on_site", _, _) => &["completed", "cancelled"],
("completed", "on_site", Some("customer")) => &["serving_on_site"],
```

      - `is_reopen` needs no change: it already uses
        `customer_can_complete`.
      - Tests:
        - add `"on_site"` to every `for delivery_type in [...]` loop
        - `processing_branches_on_delivery_type` covers `on_site` →
          `serving_on_site` only
        - reopen goes to `serving_on_site`
        - `customer_can_complete_only_handed_over_orders` includes the
          new status
2b.2. **`routes/admin/orders.rs`:** the `ready_at` `CASE` (line ~292)
      becomes `IN ('ready_for_collection', 'out_for_delivery',
      'serving_on_site')`. The status filter on `list_orders` accepts the
      new status through `is_valid_status`, so no change there. Add a case
      to `tests/admin_orders.rs`: an `on_site` order (inserted directly in
      SQL) walks processing → serving_on_site → completed, with `ready_at`
      set.
2b.3. **`order_emails.rs`:**
      - `status_email` gains `serving_on_site` arms with the
        `en`/`nl` subject, message and link label from `copy.md` §3.
      - Add the status to the test's `EMAILING` list.
      - `invoice_email`: an `on_site` address renders as
        `{copy.served_at}: {address}` (`en` "Served at", `nl` "Locatie").
        The existing `delivery_to` arm is unchanged.
      - The owner's new-order email in `routes/orders.rs` (line ~384)
        only runs for cart orders, which are never `on_site`, so it's
        unchanged.
2b.4. **Cart stays pickup/delivery.** `routes/orders.rs`'s `DeliveryType`
      enum is unchanged. Add a case to `tests/order_submission.rs`:
      `delivery_type: "on_site"` is rejected (`4xx`, no row inserted).
2b.5. **Quote side.** Group 3 uses a separate
      `FulfilmentType { Pickup, Delivery, OnSite }` enum (serde
      `snake_case`) in `routes/admin/quotes.rs`, not the cart's
      `DeliveryType`. Change 3.1 accordingly: the address is required for
      `Delivery` and `OnSite`.

## Group 3 — Admin quote API (repo)

Depends on: Group 2.

New `apps/api/src/routes/admin/quotes.rs`. Routes in `admin/mod.rs`,
behind `require_admin`:

```rust
.route("/catering-enquiries/{id}/quote", put(quotes::save_quote))
.route("/catering-enquiries/{id}/quote/send", post(quotes::send_quote))
```

3.1. **`SaveQuoteRequest`:** `delivery_type: FulfilmentType` (Group
     2b.5), `delivery_address:
     Option<String>`, `notes: Option<String>`, `valid_until: NaiveDate`,
     `items: Vec<{ description: String, quantity: i32, unit_price: Decimal }>`.
     Validation returns `400 validation_error` with fields:
     - `items`: `empty`, or `too_many` for more than 50
     - `items[i].description`: `required`, or `too_long` for more than 200
       chars
     - `items[i].quantity`: 1–10,000
     - `items[i].unit_price`: 0–100,000.00 with at most 2 decimal places
       (`unit_price.scale() <= 2` after `normalize()`)
     - `delivery_address`: `required` or `too_short` (5) for delivery and
       on-site
     - `notes`: at most 1,000 chars
     - `valid_until`: `must_be_future` if before today (Amsterdam)
     - `total`: `must_be_positive` if the sum is 0
3.2. **`save_quote`** in one transaction:
     - `SELECT … FROM catering_enquiries WHERE id = $1`, else `404`.
     - Lock any existing quote with `SELECT id, status FROM catering_quotes
       WHERE enquiry_id = $1 FOR UPDATE`. If it's `accepted`, return `409
       invalid_status_transition`.
     - `INSERT … ON CONFLICT (enquiry_id) DO UPDATE SET …, status =
       'draft', decline_reason = NULL, declined_at = NULL, version =
       catering_quotes.version + 1, updated_at = now()` with the computed
       `total`.
     - `DELETE FROM catering_quote_items WHERE quote_id = $1`, then insert
       the lines with `position` 0..n and `subtotal = quantity *
       unit_price`, computed in Rust with `Decimal`.
     - Return `AdminQuoteResponse` (3.4).
3.3. **`send_quote`** in one transaction:
     - Lock the quote joined to its enquiry. No quote → `404`.
       `accepted` → `409`. `valid_until < today` → `400
       validation_error` with `field: "valid_until"`, `message:
       "must_be_future"`.
     - `UPDATE … SET status = 'sent', sent_at = now(), decline_reason =
       NULL, declined_at = NULL`.
     - After commit, `email.spawn_send(enquiry.email, quote_email(…))`
       (Group 5.1). Resending a `sent` quote is allowed and emails again.
     - Response: `{ "quote": AdminQuoteResponse, "email_queued":
       email.is_enabled() }` (Decision 14). `spawn_send` is
       fire-and-forget, so this means "handed to SES", not "delivered".
3.4. **`AdminQuoteResponse`:** `id`, `status`, `display_status`,
     `version`, `public_token`, `quote_url` (built with `quote_url()` in
     the enquiry's locale, so admin doesn't rebuild it), `delivery_type`, `delivery_address`, `notes`,
     `total` (string), `valid_until`, `sent_at`, `accepted_at`,
     `declined_at`, `decline_reason`, `order_id`, and `items` (`description`,
     `quantity`, `unit_price`, `subtotal`). Use the
     `rust_decimal::serde::str` attributes as `OrderResponse` does.
3.5. **Enquiry endpoints** (`routes/admin/enquiries.rs`):
     - `CateringEnquiryDetail` gains `locale` and `quote:
       Option<AdminQuoteResponse>`, loaded with a second query.
     - `CateringEnquirySummary` gains `quote_status: Option<String>` (the
       display status) through `LEFT JOIN catering_quotes`.
3.6. **Admin orders** (`routes/admin/orders.rs`):
     - `OrderSummaryResponse` and `AdminOrderResponse` gain
       `catering_enquiry_id: Option<i64>` via `LEFT JOIN catering_quotes q
       ON q.order_id = o.id`.
     - Nothing else changes; the lifecycle is untouched.
3.7. **Tests: new `tests/admin_quotes.rs`** (use `common.rs` login
     helpers):
     - Save → `200 draft`, total equals the line sum, lines are in order.
     - Save again with fewer lines → old lines are gone.
     - Validation: no items, a negative price, 3 decimal places, a
       delivery without an address, `valid_until` yesterday, and 51 lines.
     - Send → `sent` with `sent_at` set, and `email_queued: false` (SES
       is unset in tests); save after send → `draft`.
     - `version` is 1 after the first save and goes up by one on each
       save after that.
     - Send with no quote → `404`; an unknown enquiry → `404`.
     - Unauthenticated → `401` on both routes.
     - Enquiry detail includes `quote` and `locale`; the list includes
       `quote_status`.

## Group 4 — Public quote API (repo)

Depends on: Group 3.

New `apps/api/src/routes/quotes.rs`. Routes in `routes/mod.rs`:

```rust
.route("/quotes/by-token/{token}", get(quotes::get_quote_by_token))
.route(
    "/quotes/by-token/{token}/accept",
    post(quotes::accept_quote).route_layer(crate::rate_limit::enquiry_rate_limiter()),
)
.route(
    "/quotes/by-token/{token}/decline",
    post(quotes::decline_quote).route_layer(crate::rate_limit::enquiry_rate_limiter()),
)
```

     A malformed token is `404`, the same as `orders/by-token`.

4.0. **Request bodies.** Accept takes `{ "version": i32 }`. Decline
     takes `{ "version": i32, "reason": Option<String> }`. A missing
     body or field gets axum's JSON rejection, as with other bad JSON.
4.1. **`PublicQuoteResponse`:**
     - `id`, `version`, `display_status`, `customer_name`, `event_type`,
       `event_date`, `guest_count`
     - `delivery_type`, `delivery_address`, `notes`, `items`, `total`,
       `valid_until`
     - `order_token: Option<Uuid>`, from the order's `public_token` once
       accepted
     - No email or phone; the page doesn't need them.
4.2. **`accept_quote`** in one transaction:
     - Lock with `SELECT q.*, e.*, (now() AT TIME ZONE
       'Europe/Amsterdam')::date AS today FROM catering_quotes q JOIN
       catering_enquiries e … WHERE q.public_token = $1 FOR UPDATE OF q`.
       Not found → `404`.
     - `accepted` → commit and return `200` with the existing order
       token. This makes accept idempotent.
     - `version != body.version` → `QuoteChanged`. This check comes
       after the accepted check, so a retry stays idempotent, and before
       the status check, so a quote the owner reverted to draft also
       reads as "changed".
     - Not `customer_can_accept`: `QuoteExpired` if it's `sent` and past
       its date, otherwise `InvalidStatusTransition`.
     - `INSERT INTO orders (customer_name, customer_email,
       customer_phone, delivery_type, delivery_address, notes, subtotal,
       delivery_fee, total, locale) VALUES (…, 0, …) RETURNING id,
       public_token`. Notes per requirement.md Decision 7.
     - Copy the lines into `order_items` (`menu_item_id`,
       `price_option_id` and `option_label` all `NULL`) in `position`
       order.
     - `UPDATE catering_quotes SET status = 'accepted', accepted_at =
       now(), order_id = $2`.
     - After commit:
       - Load the order with the existing `load_order_by_token` (make it
         `pub(crate)` if needed).
       - Send the customer `invoice_email` in the enquiry's locale, as
         `create_order` does. Share the call by pulling a small
         `send_invoice(&email, &order, …)` helper out of `create_order`.
       - Send the owner `Quote #Q accepted — order ZTM-N — €total`,
         linking to `{admin_url}/orders/{id}`.
     - Response: `200 { order_token }`.
4.3. **`decline_quote`** (body `{ reason: Option<String> }`, at most 500
     chars):
     - A guarded `UPDATE … SET status = 'declined', declined_at = now(),
       decline_reason = $2 WHERE public_token = $1 AND status = 'sent'
       AND version = $3 RETURNING …`.
     - No row: load the quote. Not found → `404`; `declined` → `200`;
       version mismatch → `409 quote_changed`; anything else → `409`.
     - On success, send the owner `Quote #Q declined by {name}` with the
       reason, linking to `{admin_url}/enquiries/catering/{enquiry_id}`.
       No customer email.
4.4. **Tests: new `tests/quote_acceptance.rs`.** Seed through the admin
     API, as Group 3's tests do.
     - `GET` by token → `200`, with no email or phone in the body;
       unknown or malformed → `404`.
     - Accepting a draft → `409`.
     - Stale page: read version 1, then the admin saves and resends
       (version 2). Accept with `version: 1` → `409 quote_changed`, and
       no order is created; accept with `version: 2` → `200`. Decline
       with a stale version → `409 quote_changed`.
     - Accepted quote, then accept again with an old version → `200` with
       the same order (idempotent).
     - Accept a sent quote:
       - `200` with an `order_token`
       - the order exists with the right items, `total`, `delivery_fee =
         0`, `status = awaiting_payment`, the enquiry's locale and
         `menu_item_id IS NULL`
       - the quote is `accepted` with its `order_id`
     - Second accept → `200` with the same `order_token`, and still one
       order.
     - Two accepts at once (`tokio::join!`) → one order.
     - `valid_until` set to yesterday through SQL, then accept → `409
       quote_expired`.
     - Decline a sent quote → `200`; decline again → `200`; accept after
       decline → `409`; admin save after decline → `draft`.
     - Decline an accepted quote → `409`.
     - Admin save of an accepted quote → `409`.
     - The created order then walks through Phase 22's `admin_orders`
       flow to `completed` (one smoke case).
     - An `on_site` quote, once accepted, creates an `on_site` order whose
       admin detail offers `next_statuses` of `["processing",
       "cancelled"]`, then `["serving_on_site", "cancelled"]` after
       payment is confirmed.
     - Six quick accepts from one `X-Forwarded-For` → a `429`.

## Group 5 — Emails and enquiry locale (repo)

Depends on: Group 2. It can run alongside Groups 3 and 4, which call it.

5.1. **`order_emails.rs`** gains:

```rust
pub fn quote_url(site_url: &str, locale: Locale, token: Uuid) -> String {
    format!("{site_url}/{}/quote/{token}", locale.as_str())
}

pub struct QuoteEmailLine<'a> { pub description: &'a str, pub quantity: i32,
                                pub unit_price: Decimal, pub subtotal: Decimal }

/// The quote sent to the customer (Phase 23 Decision 9).
pub fn quote_email(locale: Locale, customer_name: &str, quote_id: i64,
                   event_date: NaiveDate, guest_count: i32,
                   lines: &[QuoteEmailLine], total: Decimal,
                   valid_until: NaiveDate, notes: Option<&str>,
                   url: &str) -> (String, String);
```

     Add an `en` and an `nl` copy struct, in the same style as
     `InvoiceCopy`. The text is **`copy.md` §1, word for word**. Dates are
     `DD-MM-YYYY`, and amounts are `€{Decimal}`, the same as
     `invoice_email`. The signature also takes `delivery_type` and
     `delivery_address`, for the fulfilment line.
5.2. **Unit tests:** both locales return a subject containing `#Q`, and
     a body containing every line, the total, the valid-until date and the
     URL. `quote_url` has no double slash. Pin the full `en` text in one
     `assert_eq!`, like the invoice test.
5.3. **Owner emails** (accepted, declined): plain `format!` in
     `routes/quotes.rs`, next to the handlers, as with the existing owner
     alerts. The text is `copy.md` §5.
5.4. **Enquiry locale** (`routes/catering_enquiries.rs`):
     - `CreateCateringEnquiryRequest.locale: Option<String>`.
     - Validate with `Locale::parse` (`field: "locale"`, `"invalid"`) and
       insert it; a missing value is `en`.
     - Add cases to `tests/catering_enquiries.rs`: `nl` is stored,
       missing → `en`, `de` → `400`.

## Group 6 — Web: types and proxies (repo)

Depends on: Groups 3–5.

6.1. **New `apps/web/lib/quotes.ts`:**
     - `QUOTE_DISPLAY_STATUSES = ["draft", "sent", "expired",
       "accepted", "declined"] as const` and the
       `QuoteDisplayStatus` type
     - `PublicQuote`, `QuoteItem`
     - `isQuoteToken`, re-exported from `isOrderToken`'s UUID check
     - No `any`.
     - `toWhatsAppNumber(phone: string): string | null` (Decision 14):
       keep digits only, strip a leading `00`, change a leading `0` to
       `31`, and return `null` if fewer than 8 digits remain.
     - `quoteShareMessage(locale, name, quoteId, eventDate, url)`: the
       `copy.md` §4 text. Admin is outside next-intl, so it's a typed
       `Record<"en" | "nl", …>`.
     - `lib/whatsapp.ts` gains `buildWhatsAppLinkTo(number, message)`;
       the existing `buildWhatsAppLink` calls it with `WHATSAPP_NUMBER`.
6.2. **`lib/api.ts`:** add `getQuoteByToken(token)`, mirroring
     `getOrderByToken`: `null` on `404`.
6.3. **`lib/admin-enquiries.ts`:** add `AdminQuote`, `quote: AdminQuote |
     null` and `locale` on the detail type, and `quote_status:
     QuoteDisplayStatus | null` on the summary type. **`lib/admin-orders.ts`:**
     add `catering_enquiry_id: number | null` to `OrderSummary` and
     `AdminOrder`.
6.4. **Proxy routes,** copying the existing ones:
     - `app/api/quotes/[token]/accept/route.ts` and `…/decline/route.ts`,
       copies of `orders/[token]/payment-claim/route.ts`
       (`isQuoteToken` guard, forward `X-Forwarded-For`).
     - `app/api/admin/catering-enquiries/[id]/quote/route.ts` (`PUT`) and
       `…/quote/send/route.ts` (`POST`), copies of
       `admin/orders/[id]/status/route.ts` (cookie forwarding, numeric
       id guard).
6.5. **`lib/catering.ts`:** add `locale` to `CreateCateringEnquiryRequest`.
     `catering-enquiry-form.tsx` passes `locale: useLocale()`.

## Group 6b — Web: on-site in the order UI (repo)

Depends on: Group 2b.

Keep the cart's type narrow and widen only what the API returns:

6b.1. **`lib/orders.ts`:**
      - Add `"serving_on_site"` to `ORDER_STATUSES`, after
        `out_for_delivery`.
      - Keep `DeliveryType = "pickup" | "delivery"`, which the cart and
        `CreateOrderRequest` use.
      - Add `export type FulfilmentType = DeliveryType | "on_site"`, and
        use it for `OrderResponse.delivery_type`.
      - `lib/admin-orders.ts`: use `FulfilmentType` for `OrderSummary` and
        `AdminOrder`.
      - `lib/quotes.ts`: use `FulfilmentType` for quotes.
      - `components/order-details-form.tsx` keeps its own cart-only type.
6b.2. **Admin labels:**
      - `STATUS_LABELS.serving_on_site = "Catering in progress"`
        (`lib/admin-orders.ts`)
      - `admin-order-actions.tsx`: `serving_on_site: "Mark catering
        started"`
      - `admin-status-badge.tsx`: reuse `out_for_delivery`'s classes
      - the dashboard status list in `app/admin/(protected)/page.tsx`
        (line ~20): add the status after `out_for_delivery`
      - a "On-site" label wherever admin shows "Pickup"/"Delivery"
      - TypeScript's `Record<OrderStatus, …>` flags any map that's missed.
6b.3. **`components/order-progress.tsx`:**
      - Map `serving_on_site` to step 3.
      - The step-4 label branches three ways on `delivery_type`: "Ready
        for collection", "On the way", or "Catering in progress".
6b.4. **`components/order-received-button.tsx`:**
      - Render for `serving_on_site` as well.
      - The label for `on_site` is "Our event has been catered" (`nl` "Ons
        evenement is verzorgd").
6b.5. **`components/order-invoice.tsx`:** show the address for `on_site`
      too, with the "Served at" label instead of "Delivery".

## Group 7 — Web: public quote page (repo)

Depends on: Group 6.

7.1. **`app/[locale]/quote/[token]/page.tsx`:** an async server component
     modelled on `order/[token]/page.tsx`.
     - `generateMetadata`: `noindex`, `no-referrer`, title
       `t("Quote.metaTitle")`.
     - `notFound()` on `null`.
     - Render, in order: a `PageHero` with "Quote #N", a status line, the
       event summary (type, date, guests), delivery type/address, the
       line table (reuse `order-line-summary.tsx` if its props fit,
       otherwise a small `quote-lines.tsx`), the total, the owner's notes
       (`whitespace-pre-wrap`), "Valid until …", then the actions.
     - Per status:
       - `sent`: `<QuoteActions>`
       - `expired`: an "expired, contact us" notice with the WhatsApp link
         from `lib/whatsapp.ts`
       - `draft`: "being updated, you'll get an email"
       - `declined`: "You declined this quote"
       - `accepted`: a "View your order and payment details" link to
         `/{locale}/order/{order_token}`
     - No auto-refresh: the quote only changes after an email.
7.2. **`components/quote-actions.tsx`** (`"use client"`), shaped like
     `order-received-button.tsx`:
     - Props include the `version` the page rendered. Both POSTs send
       it (Decision 13).
     - **Accept:** the first click shows "Yes, accept quote" / "Cancel".
       The confirm POSTs `/api/quotes/${token}/accept` and on `200` does
       `router.push(\`/${locale}/order/${order_token}\`)` (next-intl's
       navigation `useRouter`, if that's what the other components use).
     - **Decline:** the first click shows an optional reason `<textarea>`
       (max 500) with "Decline quote" / "Cancel". The confirm POSTs
       `…/decline` and then calls `router.refresh()`.
     - `409 quote_expired` → `statusExpired`. Any other `409`
       (including `quote_changed`) → `errorChanged`. Network or `5xx` →
       `errorGeneric`. Copy keys are from `copy.md` §2.
7.3. **Accessibility:** one `<table>` with a `<caption>` for the lines,
     buttons with `aria-describedby` on the confirm text, and the existing
     `Notice` component for messages.

## Group 8 — Web: admin quote editor (repo)

Depends on: Group 6.

8.1. **`components/admin-quote-editor.tsx`** (`"use client"`). Props:
     `enquiryId`, `enquiry` (for the prefills), `quote: AdminQuote | null`.
     - **Fields:**
       - fulfilment (radio: Pickup / Delivery / On-site at the venue).
         The default follows requirement.md Decision 6: `on_site` if
         `services_required` includes `catering`, `delivery` if it
         includes `meal_delivery`, else `pickup`.
       - address: required for delivery and on-site, prefilled from
         `enquiry.location`
       - valid until: `<input type="date">`, defaulting to the earlier
         of today + 14 days and the event date
       - notes for the customer
     - **Lines:** rows of description / qty / unit price with a remove
       button, plus "Add line". One empty row when there's no quote yet.
     - **Live total:** sum in integer cents (parse `"12.50"` → `1250`)
       to avoid float drift. The API stays the source of truth. At
       €10,000 or more, show a non-blocking "Large total, please
       double-check" `Notice` (Decision 16).
     - **"Save draft":** `PUT` with prices sent as strings. Map `400`
       field errors to inline messages, the way
       `admin-menu-item-form.tsx` does. Then `router.refresh()`.
     - **"Send to customer":** disabled while there are unsaved edits
       ("Save first"). Needs a second click: "Send quote for €{total} to
       {email}?" (Decision 16). Then `POST …/send`. If `email_queued` is
       false, keep "Email is switched off. Share the link with the
       customer instead." on screen. Then `router.refresh()`.
     - **Share panel** (Decision 14), shown whenever the quote is
       `sent`, `expired` or `declined`:
       - the read-only `quote_url`
       - "Copy link" (`navigator.clipboard.writeText` in `try/catch`,
         falling back to selecting the text)
       - "Send via WhatsApp": an `<a target="_blank" rel="noopener
         noreferrer">` to `buildWhatsAppLinkTo(toWhatsAppNumber(phone),
         quoteShareMessage(enquiry.locale, …))`, hidden when the number
         is `null`
       - Sharing doesn't change the quote's status. Send is still what
         makes it acceptable.
     - **Accepted:** the whole form renders read-only, with "Accepted on
       … → View order #N" (`/orders/{order_id}`).
8.2. **`components/admin-quote-status-badge.tsx`:** the five display
     statuses, using `admin-status-badge.tsx`'s colour tokens. No new
     tokens.
8.3. **Enquiry detail page:** below the message, a "Quotation" section
     with the badge, the decline reason if declined, "Sent {time}", and
     the editor.
8.4. **Enquiry list** (`enquiries/page.tsx`, catering tab): a "Quote"
     column with the badge, or "—".
8.5. **Admin orders:** a small "Catering" tag on list rows and the order
     detail header when `catering_enquiry_id` is set. On the detail page
     it links to `/enquiries/catering/{id}`.

## Group 9 — i18n (repo)

Depends on: Groups 7 and 8.

9.0. **`Order` additions** (Group 6b): the keys in `copy.md` §3, word
     for word.
9.1. **`messages/en.json` and `nl.json`:** a new `Quote` namespace with
     the keys in `copy.md` §2, word for word. Event type labels reuse
     `ContactPage.eventType.*`. Admin copy stays English inline.

## Group 10 — Validation and client review (repo/ops)

Depends on: everything above.

10.1. Re-run `cargo sqlx prepare`, then `cargo test`, `cargo clippy`,
      `pnpm lint`, `pnpm build` and `/validate`. Tick `validation.md`.
10.2. **Staging walk-through** (requirement.md Risks 2 and 4):
      - Submit an `nl` enquiry and an `en` enquiry.
      - Quote both: one on-site, one pickup.
      - Send, revise and resend, accept one, decline the other.
      - Walk the accepted on-site order through catering in progress to
        completed, with the customer pressing "Our event has been
        catered".
      - Test the stale page: open the sent quote, then revise and resend
        it in admin, then press Accept on the old tab. It should show "This
        quote has changed".
      - Share one quote by "Send via WhatsApp" to a test phone, and check
        the message and link.
      - Forward the real emails and screenshots to the client as a final
        check against the approved `copy.md`.
      - Apply any copy changes to `copy.md` and the code.

## Implementation notes (2026-11-01)

- **Started before client sign-off on `copy.md`** (Group 0.3), at the
  user's go-ahead. All customer strings were taken from `copy.md` word for
  word. Two keys changed while implementing, and `copy.md` §3 is updated
  to match: the invoice line is `servedAtLine` ("Served at {address}"),
  like the existing `deliveryLine`, and the unused `onSiteOption` was
  dropped. `Order.status` also gained `serving_on_site` for completeness;
  nothing renders that map since Phase 22's stepper.
- **Concurrent accepts (4.2).** The first version joined `orders` in the
  locking `SELECT … FOR UPDATE OF q`. A request that waited on the lock
  saw the quote row as updated (`accepted`) but the joined order as NULL,
  because Postgres only re-checks the locked row, and it returned `409`.
  The handler now reads `q.order_id` from the locked row and looks up the
  order's token in a second query. `racing_accepts_create_one_order`
  covers it.
- **`quote_status.rs`** also has `is_expired`, which `accept_quote` uses
  to choose between `quote_expired` and `invalid_status_transition`.
- **`quote_email`** takes a `QuoteEmail` struct, not a long argument
  list, and `QuoteEmailLine` has no `unit_price`, since the email shows
  `qty x description — €subtotal`, like the invoice. A shared
  `fulfilment_line` builds the Pickup / Delivery to / Served at line for
  both the invoice and the quote email.
- **`valid_until` binding.** As with `event_date`, binding a date
  parameter needs `time::Date`, so the save uses `chrono_date_to_offset`.
- **`catering_enquiry_id`** on admin orders comes from a correlated
  subquery (`SELECT q.enquiry_id FROM catering_quotes q WHERE q.order_id
  = orders.id`), not a `LEFT JOIN`, so the same expression works in
  `RETURNING` for the status PATCH.
- **`AdminQuoteResponse`** also carries `order_reference` (`ZTM-N`) for
  the "View order" link on an accepted quote.
- **Web.**
  - `FULFILMENT_LABELS` replaces the `capitalize` of the raw value in
    admin.
  - `AdminCateringTag` is the "Catering" pill (list) and the "Catering
    enquiry #N" link (order detail).
  - `CUSTOMER_COMPLETABLE` is mirrored in `lib/orders.ts` for the
    received button.
  - The quote lines are a `QuoteLines` table component.
  - On `409 quote_expired`, the quote page refreshes to show the expired
    state and the contact link, instead of showing an inline message.
- **`robots.txt`** also disallows `/*/quote/` (token pages, like
  `/*/order/`).
- **Running the API tests locally** needs `MINIO_ACCESS_KEY` and
  `MINIO_SECRET_KEY`. Take them from `.env`'s `MINIO_ROOT_USER` and
  `MINIO_ROOT_PASSWORD`, as `docker-compose.yml` does for the API
  container.
- **Results.**
  - `cargo test`: 194 passed, 0 failed (159 before).
  - Clippy is clean, `.sqlx` is re-prepared, and the offline release
    build passes.
  - `pnpm lint` and `pnpm build` pass.
  - Live smoke test on the dev stack: enquiry `nl` → save → send →
    `en`/`nl` quote pages → stale accept `409` → accept → repeat accept
    gives the same order → on-site walk → customer received. Also a
    decline with a reason, the share panel with `wa.me/31…`, and the
    editor defaults. The smoke rows and the temporary admin were deleted
    afterwards.
