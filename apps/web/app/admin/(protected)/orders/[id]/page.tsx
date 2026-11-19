import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { ESTIMATED_BY_LABELS, FULFILMENT_LABELS } from "@/lib/admin-orders";
import { getAdminOrder } from "@/lib/admin-orders-api";
import { AdminCateringTag } from "@/components/admin-catering-tag";
import { AdminEventTag } from "@/components/admin-event-tag";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { AdminOrderActions } from "@/components/admin-order-actions";
import { AdminEmailDeliveryNotice } from "@/components/admin-email-delivery-notice";
import { AdminOrderDeliveryFee } from "@/components/admin-order-delivery-fee";
import { Notice } from "@/components/notice";
import { formatDate, formatDateTime, formatPrice } from "@/lib/format";

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
  // Phase 37 Decision 17 / open risk 6: until the customer says they've paid.
  const feeEditable =
    order.delivery_type === "delivery" && order.status === "awaiting_payment" && order.payment_claimed_at === null;

  return (
    <div className="flex flex-col gap-8">
      <Link href="/orders" className="text-sm font-semibold text-primary underline">
        ‹ Back to orders
      </Link>

      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Order #{order.id}</h1>
        <div className="flex flex-wrap items-center gap-2">
          {order.catering_enquiry_id !== null && <AdminCateringTag enquiryId={order.catering_enquiry_id} />}
          {order.event_id !== null && <AdminEventTag eventId={order.event_id} title={order.event_title} />}
          <AdminStatusBadge status={order.status} />
        </div>
      </div>

      <AdminEmailDeliveryNotice
        delivery={order.customer_email_delivery}
        phone={order.customer_phone}
        whatsAppMessage={`Hi ${order.customer_name}, about your Zain's Treat n More order ${order.payment_reference}:`}
      />

      {order.delivery_fee_pending && (
        <Notice>Delivery fee needed: set it below before the customer can pay.</Notice>
      )}

      <AdminOrderActions
        orderId={order.id}
        status={order.status}
        next={order.next_statuses}
        completedByCustomer={order.completed_by === "customer"}
      />

      <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="Payment reference" value={order.payment_reference} />
        <Field label="Customer" value={order.customer_name} />
        <Field label="Email" value={order.customer_email} />
        <Field label="Phone" value={order.customer_phone} />
        <Field label="Type" value={FULFILMENT_LABELS[order.delivery_type]} />
        <Field label="Address" value={order.delivery_address ?? "—"} />
        {order.rental_date && <Field label="Rental date" value={formatDate(order.rental_date)} />}
        <Field label="Placed" value={formatDateTime(order.created_at)} />
        {order.payment_claimed_at && (
          <Field label="Customer says paid" value={formatDateTime(order.payment_claimed_at)} />
        )}
        {order.payment_confirmed_at && (
          <Field label="Payment confirmed" value={formatDateTime(order.payment_confirmed_at)} />
        )}
        {order.ready_at && (
          <Field
            label={
              { pickup: "Ready for collection", delivery: "Out for delivery", on_site: "Catering started", event: "Ready" }[
                order.delivery_type
              ]
            }
            value={formatDateTime(order.ready_at)}
          />
        )}
        {order.completed_at && (
          <Field
            label={order.completed_by === "customer" ? "Completed by customer" : "Completed by admin"}
            value={formatDateTime(order.completed_at)}
          />
        )}
        {order.cancelled_at && <Field label="Cancelled" value={formatDateTime(order.cancelled_at)} />}
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
                  {item.kind === "rental" && (
                    <span className="mr-2 rounded-full bg-cream px-2 py-0.5 text-xs font-semibold">Rental</span>
                  )}
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
          <dt className="text-text-muted">
            Delivery fee
            {order.delivery_distance_km && ` (${order.delivery_distance_km} km)`}
          </dt>
          <dd className="m-0">{order.delivery_fee_pending ? "Needed" : formatPrice(order.delivery_fee)}</dd>
        </div>
        {order.delivery_estimated_by && (
          <p
            className={`m-0 text-right text-xs ${order.delivery_estimated_by === "straight_line" ? "font-semibold text-primary-dark" : "text-text-muted"}`}
          >
            {ESTIMATED_BY_LABELS[order.delivery_estimated_by]}
            {order.delivery_rate_per_km && ` · ${formatPrice(order.delivery_rate_per_km)}/km`}
          </p>
        )}
        <div className="flex justify-between border-t border-text/15 pt-2 text-base font-semibold">
          <dt>Total</dt>
          <dd className="m-0">{formatPrice(order.total)}</dd>
        </div>
      </dl>

      {feeEditable && (
        <AdminOrderDeliveryFee
          orderId={order.id}
          updatedAt={order.updated_at}
          pending={order.delivery_fee_pending}
          currentFee={order.delivery_fee}
        />
      )}
    </div>
  );
}
