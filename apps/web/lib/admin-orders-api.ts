import { adminApiJson } from "@/lib/admin-api";
import {
  ORDERS_PAGE_SIZE,
  type AdminOrder,
  type AdminOrderList,
  type AdminOrdersSummary,
  type OrderStatus,
} from "@/lib/admin-orders";

export function getAdminOrders({
  status,
  page = 1,
}: { status?: OrderStatus; page?: number } = {}): Promise<AdminOrderList> {
  const params = new URLSearchParams({
    limit: String(ORDERS_PAGE_SIZE),
    offset: String((page - 1) * ORDERS_PAGE_SIZE),
  });
  if (status) params.set("status", status);
  return adminApiJson<AdminOrderList>(`/api/admin/orders?${params.toString()}`);
}

export function getAdminOrder(id: number): Promise<AdminOrder> {
  return adminApiJson<AdminOrder>(`/api/admin/orders/${id}`);
}

export function getAdminOrdersSummary(): Promise<AdminOrdersSummary> {
  return adminApiJson<AdminOrdersSummary>("/api/admin/orders/summary");
}
