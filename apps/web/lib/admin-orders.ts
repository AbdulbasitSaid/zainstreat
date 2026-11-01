import type { EmailDelivery } from "@/components/admin-email-delivery-notice";
import type { FulfilmentType, OrderStatus } from "@/lib/orders";

export { ORDER_STATUSES, isOrderStatus, type OrderStatus } from "@/lib/orders";

export const STATUS_LABELS: Record<OrderStatus, string> = {
  awaiting_payment: "Awaiting payment",
  payment_claimed: "Payment claimed",
  processing: "Processing",
  ready_for_collection: "Ready for collection",
  out_for_delivery: "On the way",
  serving_on_site: "Catering in progress",
  completed: "Completed",
  cancelled: "Cancelled",
};

export const FULFILMENT_LABELS: Record<FulfilmentType, string> = {
  pickup: "Pickup",
  delivery: "Delivery",
  on_site: "On-site",
};

export const ORDERS_PAGE_SIZE = 25;

export interface OrderSummary {
  id: number;
  payment_reference: string;
  customer_name: string;
  delivery_type: FulfilmentType;
  total: string;
  status: OrderStatus;
  created_at: string;
  // Set when the order came from an accepted catering quote (Phase 23 Decision 7).
  catering_enquiry_id: number | null;
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
  delivery_type: FulfilmentType;
  delivery_address: string | null;
  notes: string | null;
  subtotal: string;
  delivery_fee: string;
  total: string;
  status: OrderStatus;
  payment_reference: string;
  payment_claimed_at: string | null;
  payment_confirmed_at: string | null;
  ready_at: string | null;
  completed_at: string | null;
  cancelled_at: string | null;
  // Who completed it; a customer-completed order can be reopened (Phase 22 Decision 8).
  completed_by: "customer" | "admin" | null;
  catering_enquiry_id: number | null;
  // admin_next() in apps/api/src/order_status.rs, computed by the API so
  // there's no TypeScript copy of the transition rules (Phase 20 open risk 4).
  next_statuses: OrderStatus[];
  created_at: string;
  updated_at: string;
  items: AdminOrderItem[];
  // The latest customer email's outcome (Phase 25 Decision 6a).
  customer_email_delivery: EmailDelivery | null;
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
  // Orders in payment_claimed, for the auto-refresh badge (Phase 22 Decision 9).
  claimed_ids: number[];
}
