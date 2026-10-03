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
