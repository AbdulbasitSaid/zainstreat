import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminOrder } from "@/lib/admin-orders-api";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { AdminOrderStatusSelect } from "@/components/admin-order-status-select";
import { formatDateTime, formatPrice } from "@/lib/format";

export const metadata: Metadata = {
  title: "Order | Zain's Admin",
};

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">{label}</p>
      <p className="m-0">{value}</p>
    </div>
  );
}

export default async function AdminOrderDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const orderId = Number.parseInt(id, 10);
  if (!Number.isInteger(orderId) || orderId < 1) {
    notFound();
  }

  const order = await getAdminOrder(orderId);

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/orders" className="text-sm font-semibold text-primary underline">
        ‹ Back to orders
      </Link>

      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Order #{order.id}</h1>
        <AdminStatusBadge status={order.status} />
      </div>

      <AdminOrderStatusSelect orderId={order.id} status={order.status} />

      <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="Customer" value={order.customer_name} />
        <Field label="Email" value={order.customer_email} />
        <Field label="Phone" value={order.customer_phone} />
        <Field label="Type" value={order.delivery_type} />
        <Field label="Address" value={order.delivery_address ?? "—"} />
        <Field label="Placed" value={formatDateTime(order.created_at)} />
        <Field label="Last updated" value={formatDateTime(order.updated_at)} />
      </div>

      <div>
        <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">Notes</p>
        <p className="m-0 whitespace-pre-wrap">{order.notes?.trim() || "—"}</p>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Item</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Unit price</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Qty</th>
              <th scope="col" className="py-3 font-semibold">Subtotal</th>
            </tr>
          </thead>
          <tbody>
            {order.items.map((item) => (
              <tr key={item.id} className="border-b border-text/10">
                <td className="py-3 pr-4">
                  {item.item_name}
                  {item.option_label && (
                    <span className="text-text-muted"> · {item.option_label}</span>
                  )}
                </td>
                <td className="py-3 pr-4">{formatPrice(item.unit_price)}</td>
                <td className="py-3 pr-4">{item.quantity}</td>
                <td className="py-3 font-semibold">{formatPrice(item.subtotal)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <dl className="m-0 ml-auto flex w-full max-w-[18rem] flex-col gap-2 text-sm">
        <div className="flex justify-between">
          <dt className="text-text-muted">Subtotal</dt>
          <dd className="m-0">{formatPrice(order.subtotal)}</dd>
        </div>
        <div className="flex justify-between">
          <dt className="text-text-muted">Delivery fee</dt>
          <dd className="m-0">{formatPrice(order.delivery_fee)}</dd>
        </div>
        <div className="flex justify-between border-t border-text/15 pt-2 text-base font-semibold">
          <dt>Total</dt>
          <dd className="m-0">{formatPrice(order.total)}</dd>
        </div>
      </dl>
    </div>
  );
}
