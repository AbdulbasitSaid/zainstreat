"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import type { OrderStatus } from "@/lib/orders";

// The customer's "I've paid" (Phase 20 Decision 7). The API is idempotent,
// so a stale page pressing it again is harmless.
export function PaymentClaimButton({ token, status }: { token: string; status: OrderStatus }) {
  const t = useTranslations("Order");
  const router = useRouter();
  const [claimed, setClaimed] = useState(status === "payment_claimed");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState(false);

  if (claimed) {
    return <Notice>{t("claimThanks")}</Notice>;
  }

  if (status !== "awaiting_payment") {
    return null;
  }

  async function handleClaim() {
    setSubmitting(true);
    setError(false);

    const response = await fetch(`/api/orders/${token}/payment-claim`, { method: "POST" });

    setSubmitting(false);

    if (!response.ok) {
      setError(true);
      return;
    }

    setClaimed(true);
    router.refresh();
  }

  return (
    <div className="flex flex-col items-center gap-3">
      <Button type="button" onClick={handleClaim} disabled={submitting}>
        {submitting ? t("claiming") : t("iPaid")}
      </Button>
      {error && <Notice>{t("claimError")}</Notice>}
    </div>
  );
}
