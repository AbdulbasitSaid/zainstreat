# Phase 10 — Order Submission: Implementation Plan

Groups 1–6 touch only `apps/api`; Groups 7–12 touch only `apps/web`. No
infra files (`docker-compose*.yml`, `Caddyfile`, `.github/**`) change —
Decision 7 deliberately avoids needing any.

## Group 0 — Branch

Branched `2026-10-03-phase-10-order-submission` off `master` (Phase 9
already merged).

## Group 1 — Migration: `order_items` price-option columns

Depends on: nothing new (schema already has `menu_item_price_options` from
Phase 7).

1.1. From `apps/api`, with `DATABASE_URL` pointing at the dev Postgres:
```bash
sqlx migrate add -r add_order_item_price_options
```

1.2. `..._add_order_item_price_options.up.sql`:
```sql
ALTER TABLE order_items
    ADD COLUMN price_option_id BIGINT REFERENCES menu_item_price_options(id) ON DELETE SET NULL,
    ADD COLUMN option_label TEXT;
```

1.3. `..._add_order_item_price_options.down.sql`:
```sql
ALTER TABLE order_items
    DROP COLUMN option_label,
    DROP COLUMN price_option_id;
```

## Group 2 — `AppError` becomes an enum: `apps/api/src/error.rs`

Depends on: nothing. Rewrite the whole file:

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UnavailableItem {
    pub menu_item_id: i64,
    pub price_option_id: Option<i64>,
    pub name: Option<String>,
    pub reason: String,
}

pub enum AppError {
    Database(sqlx::Error),
    Validation(Vec<FieldError>),
    ItemsUnavailable(Vec<UnavailableItem>),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(err) => {
                let (status, code) = match &err {
                    sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "not_found"),
                    sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_) => {
                        (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
                    }
                    sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                        Some("23505") | Some("23503") => (StatusCode::CONFLICT, "conflict"),
                        Some("23514") => (StatusCode::BAD_REQUEST, "bad_request"),
                        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error"),
                    },
                    _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_server_error"),
                };

                if status.is_server_error() {
                    tracing::error!(error = ?err, %status, "request failed");
                } else {
                    tracing::warn!(error = ?err, %status, "request failed");
                }

                (status, Json(json!({ "error": code }))).into_response()
            }
            AppError::Validation(fields) => {
                tracing::warn!(?fields, "validation failed");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "validation_error", "fields": fields })),
                )
                    .into_response()
            }
            AppError::ItemsUnavailable(items) => {
                tracing::warn!(?items, "order rejected: items unavailable");
                (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "items_unavailable", "items": items })),
                )
                    .into_response()
            }
        }
    }
}
```

No changes needed in `categories.rs`/`menu_items.rs`/`health.rs` — they only
ever produce `AppError` via `?` on a `sqlx::Error`, which still works
through the `From` impl.

## Group 3 — Order submission handler: `apps/api/src/routes/orders.rs` (new)

Depends on: Groups 1, 2.

```rust
use crate::error::{AppError, FieldError, UnavailableItem};
use axum::{extract::State, http::StatusCode, Json};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryType {
    Pickup,
    Delivery,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub menu_item_id: i64,
    pub price_option_id: Option<i64>,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: DeliveryType,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Serialize)]
pub struct OrderItemResponse {
    pub item_name: String,
    pub option_label: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    pub quantity: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: i64,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_phone: String,
    pub delivery_type: String,
    pub delivery_address: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub subtotal: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub delivery_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<OrderItemResponse>,
}

fn is_valid_email(value: &str) -> bool {
    // Deliberately loose — just "looks like an email". Decision 10 scopes
    // real validation hardening to roadmap.md Phase 16.
    match value.split_once('@') {
        Some((local, domain)) => !local.is_empty() && domain.contains('.'),
        None => false,
    }
}

