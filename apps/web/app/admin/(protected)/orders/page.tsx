import Link from "next/link";
import type { Metadata } from "next";
import { isOrderStatus, ORDERS_PAGE_SIZE, ORDER_STATUSES } from "@/lib/admin-orders";
import { getAdminOrders } from "@/lib/admin-orders-api";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Orders | Zain's Admin",
};

function buildHref(status: string | undefined, page: number) {
  const params = new URLSearchParams();
  if (status) params.set("status", status);
  if (page > 1) params.set("page", String(page));
  const query = params.toString();
  return query ? `/admin/orders?${query}` : "/admin/orders";
}

export default async function AdminOrdersPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const { status: rawStatus, page: rawPage } = await searchParams;

  // Anything not a real status is treated as "All" rather than 400ing the
  // page — a hand-typed URL shouldn't break the dashboard.
  const status = rawStatus && isOrderStatus(rawStatus) ? rawStatus : undefined;
  const parsedPage = Number.parseInt(rawPage ?? "1", 10);
  const page = Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1;

  const { orders, total } = await getAdminOrders({ status, page });
  const pageCount = Math.max(1, Math.ceil(total / ORDERS_PAGE_SIZE));

  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Orders</h1>

      <div className="flex flex-wrap gap-2">
        {[undefined, ...ORDER_STATUSES].map((value) => (
          <Link
            key={value ?? "all"}
            href={buildHref(value, 1)}
            aria-current={status === value ? "page" : undefined}
            className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider transition-colors duration-150 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              status === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value ?? "All"}
          </Link>
        ))}
      </div>

      {orders.length === 0 ? (
        <p className="m-0 text-text-muted">
          {status ? `No ${status} orders.` : "No orders yet."}
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
                <th scope="col" className="py-3 pr-4 font-semibold">Order</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Customer</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Type</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Total</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Status</th>
                <th scope="col" className="py-3 font-semibold">Placed</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((order) => (
                <tr key={order.id} className="border-b border-text/10 hover:bg-background-soft">
                  <td className="py-3 pr-4">
                    <Link
                      href={`/admin/orders/${order.id}`}
                      className="font-semibold text-primary underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                    >
                      #{order.id}
                    </Link>
                  </td>
                  <td className="py-3 pr-4">{order.customer_name}</td>
                  <td className="py-3 pr-4 capitalize">{order.delivery_type}</td>
                  <td className="py-3 pr-4 font-semibold">{formatPrice(order.total)}</td>
                  <td className="py-3 pr-4"><AdminStatusBadge status={order.status} /></td>
                  <td className="py-3 text-text-muted">{formatDateTime(order.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {pageCount > 1 && (
        <nav aria-label="Pagination" className="flex items-center justify-center gap-6 text-sm">
          {page > 1 ? (
            <Link href={buildHref(status, page - 1)} className="font-semibold text-primary underline">
              ‹ Previous
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">‹ Previous</span>
          )}
          <span className="text-text-muted">Page {page} of {pageCount}</span>
          {page < pageCount ? (
            <Link href={buildHref(status, page + 1)} className="font-semibold text-primary underline">
              Next ›
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">Next ›</span>
          )}
        </nav>
      )}
    </div>
  );
}
