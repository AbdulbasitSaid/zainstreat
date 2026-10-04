import type { OrderStatus } from "@/lib/admin-orders";

const STATUS_CLASSES: Record<OrderStatus, string> = {
  new: "bg-accent-light text-primary-dark",
  confirmed: "bg-background-soft text-primary-dark",
  preparing: "bg-cream text-text",
  ready: "bg-whatsapp/15 text-whatsapp-dark",
  completed: "bg-card text-text-muted",
  cancelled: "bg-text/10 text-text-muted",
};

export function AdminStatusBadge({ status }: { status: OrderStatus }) {
  return (
    <span
      className={`inline-block rounded-full px-3 py-1 text-xs font-semibold capitalize ${STATUS_CLASSES[status]}`}
    >
      {status}
    </span>
  );
}