fn validate(payload: &CreateOrderRequest) -> Vec<FieldError> {
    let mut errors = Vec::new();

    if payload.customer_name.trim().is_empty() {
        errors.push(FieldError { field: "customer_name".into(), message: "required".into() });
    }
    if payload.customer_email.trim().is_empty() {
        errors.push(FieldError { field: "customer_email".into(), message: "required".into() });
    } else if !is_valid_email(&payload.customer_email) {
        errors.push(FieldError { field: "customer_email".into(), message: "invalid".into() });
    }
    if payload.customer_phone.trim().is_empty() {
        errors.push(FieldError { field: "customer_phone".into(), message: "required".into() });
    }
    if matches!(payload.delivery_type, DeliveryType::Delivery)
        && payload.delivery_address.as_deref().unwrap_or("").trim().is_empty()
    {
        errors.push(FieldError { field: "delivery_address".into(), message: "required".into() });
    }
    if payload.items.is_empty() {
        errors.push(FieldError { field: "items".into(), message: "empty".into() });
    }
    for (index, item) in payload.items.iter().enumerate() {
        if item.quantity <= 0 {
            errors.push(FieldError {
                field: format!("items[{index}].quantity"),
                message: "must be positive".into(),
            });
        }
    }

    errors
}

struct MenuItemRow {
    name: String,
    price: Option<Decimal>,
    is_available: bool,
}

struct PriceOptionRow {
    label: String,
    price: Decimal,
}

struct PricedLine {
    menu_item_id: i64,
    price_option_id: Option<i64>,
    item_name: String,
    option_label: Option<String>,
    unit_price: Decimal,
    quantity: i32,
}

pub async fn create_order(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    let field_errors = validate(&payload);
    if !field_errors.is_empty() {
        return Err(AppError::Validation(field_errors));
    }

    let mut tx = pool.begin().await?;

    let mut unavailable: Vec<UnavailableItem> = Vec::new();
    let mut priced_lines: Vec<PricedLine> = Vec::new();

    for item in &payload.items {
        let menu_item = sqlx::query_as!(
            MenuItemRow,
            "SELECT name, price, is_available FROM menu_items WHERE id = $1 AND deleted_at IS NULL",
            item.menu_item_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(menu_item) = menu_item else {
            unavailable.push(UnavailableItem {
                menu_item_id: item.menu_item_id,
                price_option_id: item.price_option_id,
                name: None,
                reason: "not_found".into(),
            });
            continue;
        };

        if !menu_item.is_available {
            unavailable.push(UnavailableItem {
                menu_item_id: item.menu_item_id,
                price_option_id: item.price_option_id,
                name: Some(menu_item.name),
                reason: "unavailable".into(),
            });
            continue;
        }

        let (unit_price, option_label) = match (item.price_option_id, menu_item.price) {
            (Some(option_id), None) => {
                let option = sqlx::query_as!(
                    PriceOptionRow,
                    "SELECT label, price FROM menu_item_price_options \
                     WHERE id = $1 AND menu_item_id = $2 AND deleted_at IS NULL",
                    option_id,
                    item.menu_item_id
                )
                .fetch_optional(&mut *tx)
                .await?;

                match option {
                    Some(option) => (option.price, Some(option.label)),
                    None => {
                        unavailable.push(UnavailableItem {
                            menu_item_id: item.menu_item_id,
                            price_option_id: item.price_option_id,
                            name: Some(menu_item.name),
                            reason: "unavailable".into(),
                        });
                        continue;
                    }
                }
            }
            (None, Some(price)) => (price, None),
            // A flat-priced item submitted with an option id, or an
            // option-priced item submitted with none — the menu changed
            // shape (Phase 13 doesn't exist yet, but future-proof it).
            _ => {
                unavailable.push(UnavailableItem {
                    menu_item_id: item.menu_item_id,
                    price_option_id: item.price_option_id,
                    name: Some(menu_item.name),
                    reason: "invalid_selection".into(),
                });
                continue;
            }
        };

        priced_lines.push(PricedLine {
            menu_item_id: item.menu_item_id,
            price_option_id: item.price_option_id,
            item_name: menu_item.name,
            option_label,
            unit_price,
            quantity: item.quantity,
        });
    }

    if !unavailable.is_empty() {
        return Err(AppError::ItemsUnavailable(unavailable));
    }

    let subtotal = priced_lines
        .iter()
        .fold(Decimal::ZERO, |acc, line| acc + line.unit_price * Decimal::from(line.quantity));
    let delivery_fee = Decimal::ZERO; // Decision 9 — Beyond MVP.
    let total = subtotal + delivery_fee;

    let delivery_type_str = match payload.delivery_type {
        DeliveryType::Pickup => "pickup",
        DeliveryType::Delivery => "delivery",
    };

    let order = sqlx::query!(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type,
             delivery_address, notes, subtotal, delivery_fee, total)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, status, created_at
        "#,
        payload.customer_name,
        payload.customer_email,
        payload.customer_phone,
        delivery_type_str,
        payload.delivery_address,
        payload.notes,
        subtotal,
        delivery_fee,
        total,
    )
    .fetch_one(&mut *tx)
    .await?;

    let mut item_responses = Vec::with_capacity(priced_lines.len());
    for line in &priced_lines {
        let line_subtotal = line.unit_price * Decimal::from(line.quantity);
        sqlx::query!(
            r#"
            INSERT INTO order_items
                (order_id, menu_item_id, price_option_id, item_name, option_label,
                 unit_price, quantity, subtotal)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            order.id,
            line.menu_item_id,
            line.price_option_id,
            line.item_name,
            line.option_label,
            line.unit_price,
            line.quantity,
            line_subtotal,
        )
        .execute(&mut *tx)
        .await?;

        item_responses.push(OrderItemResponse {
            item_name: line.item_name.clone(),
            option_label: line.option_label.clone(),
            unit_price: line.unit_price,
            quantity: line.quantity,
            subtotal: line_subtotal,
        });
    }

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse {
            id: order.id,
            customer_name: payload.customer_name,
            customer_email: payload.customer_email,
            customer_phone: payload.customer_phone,
            delivery_type: delivery_type_str.to_string(),
            delivery_address: payload.delivery_address,
            notes: payload.notes,
            subtotal,
            delivery_fee,
            total,
            status: order.status,
            created_at: order.created_at,
            items: item_responses,
        }),
    ))
}
```

Notes:
- `DeliveryType`'s `#[serde(rename_all = "lowercase")]` accepts the JSON
  strings `"pickup"`/`"delivery"` straight from the frontend's
  `OrderDetailsValues.deliveryType` union.
