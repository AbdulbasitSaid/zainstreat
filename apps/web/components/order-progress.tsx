import { useLocale, useTranslations } from "next-intl";
import type { OrderResponse, OrderStatus } from "@/lib/orders";

// How far along each status is, on the five-step tracker below. Driven by the
// status rather than the timestamps, so orders from before Phase 22 (no
// ready_at/completed_at) still show the right step; they just show no time.
const STEP_REACHED: Record<Exclude<OrderStatus, "cancelled">, number> = {
  awaiting_payment: 0,
  payment_claimed: 1,
  processing: 2,
  ready_for_collection: 3,
  out_for_delivery: 3,
  serving_on_site: 3,
  completed: 4,
};

// The customer's view of the Phase 20 lifecycle, with its pickup/delivery
// branch (Phase 22 Decision 3) and Phase 23's on-site catering branch.
export function OrderProgress({ order }: { order: OrderResponse }) {
  const t = useTranslations("Order.tracking");
  const locale = useLocale();

  // Explicit timeZone, same reason as order-invoice.tsx.
  const formatTime = (iso: string) =>
    new Intl.DateTimeFormat(locale, {
      timeZone: "Europe/Amsterdam",
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(iso));

  if (order.status === "cancelled") {
    return (
      <div className="rounded-card bg-card p-5 text-center">
        <p className="m-0 text-lg font-semibold">{t("cancelled")}</p>
        {order.cancelled_at && (
          <p className="m-0 mt-1 text-sm text-text-muted">{t("cancelledAt", { date: formatTime(order.cancelled_at) })}</p>
        )}
      </div>
    );
  }

  const handoverLabel = {
    pickup: t("readyForCollection"),
    delivery: t("onTheWay"),
    on_site: t("cateringInProgress"),
  }[order.delivery_type];

  const steps = [
    { label: t("placed"), at: order.created_at },
    { label: t("paymentSent"), at: order.payment_claimed_at },
    { label: t("paymentConfirmed"), at: order.payment_confirmed_at },
    { label: handoverLabel, at: order.ready_at },
    { label: t("completed"), at: order.completed_at },
  ];
  const reached = STEP_REACHED[order.status];

  return (
    <ol aria-label={t("progressLabel")} className="m-0 flex list-none flex-col gap-3 p-0 text-left">
      {steps.map((step, index) => {
        const done = index <= reached;
        const current = index === reached;
        return (
          <li key={index} aria-current={current ? "step" : undefined} className="flex items-start gap-3">
            <span
              aria-hidden="true"
              className={`mt-1 flex size-5 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
                done ? "bg-primary text-white" : "border-2 border-text-muted/40"
              }`}
            >
              {done ? "✓" : ""}
            </span>
            <span className="flex flex-col">
              <span className={current ? "font-semibold" : done ? "" : "text-text-muted"}>{step.label}</span>
              {done && step.at && <span className="text-sm text-text-muted">{formatTime(step.at)}</span>}
            </span>
          </li>
        );
      })}
    </ol>
  );
}
