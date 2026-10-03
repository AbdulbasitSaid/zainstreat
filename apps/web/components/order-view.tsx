"use client";

import { useMemo, useState, useSyncExternalStore } from "react";
import { useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderDetailsForm, type OrderDetailsValues } from "@/components/order-details-form";
import { OrderReview } from "@/components/order-review";
import { OrderSuccess } from "@/components/order-success";
import { useCart } from "@/lib/cart-context";
import { submitOrder, type OrderResponse, type UnavailableOrderItem } from "@/lib/orders";

const LAST_ORDER_KEY = "zainstreat:last-order:v1";

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

export function OrderView() {
  const t = useTranslations("Order");
  const { lines, subtotal, clearCart } = useCart();
  const [step, setStep] = useState<Step>("details");
  const [details, setDetails] = useState<OrderDetailsValues | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [unavailableItems, setUnavailableItems] = useState<UnavailableOrderItem[] | null>(null);
  const [submitError, setSubmitError] = useState(false);
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

  async function handlePlaceOrder(values: OrderDetailsValues) {
    setSubmitting(true);
    setUnavailableItems(null);
    setSubmitError(false);

    const result = await submitOrder({
      customer_name: values.name,
      customer_email: values.email,
      customer_phone: values.phone,
      delivery_type: values.deliveryType,
      delivery_address: values.deliveryType === "delivery" ? values.deliveryAddress : null,
      notes: values.notes.trim() ? values.notes : null,
      items: lines.map((line) => ({
        menu_item_id: line.itemId,
        price_option_id: line.priceOptionId,
        quantity: line.quantity,
      })),
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
      setUnavailableItems(result.items);
      return;
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
        }))}
        subtotal={subtotal}
        unavailableItems={unavailableItems}
        submitting={submitting}
        submitError={submitError}
        onEdit={() => setStep("details")}
        onPlaceOrder={() => handlePlaceOrder(details)}
      />
    );
  }

  return (
    <OrderDetailsForm
      initialValues={details}
      onSubmit={(values) => {
        setDetails(values);
        setUnavailableItems(null);
        setSubmitError(false);
        setStep("review");
      }}
    />
  );
}
