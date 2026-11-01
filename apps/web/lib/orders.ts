export interface OrderItemRequest {
  menu_item_id: number;
  price_option_id: number | null;
  quantity: number;
}

export type DeliveryType = "pickup" | "delivery";

// What an order can come back as: the cart's two, plus on-site for an
// accepted catering quote (Phase 23 Decision 12). The cart keeps sending
// DeliveryType only.
export type FulfilmentType = DeliveryType | "on_site";

// Mirrors ORDER_STATUSES in apps/api/src/order_status.rs and the CHECK on
// orders.status (Phase 20 Decision 1; serving_on_site from Phase 23).
export const ORDER_STATUSES = [
  "awaiting_payment",
  "payment_claimed",
  "processing",
  "ready_for_collection",
  "out_for_delivery",
  "serving_on_site",
  "completed",
  "cancelled",
] as const;

export type OrderStatus = (typeof ORDER_STATUSES)[number];

// Mirrors CUSTOMER_COMPLETABLE in apps/api/src/order_status.rs: the
// statuses from which the customer can confirm receipt (Phase 22 Decision 8).
export const CUSTOMER_COMPLETABLE: readonly OrderStatus[] = [
  "ready_for_collection",
  "out_for_delivery",
  "serving_on_site",
];

export function isOrderStatus(value: string): value is OrderStatus {
  return (ORDER_STATUSES as readonly string[]).includes(value);
}

// Order tokens are UUIDs (orders.public_token); anything else can't match.
const ORDER_TOKEN_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isOrderToken(value: string): boolean {
  return ORDER_TOKEN_PATTERN.test(value);
}

export interface CreateOrderRequest {
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: DeliveryType;
  delivery_address: string | null;
  notes: string | null;
  items: OrderItemRequest[];
  // The site language, for the customer's emails (Phase 22 Decision 5).
  locale: string;
}

export interface OrderItemResponse {
  item_name: string;
  option_label: string | null;
  unit_price: string;
  quantity: number;
  subtotal: string;
}

export interface PaymentInstructions {
  account_name: string;
  iban: string;
  bic: string | null;
  // Chamber of Commerce number, only when BUSINESS_KVK is set (Phase 20 open risk 2).
  kvk: string | null;
  reference: string;
}

export interface OrderResponse {
  id: number;
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: FulfilmentType;
  delivery_address: string | null;
  notes: string | null;
  subtotal: string;
  delivery_fee: string;
  total: string;
  status: OrderStatus;
  created_at: string;
  items: OrderItemResponse[];
  public_token: string;
  payment_reference: string;
  // null when the business hasn't configured bank details (Phase 20 Decision 5).
  payment: PaymentInstructions | null;
  // When each tracking step was reached; null until then (Phase 22 Decision 4).
  payment_claimed_at: string | null;
  payment_confirmed_at: string | null;
  ready_at: string | null;
  completed_at: string | null;
  cancelled_at: string | null;
  completed_by: "customer" | "admin" | null;
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
