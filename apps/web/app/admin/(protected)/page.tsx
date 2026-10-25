import Link from "next/link";
import type { Metadata } from "next";
import { STATUS_LABELS, type OrderStatus } from "@/lib/admin-orders";
import { getAdminOrdersSummary } from "@/lib/admin-orders-api";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Dashboard | Zain's Admin",
};

// Payment claims first: they're the one status waiting on the owner
// (Phase 20 Decision 12).
const DASHBOARD_STATUSES: OrderStatus[] = [
  "payment_claimed",
  "awaiting_payment",
  "processing",
  "ready_for_collection",
  "out_for_delivery",
  "completed",
  "cancelled",
];

export default async function AdminDashboardPage() {
  const summary = await getAdminOrdersSummary();

  return (
    <div className="flex flex-col gap-10">
      <h1 className="m-0">Dashboard</h1>

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div className="rounded-card bg-background-soft p-5">
          <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">
            Placed today
          </p>
          <p className="m-0 text-3xl font-semibold text-primary">{summary.today}</p>
        </div>
        {DASHBOARD_STATUSES.map((status) => (
          <Link
            key={status}
            href={`/admin/orders?status=${status}`}
            className={`rounded-card p-5 ${
              status === "payment_claimed" && summary.counts[status] > 0
                ? "bg-accent-light ring-2 ring-primary"
                : "bg-card"
            } transition-transform duration-150 ease-out-expo hover:-translate-y-0.5 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2`}
          >
            <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">
              {STATUS_LABELS[status]}
              {status === "payment_claimed" && summary.counts[status] > 0 && " · needs action"}
            </p>
            <p className="m-0 text-3xl font-semibold">{summary.counts[status]}</p>
          </Link>
        ))}
      </div>

      <div className="flex flex-col gap-4">
        <div className="flex items-baseline justify-between">
          <h2 className="m-0 text-xl font-semibold">Recent orders</h2>
          <Link href="/admin/orders" className="text-sm font-semibold text-primary underline">
            View all
          </Link>
        </div>

        {summary.recent.length === 0 ? (
          <p className="m-0 text-text-muted">No orders yet.</p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {summary.recent.map((order) => (
              <li key={order.id}>
                <Link
                  href={`/admin/orders/${order.id}`}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-card border border-text/10 px-5 py-4 hover:bg-background-soft focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                >
                  <span className="font-semibold">#{order.id}</span>
                  <span>{order.customer_name}</span>
                  <span className="text-text-muted">{formatDateTime(order.created_at)}</span>
                  <span className="font-semibold">{formatPrice(order.total)}</span>
                  <AdminStatusBadge status={order.status} />
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
