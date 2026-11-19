"use client";

import { useMemo, useState, useSyncExternalStore } from "react";
import { useLocale, useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderDetailsForm, type OrderDetailsValues } from "@/components/order-details-form";
import { OrderReview } from "@/components/order-review";
import { OrderSuccess } from "@/components/order-success";
import { useCart } from "@/lib/cart-context";
import type { DeliveryInfo } from "@/lib/delivery";
import { formatPrice } from "@/lib/format";
import {
  belowMinimumFromErrors,
  submitOrder,
  type BelowMinimumLine,
  type FieldError,
  type OrderResponse,
} from "@/lib/orders";

// v2 (Phase 20): orders now carry public_token/payment, so a stored v1 order
// is never shown.
const LAST_ORDER_KEY = "zainstreat:last-order:v2";

type Step = "details" | "review";

function subscribeNoop() {
  return () => {};
}

function getStoredOrderSnapshot(): string | null {
  try {
    return window.sessionStorage.getItem(LAST_ORDER_KEY);
  } catch {
    return null;
  }
}

function getServerOrderSnapshot(): string | null {
  return null;
}

const RENTAL_UNAVAILABLE_FIELD = /^items\[(\d+)\]\.rental_id$/;

// A rejected delivery or rental date sends the customer back to the details
// step with this message (Phase 37 Decisions 4, 8, 17).
function detailsNoticeKey(fields: FieldError[], lookupDown: boolean): string | null {
  const has = (field: string, message?: string) =>
    fields.some((error) => error.field === field && (message === undefined || error.message === message));
  if (has("delivery_type", "unavailable")) return "deliveryNowUnavailable";
  if (has("delivery_address_id", "required") && lookupDown) return "lookupBack";
  if (has("delivery_address_id", "out_of_range")) return "outOfRange";
  if (has("delivery_address_id")) return "deliveryUnavailable";
  if (has("rental_date")) return "rentalDateRejected";
  return null;
}

