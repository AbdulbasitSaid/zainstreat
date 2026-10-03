"use client";

import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderLineSummary, type OrderSummaryLine } from "@/components/order-line-summary";
import { formatPrice } from "@/lib/format";
import type { OrderDetailsValues } from "@/components/order-details-form";
import type { UnavailableOrderItem } from "@/lib/orders";

export function OrderReview({
  details,
  lines,
  subtotal,
  unavailableItems,
  submitting,
  submitError,
  onEdit,
  onPlaceOrder,
}: {
  details: OrderDetailsValues;
  lines: OrderSummaryLine[];
  subtotal: number;
  unavailableItems: UnavailableOrderItem[] | null;
  submitting: boolean;
  submitError: boolean;
  onEdit: () => void;
  onPlaceOrder: () => void;
}) {
  const t = useTranslations("Order");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8">
      <section className="flex flex-col gap-1 rounded-card bg-background-soft p-5">
        <p className="m-0 font-semibold">{details.name}</p>
        <p className="m-0 text-sm text-text-muted">{details.email} · {details.phone}</p>
        <p className="m-0 text-sm text-text-muted">
          {details.deliveryType === "delivery" ? `${t("deliveryOption")}: ${details.deliveryAddress}` : t("pickupOption")}
        </p>
        {details.notes && <p className="m-0 text-sm text-text-muted">{details.notes}</p>}
        <button type="button" onClick={onEdit} className="mt-2 self-start text-sm text-primary underline">
          {t("editDetails")}
        </button>
      </section>

      <OrderLineSummary lines={lines} />

      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(subtotal.toFixed(2))}</span>
      </p>

      {unavailableItems && unavailableItems.length > 0 && (
        <Notice>
          {t("itemsUnavailable")}
          <ul>
            {unavailableItems.map((item) => (
              <li key={`${item.menu_item_id}:${item.price_option_id ?? "flat"}`}>
                {item.name ?? t("unknownItem")}
              </li>
            ))}
          </ul>
        </Notice>
      )}
      {submitError && <Notice>{t("submitError")}</Notice>}

      <Button onClick={onPlaceOrder} disabled={submitting} className="self-end">
        {submitting ? t("placingOrder") : t("placeOrder")}
      </Button>
    </div>
  );
}
