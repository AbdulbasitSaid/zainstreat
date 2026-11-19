import { useTranslations } from "next-intl";
import { formatPrice } from "@/lib/format";

// Subtotal, delivery and total (Phase 37 Decisions 13, 17). Without a
// delivery fee it's just the total, as before. Server-compatible.
export function OrderTotals({
  subtotal,
  delivery,
}: {
  subtotal: string;
  // null for pickup (and every order without a delivery charge).
  delivery: { fee: string; distanceKm: string | null } | "pending" | null;
}) {
  const t = useTranslations("Order");
  const rowClass = "m-0 flex justify-between gap-4 text-text-muted";

  if (delivery === null) {
    return (
      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(subtotal)}</span>
      </p>
    );
  }

  const total =
    delivery === "pending"
      ? t("totalPlusDelivery", { subtotal: formatPrice(subtotal) })
      : formatPrice((Number.parseFloat(subtotal) + Number.parseFloat(delivery.fee)).toFixed(2));

  return (
    <div className="ml-auto flex w-full max-w-xs flex-col gap-1">
      <p className={rowClass}>
        <span>{t("subtotal")}</span>
        <span>{formatPrice(subtotal)}</span>
      </p>
      <p className={rowClass}>
        <span>
          {delivery !== "pending" && delivery.distanceKm
            ? t("deliveryFeeLine", { km: delivery.distanceKm })
            : t("deliveryOption")}
        </span>
        <span>{delivery === "pending" ? t("deliveryToConfirm") : formatPrice(delivery.fee)}</span>
      </p>
      <p className="m-0 flex justify-between gap-4 border-t border-text/10 pt-1 text-lg font-semibold">
        <span>{t("total")}</span>
        <span className="text-primary">{total}</span>
      </p>
    </div>
  );
}