export function OrderView({ deliveryInfo }: { deliveryInfo: DeliveryInfo | null }) {
  const t = useTranslations("Order");
  const locale = useLocale();
  const { lines, subtotal, hasRentals, clearCart } = useCart();
  const [step, setStep] = useState<Step>("details");
  const [details, setDetails] = useState<OrderDetailsValues | null>(null);
  const [detailsNotice, setDetailsNotice] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [unavailableNames, setUnavailableNames] = useState<string[] | null>(null);
  const [belowMinimumLines, setBelowMinimumLines] = useState<BelowMinimumLine[] | null>(null);
  const [submitError, setSubmitError] = useState(false);
  const [feeChangedTo, setFeeChangedTo] = useState<string | null>(null);
  const [placedOrder, setPlacedOrder] = useState<OrderResponse | null>(null);

  // Recover a just-completed order after a hard refresh (Decision 5) via
  // useSyncExternalStore rather than an effect + setState — sessionStorage
  // is an external store React doesn't own, and this avoids the
  // server/client first-render mismatch an effect-based read would risk.
  const storedOrderRaw = useSyncExternalStore(subscribeNoop, getStoredOrderSnapshot, getServerOrderSnapshot);
  const recoveredOrder = useMemo<OrderResponse | null>(() => {
    if (!storedOrderRaw) return null;
    try {
      return JSON.parse(storedOrderRaw) as OrderResponse;
    } catch {
      return null;
    }
  }, [storedOrderRaw]);

  // Only trust the recovered order while the cart is empty, i.e. we're not
  // mid-way through building a new one.
  const confirmedOrder = placedOrder ?? (lines.length === 0 ? recoveredOrder : null);

  function backToDetails(next: OrderDetailsValues, noticeKey: string) {
    setDetails(next);
    setDetailsNotice(t(noticeKey));
    setStep("details");
  }

  async function handlePlaceOrder(values: OrderDetailsValues) {
    setSubmitting(true);
    setUnavailableNames(null);
    setBelowMinimumLines(null);
    setSubmitError(false);
    setFeeChangedTo(null);

    const delivery = values.deliveryType === "delivery";
    const result = await submitOrder({
      customer_name: values.name,
      customer_email: values.email,
      customer_phone: values.phone,
      delivery_type: values.deliveryType,
      delivery_address: delivery && values.addressLookupDown ? values.deliveryAddress : null,
      delivery_address_id: delivery && !values.addressLookupDown ? values.deliveryAddressId : null,
      delivery_address_extra: delivery && values.deliveryAddressExtra.trim() ? values.deliveryAddressExtra.trim() : null,
      expected_delivery_fee: delivery && !values.addressLookupDown ? (values.deliveryQuote?.fee ?? null) : null,
      rental_date: hasRentals ? values.rentalDate : null,
      notes: values.notes.trim() ? values.notes : null,
      items: lines.map((line) =>
        line.kind === "rental"
          ? { rental_id: line.itemId, quantity: line.quantity }
          : { menu_item_id: line.itemId, price_option_id: line.priceOptionId, quantity: line.quantity },
      ),
      locale,
    });

    setSubmitting(false);

    if (result.ok) {
      setPlacedOrder(result.order);
      clearCart();
      try {
        window.sessionStorage.setItem(LAST_ORDER_KEY, JSON.stringify(result.order));
      } catch {
        // sessionStorage unavailable — confirmation still renders this pass, just won't survive a reload.
      }
      return;
    }

    if (result.kind === "items_unavailable") {
      setUnavailableNames(result.items.map((item) => item.name ?? t("unknownItem")));
      return;
    }

    // Phase 37 Decision 12: the customer sees the new fee before ordering.
    if (result.kind === "delivery_fee_changed" && values.deliveryQuote) {
      setDetails({
        ...values,
        deliveryQuote: { ...values.deliveryQuote, fee: result.fee, distance_km: result.distanceKm },
      });
      setFeeChangedTo(result.fee);
      return;
    }

    // Decision 17: lookup went down between picking and ordering.
    if (result.kind === "lookup_down") {
      backToDetails({ ...values, addressLookupDown: true, deliveryAddressId: null, deliveryQuote: null }, "lookupDown");
      return;
    }

    if (result.kind === "validation_error") {
      const belowMinimum = belowMinimumFromErrors(result.fields, lines);
      if (belowMinimum.length > 0) {
        setBelowMinimumLines(belowMinimum);
        return;
      }
      const rentalNames = result.fields.flatMap((error) => {
        const match = error.message === "unavailable" ? RENTAL_UNAVAILABLE_FIELD.exec(error.field) : null;
        const line = match ? lines[Number(match[1])] : undefined;
        return line ? [line.name] : [];
      });
      if (rentalNames.length > 0) {
        setUnavailableNames(rentalNames);
        return;
      }
      const noticeKey = detailsNoticeKey(result.fields, values.addressLookupDown);
      if (noticeKey === "deliveryNowUnavailable") {
        backToDetails({ ...values, deliveryType: "pickup", deliveryQuote: null }, noticeKey);
        return;
      }
      if (noticeKey === "lookupBack") {
        backToDetails({ ...values, addressLookupDown: false, deliveryAddressId: null, deliveryQuote: null }, noticeKey);
        return;
      }
      if (noticeKey) {
        backToDetails(
          noticeKey === "rentalDateRejected" ? values : { ...values, deliveryAddressId: null, deliveryQuote: null },
          noticeKey,
        );
        return;
      }
    }

    setSubmitError(true);
  }

  if (confirmedOrder) {
    return <OrderSuccess order={confirmedOrder} />;
  }

  if (lines.length === 0) {
    return (
      <div className="flex flex-col items-center gap-6 py-16 text-center">
        <Notice>{t("emptyCart")}</Notice>
        <ButtonLink href="/menu">{t("continueShopping")}</ButtonLink>
      </div>
    );
  }

  if (step === "review" && details) {
    return (
      <OrderReview
        details={details}
        lines={lines.map((line) => ({
          name: line.name,
          optionLabel: line.optionLabel,
          unitPrice: line.unitPrice,
          quantity: line.quantity,
          perEvent: line.kind === "rental",
        }))}
        subtotal={subtotal}
        unavailableNames={unavailableNames}
        belowMinimumLines={belowMinimumLines}
        feeChangedMessage={feeChangedTo ? t("feeChanged", { fee: formatPrice(feeChangedTo) }) : null}
        submitting={submitting}
        submitError={submitError}
        onEdit={() => {
          setDetailsNotice(null);
          setStep("details");
        }}
        onPlaceOrder={() => handlePlaceOrder(details)}
      />
    );
  }

  return (
    <OrderDetailsForm
      initialValues={details}
      deliveryInfo={deliveryInfo}
      hasRentals={hasRentals}
      notice={detailsNotice}
      onSubmit={(values) => {
        setDetails(values);
        setDetailsNotice(null);
        setUnavailableNames(null);
        setBelowMinimumLines(null);
        setSubmitError(false);
        setFeeChangedTo(null);
        setStep("review");
      }}
    />
  );
}
