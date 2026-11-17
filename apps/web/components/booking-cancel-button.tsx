"use client";

import { useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";

// The guest's "Cancel my booking", behind a confirm dialog (Phase 31
// Decision 14). A paid booking's refund is arranged by the owner.
export function BookingCancelButton({ token, paid }: { token: string; paid: boolean }) {
  const t = useTranslations("BookingPage");
  const router = useRouter();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState(false);

  async function cancel() {
    setSubmitting(true);
    setError(false);
    const response = await fetch(`/api/events/bookings/${token}/cancel`, { method: "POST" });
    setSubmitting(false);
    if (!response.ok) {
      setError(true);
      return;
    }
    dialogRef.current?.close();
    router.refresh();
  }

  return (
    <>
      <Button type="button" variant="secondary" onClick={() => dialogRef.current?.showModal()}>
        {t("cancelButton")}
      </Button>
      <dialog
        ref={dialogRef}
        aria-labelledby="booking-cancel-title"
        className="m-auto w-[min(28rem,92vw)] rounded-card bg-background p-6 text-text backdrop:bg-text/50"
      >
        <h2 id="booking-cancel-title" className="mt-0 text-2xl">
          {t("cancelConfirmTitle")}
        </h2>
        <p>{paid ? t("cancelConfirmPaid") : t("cancelConfirmBody")}</p>
        {error && (
          <p role="alert" className="text-primary">
            {t("cancelError")}
          </p>
        )}
        <div className="mt-4 flex flex-wrap justify-end gap-3">
          <Button type="button" variant="outline" onClick={() => dialogRef.current?.close()}>
            {t("cancelNo")}
          </Button>
          <Button type="button" onClick={() => void cancel()} disabled={submitting}>
            {t("cancelYes")}
          </Button>
        </div>
      </dialog>
    </>
  );
}