- `Decimal::ZERO` / `Decimal::from(i32)` are both part of `rust_decimal`'s
  public API already in use elsewhere in this codebase (`menu_items.rs`
  already depends on the same crate) — adjust if the exact associated-const
  name differs from what's pinned in `Cargo.lock` at implementation time.
- Mirrors `menu_items.rs`'s existing style: plain structs + `query_as!`
  for typed rows, not an ORM.

## Group 4 — Wire the route: `apps/api/src/routes/mod.rs`

Depends on: Group 3.

```rust
pub mod categories;
pub mod health;
pub mod menu_items;
pub mod orders;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn api_router() -> Router<PgPool> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
}
```

## Group 5 — Integration tests: `apps/api/tests/order_submission.rs` (new)

Depends on: Groups 1–4. Mirrors `tests/menu_browsing.rs`'s `post`/`get`
helper pattern (add a `post` helper alongside the existing `get` one, or a
shared `apps/api/tests/common.rs` if duplication across both test files
gets awkward — either is fine, follow whichever reads cleaner once both
exist).

Cases to cover:
- Happy path, flat-priced item, pickup: `201`, response `total` matches
  `price * quantity`, `order_items`/`orders` rows exist in the test DB.
- Happy path, priced-option item: response's `option_label`/`unit_price`
  match the specific `menu_item_price_options` row, not the item's (null)
  flat `price`.
- `delivery_type: "delivery"` with no `delivery_address` → `400
  validation_error` naming the `delivery_address` field.
- Empty `items: []` → `400 validation_error` naming the `items` field.
- `quantity: 0` (or negative) on a line → `400 validation_error`.
- An item with `is_available: false` → `409 items_unavailable`, reason
  `"unavailable"`; **and** confirm no `orders`/`order_items` row was
  inserted (the rejected line's sibling valid lines must not partially
  persist either — proves the transaction rolled back).
- A soft-deleted (`deleted_at` set) `menu_item_id` → `409
  items_unavailable`, reason `"not_found"`.
- Mismatched option (flat-priced item's id sent with a non-null
  `price_option_id`, or vice versa) → `409 items_unavailable`, reason
  `"invalid_selection"`.
