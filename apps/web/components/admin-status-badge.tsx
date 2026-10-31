import { STATUS_LABELS, type OrderStatus } from "@/lib/admin-orders";

const STATUS_CLASSES: Record<OrderStatus, string> = {
  awaiting_payment: "bg-background-soft text-text-muted",
  // The one status waiting on the owner — the most eye-catching style.
  payment_claimed: "bg-accent-light text-primary-dark",
  processing: "bg-cream text-text",
  ready_for_collection: "bg-whatsapp/15 text-whatsapp-dark",
  out_for_delivery: "bg-whatsapp/15 text-whatsapp-dark",
  completed: "bg-card text-text-muted",
  cancelled: "bg-text/10 text-text-muted",
};

export function AdminStatusBadge({ status }: { status: OrderStatus }) {
  return (
    <span className={`inline-block rounded-full px-3 py-1 text-xs font-semibold ${STATUS_CLASSES[status]}`}>
      {STATUS_LABELS[status]}
    </span>
  );
}
