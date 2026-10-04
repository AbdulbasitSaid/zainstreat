export const ORDER_STATUSES = [
  "new",
  "confirmed",
  "preparing",
  "ready",
  "completed",
  "cancelled",
] as const;

export type OrderStatus = (typeof ORDER_STATUSES)[number];

export function isOrderStatus(value: string): value is OrderStatus {
  return (ORDER_STATUSES as readonly string[]).includes(value);
}

export const ORDERS_PAGE_SIZE = 25;

export interface OrderSummary {
  id: number;
  customer_name: string;
  delivery_type: "pickup" | "delivery";
  total: string;
  status: OrderStatus;
  created_at: string;
}

export interface AdminOrderItem {
  id: number;
  item_name: string;
  option_label: string | null;
  unit_price: string;
  quantity: number;
  subtotal: string;
}

export interface AdminOrder {
  id: number;
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  delivery_type: "pickup" | "delivery";
  delivery_address: string | null;
  notes: string | null;
  subtotal: string;
  delivery_fee: string;
  total: string;
  status: OrderStatus;
  created_at: string;
  updated_at: string;
  items: AdminOrderItem[];
}

export interface AdminOrderList {
  orders: OrderSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface AdminOrdersSummary {
  counts: Record<OrderStatus, number>;
  today: number;
  recent: OrderSummary[];
}