- Submitted price/name in the request body (if the test sends a bogus
  `unit_price`-shaped field at all — only if the request DTO even exposes
  one, which per Group 3's `CreateOrderItemRequest` it does not) is ignored;
  response reflects the current DB row regardless of what the client
  thought the price was.

## Group 6 — Offline query cache

Depends on: Groups 1–5 (needs the new queries to exist).

6.1. With the dev stack up and `DATABASE_URL` exported, from `apps/api`:
```bash
cargo sqlx prepare
```
Commit the updated `apps/api/.sqlx/` directory.

## Group 7 — Route Handler proxy: `apps/web/app/api/orders/route.ts` (new)

Depends on: nothing frontend-side; depends on Group 4 existing for the
proxy's target to be meaningful.

```ts
import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();

  const apiResponse = await fetch(`${API_BASE_URL}/api/orders`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });

  const responseBody = await apiResponse.text();
  return new NextResponse(responseBody, {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
```

Same-origin from the browser's perspective (Decision 7) — no CORS headers
needed on either side.

## Group 8 — Client API helper: `apps/web/lib/orders.ts` (new)

Depends on: Group 7.

```ts
export interface OrderItemRequest {
  menu_item_id: number;
  price_option_id: number | null;
  quantity: number;
}

export type DeliveryType = "pickup" | "delivery";

export interface CreateOrderRequest {
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: DeliveryType;
  delivery_address: string | null;
  notes: string | null;
  items: OrderItemRequest[];
}

export interface OrderItemResponse {
  item_name: string;
  option_label: string | null;
  unit_price: string;
  quantity: number;
  subtotal: string;
}

export interface OrderResponse {
  id: number;
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: DeliveryType;
  delivery_address: string | null;
  notes: string | null;
  subtotal: string;
  delivery_fee: string;
  total: string;
  status: string;
  created_at: string;
  items: OrderItemResponse[];
}

export interface UnavailableOrderItem {
  menu_item_id: number;
  price_option_id: number | null;
  name: string | null;
  reason: string;
}

export interface FieldError {
  field: string;
  message: string;
}

export type SubmitOrderResult =
  | { ok: true; order: OrderResponse }
  | { ok: false; kind: "items_unavailable"; items: UnavailableOrderItem[] }
  | { ok: false; kind: "validation_error"; fields: FieldError[] }
  | { ok: false; kind: "unknown" };

function isUnavailableItem(value: unknown): value is UnavailableOrderItem {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.menu_item_id === "number" &&
    (v.price_option_id === null || typeof v.price_option_id === "number") &&
    (v.name === null || typeof v.name === "string") &&
    typeof v.reason === "string"
  );
}

function isFieldError(value: unknown): value is FieldError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.field === "string" && typeof v.message === "string";
}

export async function submitOrder(payload: CreateOrderRequest): Promise<SubmitOrderResult> {
  const response = await fetch("/api/orders", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  const body: unknown = await response.json().catch(() => null);

  if (response.ok) {
    return { ok: true, order: body as OrderResponse };
  }

  if (
    response.status === 409 &&
    typeof body === "object" &&
    body !== null &&
    Array.isArray((body as Record<string, unknown>).items) &&
    (body as Record<string, unknown[]>).items.every(isUnavailableItem)
  ) {
    return { ok: false, kind: "items_unavailable", items: (body as { items: UnavailableOrderItem[] }).items };
  }

  if (
    response.status === 400 &&
    typeof body === "object" &&
    body !== null &&
    Array.isArray((body as Record<string, unknown>).fields) &&
    (body as Record<string, unknown[]>).fields.every(isFieldError)
  ) {
    return { ok: false, kind: "validation_error", fields: (body as { fields: FieldError[] }).fields };
  }

  return { ok: false, kind: "unknown" };
}
```

Same "parse as `unknown`, narrow with a hand-written type guard" convention
Phase 9's `lib/cart-context.tsx` established for untrusted JSON.

## Group 9 — Order details form: `apps/web/components/order-details-form.tsx` (new)

Depends on: nothing. `"use client"`.

```tsx
"use client";

import { useState, type FormEvent } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";

export type DeliveryType = "pickup" | "delivery";

export interface OrderDetailsValues {
  name: string;
  email: string;
  phone: string;
  deliveryType: DeliveryType;
  deliveryAddress: string;
  notes: string;
}

type FieldErrors = Partial<Record<keyof OrderDetailsValues, string>>;

const EMPTY_VALUES: OrderDetailsValues = {
  name: "",
  email: "",
  phone: "",
  deliveryType: "pickup",
  deliveryAddress: "",
  notes: "",
};

function isValidEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

function validate(values: OrderDetailsValues): FieldErrors {
  const errors: FieldErrors = {};
  if (!values.name.trim()) errors.name = "required";
  if (!values.email.trim()) errors.email = "required";
  else if (!isValidEmail(values.email)) errors.email = "invalid";
  if (!values.phone.trim()) errors.phone = "required";
  if (values.deliveryType === "delivery" && !values.deliveryAddress.trim()) {
    errors.deliveryAddress = "required";
  }
  return errors;
}

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

export function OrderDetailsForm({
  initialValues,
  onSubmit,
}: {
  initialValues: OrderDetailsValues | null;
  onSubmit: (values: OrderDetailsValues) => void;
}) {
  const t = useTranslations("Order");
  const [values, setValues] = useState<OrderDetailsValues>(initialValues ?? EMPTY_VALUES);
  const [errors, setErrors] = useState<FieldErrors>({});

  function update<K extends keyof OrderDetailsValues>(key: K, value: OrderDetailsValues[K]) {
    setValues((prev) => ({ ...prev, [key]: value }));
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const nextErrors = validate(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length === 0) {
      onSubmit(values);
    }
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="mx-auto flex max-w-[640px] flex-col gap-5 py-8">
      <div>
        <label htmlFor="order-name" className={FIELD_LABEL_CLASS}>{t("nameLabel")}</label>
        <input
          id="order-name"
          className="field"
          value={values.name}
          onChange={(e) => update("name", e.target.value)}
        />
        {errors.name && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.name}`, { field: t("nameLabel") })}</p>}
      </div>

      <div>
        <label htmlFor="order-email" className={FIELD_LABEL_CLASS}>{t("emailLabel")}</label>
        <input
          id="order-email"
          type="email"
          className="field"
          value={values.email}
          onChange={(e) => update("email", e.target.value)}
        />
        {errors.email && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.email}`, { field: t("emailLabel") })}</p>}
      </div>

      <div>
        <label htmlFor="order-phone" className={FIELD_LABEL_CLASS}>{t("phoneLabel")}</label>
        <input
          id="order-phone"
          type="tel"
          className="field"
          value={values.phone}
          onChange={(e) => update("phone", e.target.value)}
        />
        {errors.phone && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.phone}`, { field: t("phoneLabel") })}</p>}
      </div>

      <fieldset className="flex flex-col gap-2">
        <legend className={FIELD_LABEL_CLASS}>{t("deliveryTypeLabel")}</legend>
        <label className="flex items-center gap-2">
          <input
            type="radio"
            name="deliveryType"
            checked={values.deliveryType === "pickup"}
            onChange={() => update("deliveryType", "pickup")}
          />
          {t("pickupOption")}
        </label>
        <label className="flex items-center gap-2">
          <input
            type="radio"
            name="deliveryType"
            checked={values.deliveryType === "delivery"}
            onChange={() => update("deliveryType", "delivery")}
          />
          {t("deliveryOption")}
        </label>
      </fieldset>

      {values.deliveryType === "delivery" && (
        <div>
          <label htmlFor="order-address" className={FIELD_LABEL_CLASS}>{t("addressLabel")}</label>
          <input
            id="order-address"
            className="field"
            value={values.deliveryAddress}
            onChange={(e) => update("deliveryAddress", e.target.value)}
          />
          {errors.deliveryAddress && (
            <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.deliveryAddress}`, { field: t("addressLabel") })}</p>
          )}
        </div>
      )}

      <div>
        <label htmlFor="order-notes" className={FIELD_LABEL_CLASS}>{t("notesLabel")}</label>
        <textarea
          id="order-notes"
          className="field"
          rows={3}
          value={values.notes}
          onChange={(e) => update("notes", e.target.value)}
        />
      </div>

      <Button type="submit" className="self-start">{t("continueToReview")}</Button>
    </form>
  );
}
```

`t(\`errors.${errors.name}\`, ...)` assumes translation keys
`Order.errors.required` / `Order.errors.invalid` exist (Group 11) — adjust
the exact message-interpolation approach during implementation if
`next-intl`'s dynamic-key typing complains; a small `ERROR_MESSAGE_KEYS`
lookup map is an easy fallback if so.

## Group 10 — Review, success, and the wizard: three new components + edit `cart-view.tsx`

Depends on: Groups 8, 9.

**`apps/web/components/order-line-summary.tsx`** (new) — read-only line
list shared by the review and success steps (no stepper/remove, unlike
Phase 9's `CartLineItem`):

```tsx
import { formatPrice } from "@/lib/format";

export interface OrderSummaryLine {
  name: string;
  optionLabel: string | null;
  unitPrice: string;
  quantity: number;
}

export function OrderLineSummary({ lines }: { lines: OrderSummaryLine[] }) {
  return (
    <ul className="m-0 list-none p-0">
      {lines.map((line, index) => (
        <li key={index} className="flex items-center justify-between border-b border-text/10 py-3 last:border-0">
          <div>
            <p className="m-0 font-semibold">{line.name}</p>
            {line.optionLabel && <p className="m-0 text-xs text-text-muted">{line.optionLabel}</p>}
          </div>
          <p className="m-0 text-sm text-text-muted">
            {line.quantity} × {formatPrice(line.unitPrice)}
          </p>
        </li>
      ))}
    </ul>
  );
}
```

**`apps/web/components/order-review.tsx`** (new), `"use client"` — takes
the cart's `CartLine[]` (mapped to `OrderSummaryLine[]`) plus the details
form's values, shows unavailable-item errors inline when present:

```tsx
"use client";

import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderLineSummary, type OrderSummaryLine } from "@/components/order-line-summary";
import { formatPrice } from "@/lib/format";
import type { OrderDetailsValues } from "@/components/order-details-form";
import type { UnavailableOrderItem } from "@/lib/orders";

export function OrderReview({
  details,
  lines,
  subtotal,
  unavailableItems,
  submitting,
  submitError,
  onEdit,
  onPlaceOrder,
}: {
  details: OrderDetailsValues;
  lines: OrderSummaryLine[];
  subtotal: number;
  unavailableItems: UnavailableOrderItem[] | null;
  submitting: boolean;
  submitError: boolean;
  onEdit: () => void;
  onPlaceOrder: () => void;
}) {
  const t = useTranslations("Order");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8">
      <section className="flex flex-col gap-1 rounded-card bg-background-soft p-5">
        <p className="m-0 font-semibold">{details.name}</p>
        <p className="m-0 text-sm text-text-muted">{details.email} · {details.phone}</p>
        <p className="m-0 text-sm text-text-muted">
          {details.deliveryType === "delivery" ? `${t("deliveryOption")}: ${details.deliveryAddress}` : t("pickupOption")}
        </p>
        {details.notes && <p className="m-0 text-sm text-text-muted">{details.notes}</p>}
        <button type="button" onClick={onEdit} className="mt-2 self-start text-sm text-primary underline">
          {t("editDetails")}
        </button>
      </section>

      <OrderLineSummary lines={lines} />

      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(subtotal.toFixed(2))}</span>
      </p>

      {unavailableItems && unavailableItems.length > 0 && (
        <Notice>
          {t("itemsUnavailable")}
          <ul>
            {unavailableItems.map((item) => (
              <li key={`${item.menu_item_id}:${item.price_option_id ?? "flat"}`}>
                {item.name ?? t("unknownItem")}
              </li>
            ))}
          </ul>
        </Notice>
      )}
      {submitError && <Notice>{t("submitError")}</Notice>}

      <Button onClick={onPlaceOrder} disabled={submitting} className="self-end">
        {submitting ? t("placingOrder") : t("placeOrder")}
      </Button>
    </div>
  );
}
```

**`apps/web/components/order-success.tsx`** (new):

```tsx
import { useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { OrderLineSummary } from "@/components/order-line-summary";
import { formatPrice } from "@/lib/format";
import type { OrderResponse } from "@/lib/orders";

export function OrderSuccess({ order }: { order: OrderResponse }) {
  const t = useTranslations("Order");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8 text-center">
      <h2 className="m-0">{t("successHeading", { id: order.id })}</h2>
      <p className="m-0 text-text-muted">{t("successBody")}</p>
      <OrderLineSummary
        lines={order.items.map((item) => ({
          name: item.item_name,
          optionLabel: item.option_label,
          unitPrice: item.unit_price,
          quantity: item.quantity,
        }))}
      />
      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(order.total)}</span>
      </p>
      <ButtonLink href="/menu" className="self-center">{t("backToMenu")}</ButtonLink>
    </div>
  );
}
```

**`apps/web/components/order-view.tsx`** (new), `"use client"` — the
wizard orchestrator:

```tsx
"use client";

import { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderDetailsForm, type OrderDetailsValues } from "@/components/order-details-form";
import { OrderReview } from "@/components/order-review";
import { OrderSuccess } from "@/components/order-success";
import { useCart } from "@/lib/cart-context";
import { submitOrder, type OrderResponse, type UnavailableOrderItem } from "@/lib/orders";

const LAST_ORDER_KEY = "zainstreat:last-order:v1";

type Step = "details" | "review" | "success";

export function OrderView() {
  const t = useTranslations("Order");
  const { lines, subtotal, clearCart } = useCart();
  const [step, setStep] = useState<Step>("details");
  const [details, setDetails] = useState<OrderDetailsValues | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [unavailableItems, setUnavailableItems] = useState<UnavailableOrderItem[] | null>(null);
  const [submitError, setSubmitError] = useState(false);
  const [order, setOrder] = useState<OrderResponse | null>(null);

  // Recover a just-completed order after a hard refresh (Decision 5) — only
  // when the cart is already empty, i.e. we're not mid-way through building
  // a new one.
  useEffect(() => {
    if (lines.length > 0) return;
    try {
      const raw = window.sessionStorage.getItem(LAST_ORDER_KEY);
      if (raw) {
        setOrder(JSON.parse(raw) as OrderResponse);
        setStep("success");
      }
    } catch {
      // ignore — no recovered order, falls through to the empty-cart state below.
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handlePlaceOrder(values: OrderDetailsValues) {
    setSubmitting(true);
    setUnavailableItems(null);
    setSubmitError(false);

    const result = await submitOrder({
      customer_name: values.name,
      customer_email: values.email,
      customer_phone: values.phone,
      delivery_type: values.deliveryType,
      delivery_address: values.deliveryType === "delivery" ? values.deliveryAddress : null,
      notes: values.notes.trim() ? values.notes : null,
      items: lines.map((line) => ({
        menu_item_id: line.itemId,
        price_option_id: line.priceOptionId,
        quantity: line.quantity,
      })),
    });

    setSubmitting(false);

    if (result.ok) {
      setOrder(result.order);
      clearCart();
      try {
        window.sessionStorage.setItem(LAST_ORDER_KEY, JSON.stringify(result.order));
      } catch {
        // sessionStorage unavailable — confirmation still renders this pass, just won't survive a reload.
      }
      setStep("success");
      return;
    }

    if (result.kind === "items_unavailable") {
      setUnavailableItems(result.items);
      return;
    }

    setSubmitError(true);
  }

  if (step !== "success" && lines.length === 0) {
    return (
      <div className="flex flex-col items-center gap-6 py-16 text-center">
        <Notice>{t("emptyCart")}</Notice>
        <ButtonLink href="/menu">{t("continueShopping")}</ButtonLink>
      </div>
    );
  }

  if (step === "success" && order) {
    return <OrderSuccess order={order} />;
  }

  if (step === "review" && details) {
    return (
      <OrderReview
        details={details}
        lines={lines.map((line) => ({
          name: line.name,
          optionLabel: line.optionLabel,
          unitPrice: line.unitPrice,
          quantity: line.quantity,
        }))}
        subtotal={subtotal}
        unavailableItems={unavailableItems}
        submitting={submitting}
        submitError={submitError}
        onEdit={() => setStep("details")}
        onPlaceOrder={() => handlePlaceOrder(details)}
      />
    );
  }

  return (
    <OrderDetailsForm
      initialValues={details}
      onSubmit={(values) => {
        setDetails(values);
        setUnavailableItems(null);
        setSubmitError(false);
        setStep("review");
      }}
    />
  );
}
```

**Edit `apps/web/components/cart-view.tsx`**: replace the disabled
placeholder button with a real link now that `/order` exists:

```tsx
<ButtonLink href="/order">{t("proceedToOrder")}</ButtonLink>
```

removing the `disabled`/`title={t("proceedToOrderComingSoon")}` props and
the now-unused `proceedToOrderComingSoon` translation key (Group 11 — drop
it from both message files rather than leaving a dead key).

## Group 11 — Order page route: `apps/web/app/[locale]/order/page.tsx` (new)

Depends on: Group 10. Mirrors `cart/page.tsx`'s `generateMetadata` pattern:

```tsx
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { OrderView } from "@/components/order-view";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "Order" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}

export default async function OrderPage() {
  const t = await getTranslations("Order");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
      </PageHero>
      <OrderView />
    </main>
  );
}
```

## Group 12 — Translations

Add to **both** `apps/web/messages/en.json` and `apps/web/messages/nl.json`:

- A new top-level `Order` namespace:

  English:
  ```json
  "Order": {
    "metaTitle": "Place Your Order | Zain's Treat n More",
    "metaDescription": "Enter your details and review your order before submitting.",
    "heading": "Your Order",
    "emptyCart": "Your cart is empty — add something from the menu first.",
    "continueShopping": "Continue Shopping",
    "nameLabel": "Full Name",
    "emailLabel": "Email",
    "phoneLabel": "Phone Number",
    "deliveryTypeLabel": "Pickup or Delivery",
    "pickupOption": "Pickup",
    "deliveryOption": "Delivery",
    "addressLabel": "Delivery Address",
    "notesLabel": "Notes (optional)",
    "continueToReview": "Review Order",
    "editDetails": "Edit Details",
    "total": "Total",
    "itemsUnavailable": "Some items in your order are no longer available:",
    "unknownItem": "An item",
    "submitError": "Something went wrong submitting your order. Please try again.",
    "placeOrder": "Place Order",
    "placingOrder": "Placing Order…",
    "successHeading": "Order #{id} Confirmed",
    "successBody": "Thank you! We'll contact you by phone or WhatsApp shortly to confirm your order.",
    "backToMenu": "Back to Menu",
    "errors": {
      "required": "{field} is required.",
      "invalid": "Please enter a valid {field}."
    }
  }
  ```

  Dutch:
  ```json
  "Order": {
    "metaTitle": "Bestelling Plaatsen | Zain's Treat n More",
    "metaDescription": "Vul je gegevens in en controleer je bestelling voordat je verzendt.",
    "heading": "Jouw Bestelling",
    "emptyCart": "Je winkelwagen is leeg — voeg eerst iets toe vanaf het menu.",
    "continueShopping": "Verder Winkelen",
    "nameLabel": "Volledige Naam",
    "emailLabel": "E-mail",
    "phoneLabel": "Telefoonnummer",
    "deliveryTypeLabel": "Afhalen of Bezorgen",
    "pickupOption": "Afhalen",
    "deliveryOption": "Bezorgen",
    "addressLabel": "Bezorgadres",
    "notesLabel": "Opmerkingen (optioneel)",
    "continueToReview": "Bestelling Controleren",
    "editDetails": "Gegevens Bewerken",
    "total": "Totaal",
    "itemsUnavailable": "Sommige items in je bestelling zijn niet meer beschikbaar:",
    "unknownItem": "Een item",
    "submitError": "Er ging iets mis bij het verzenden van je bestelling. Probeer het opnieuw.",
    "placeOrder": "Bestelling Plaatsen",
    "placingOrder": "Bezig met plaatsen…",
    "successHeading": "Bestelling #{id} Bevestigd",
    "successBody": "Bedankt! We nemen snel telefonisch of via WhatsApp contact met je op om je bestelling te bevestigen.",
    "backToMenu": "Terug naar Menu",
    "errors": {
      "required": "{field} is verplicht.",
      "invalid": "Voer een geldige {field} in."
    }
  }
  ```

- `Cart` namespace: drop `proceedToOrderComingSoon` (no longer rendered,
  Group 10) from both files. `proceedToOrder` key is reused as-is, no
  wording change needed.

Both files must stay in sync (same key set) — existing convention confirmed
across every other namespace.

## Verification

See `validation.md`.
