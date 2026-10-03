import { useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { OrderLineSummary } from "@/components/order-line-summary";
import { formatPrice } from "@/lib/format";
import type { OrderResponse } from "@/lib/orders";

export function OrderSuccess({ order }: { order: OrderResponse }) {
  const t = useTranslations("Order");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8 text-center">
      <h2 className="m-0">{t("successHeading", { id: order.id })}</h2>
      <p className="m-0 text-text-muted">{t("successBody")}</p>
      <OrderLineSummary
        lines={order.items.map((item) => ({
          name: item.item_name,
          optionLabel: item.option_label,
          unitPrice: item.unit_price,
          quantity: item.quantity,
        }))}
      />
      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(order.total)}</span>
      </p>
      <ButtonLink href="/menu" className="self-center">{t("backToMenu")}</ButtonLink>
    </div>
  );
}
