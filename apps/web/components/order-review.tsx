"use client";

import { useTranslations } from "next-intl";
import { Button, ButtonLink } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderLineSummary, type OrderSummaryLine } from "@/components/order-line-summary";
import { OrderTotals } from "@/components/order-totals";
import type { OrderDetailsValues } from "@/components/order-details-form";
import type { BelowMinimumLine } from "@/lib/orders";

export function OrderReview({
  details,
  lines,
  subtotal,
  unavailableNames,
  belowMinimumLines,
  feeChangedMessage,
  submitting,
  submitError,
  onEdit,
  onPlaceOrder,
}: {
  details: OrderDetailsValues;
  lines: OrderSummaryLine[];
  subtotal: number;
  unavailableNames: string[] | null;
  belowMinimumLines: BelowMinimumLine[] | null;
  feeChangedMessage: string | null;
  submitting: boolean;
  submitError: boolean;
  onEdit: () => void;
  onPlaceOrder: () => void;
}) {
  const t = useTranslations("Order");
  const isDelivery = details.deliveryType === "delivery";
  const delivery = !isDelivery
    ? null
    : details.addressLookupDown || !details.deliveryQuote
      ? ("pending" as const)
      : { fee: details.deliveryQuote.fee, distanceKm: details.deliveryQuote.distance_km };
  const address = [details.deliveryAddress, details.deliveryAddressExtra.trim()].filter(Boolean).join(", ");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8">
      <section className="flex flex-col gap-1 rounded-card bg-background-soft p-5">
        <p className="m-0 font-semibold">{details.name}</p>
        <p className="m-0 text-sm text-text-muted">{details.email} · {details.phone}</p>
        <p className="m-0 text-sm text-text-muted">
          {isDelivery ? `${t("deliveryOption")}: ${address}` : t("pickupOption")}
        </p>
        {details.rentalDate && (
          <p className="m-0 text-sm text-text-muted">{t("rentalDateLine", { date: details.rentalDate })}</p>
        )}
        {details.notes && <p className="m-0 text-sm text-text-muted">{details.notes}</p>}
        <button type="button" onClick={onEdit} className="mt-2 self-start text-sm text-primary underline">
          {t("editDetails")}
        </button>
      </section>

      <OrderLineSummary lines={lines} perEventLabel={t("perEvent")} />

      <OrderTotals subtotal={subtotal.toFixed(2)} delivery={delivery} estimated />
      {delivery === "pending" && <p className="m-0 text-right text-sm text-text-muted">{t("feePendingAtCheckout")}</p>}

      {unavailableNames && unavailableNames.length > 0 && (
        <Notice>
          {t("itemsUnavailable")}
          <ul>
            {unavailableNames.map((name, index) => (
              <li key={`${name}:${index}`}>{name}</li>
            ))}
          </ul>
        </Notice>
      )}
      {belowMinimumLines && belowMinimumLines.length > 0 && (
        <Notice>
          {t("belowMinimum")}
          <ul>
            {belowMinimumLines.map((line) => (
              <li key={`${line.name}:${line.optionLabel ?? ""}`}>
                {line.optionLabel ? `${line.name} (${line.optionLabel})` : line.name}
                {line.minQuantity > 1 && ` — ${t("minimumOrder", { count: line.minQuantity })}`}
              </li>
            ))}
          </ul>
          <ButtonLink href="/cart" variant="secondary" className="mt-2">
            {t("updateCart")}
          </ButtonLink>
        </Notice>
      )}
      {submitError && <Notice>{t("submitError")}</Notice>}
      {feeChangedMessage && (
        <div role="alert">
          <Notice>{feeChangedMessage}</Notice>
        </div>
      )}

      <Button onClick={onPlaceOrder} disabled={submitting} className="self-end">
        {submitting ? t("placingOrder") : t("placeOrder")}
      </Button>
    </div>
  );
}
