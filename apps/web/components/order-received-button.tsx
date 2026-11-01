"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { CUSTOMER_COMPLETABLE, type FulfilmentType, type OrderStatus } from "@/lib/orders";

// The customer's "I've collected / received it" (Phase 22 Decision 8). Two
// clicks, like the admin's cancel, since it can only be undone by the owner.
// On-site catering confirms the event was catered (Phase 23 Decision 12).
export function OrderReceivedButton({
  token,
  status,
  deliveryType,
}: {
  token: string;
  status: OrderStatus;
  deliveryType: FulfilmentType;
}) {
  const t = useTranslations("Order.tracking");
  const router = useRouter();
  const [confirming, setConfirming] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!CUSTOMER_COMPLETABLE.includes(status)) {
    return null;
  }

  const label = {
    pickup: t("receivedPickup"),
    delivery: t("receivedDelivery"),
    on_site: t("receivedOnSite"),
  }[deliveryType];

  async function handleConfirm() {
    setSubmitting(true);
    setError(null);

    const response = await fetch(`/api/orders/${token}/received`, { method: "POST" });

    setSubmitting(false);
    setConfirming(false);

    if (response.status === 409) {
      setError(t("movedOn"));
      return;
    }
    if (!response.ok) {
      setError(t("receivedError"));
      return;
    }

    router.refresh();
  }

  return (
    <div className="flex flex-col items-center gap-3">
      {confirming ? (
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Button type="button" onClick={handleConfirm} disabled={submitting}>
            {submitting ? t("receiving") : deliveryType === "on_site" ? t("receivedOnSiteConfirm") : t("receivedConfirm")}
          </Button>
          <button
            type="button"
            className="text-sm font-semibold text-text-muted underline"
            disabled={submitting}
            onClick={() => setConfirming(false)}
          >
            {t("receivedKeep")}
          </button>
        </div>
      ) : (
        <Button type="button" variant="secondary" onClick={() => setConfirming(true)}>
          {label}
        </Button>
      )}
      {error && <Notice>{error}</Notice>}
    </div>
  );
}
