"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { useRouter } from "@/i18n/navigation";

type Mode = "idle" | "accepting" | "declining";

// The customer's accept/decline on a sent quote (Phase 23 Decision 4). Each
// needs a second click. `version` is the one this page rendered, so a page
// left open while the owner revised the quote can't accept the new version
// (Decision 13).
export function QuoteActions({ token, version }: { token: string; version: number }) {
  const t = useTranslations("Quote");
  const router = useRouter();
  const [mode, setMode] = useState<Mode>("idle");
  const [reason, setReason] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function post(action: "accept" | "decline", body: Record<string, unknown>): Promise<Response | null> {
    setSubmitting(true);
    setError(null);
    try {
      return await fetch(`/api/quotes/${token}/${action}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version, ...body }),
      });
    } catch {
      return null;
    } finally {
      setSubmitting(false);
    }
  }

  async function showError(response: Response | null) {
    if (response?.status === 409) {
      const body: unknown = await response.json().catch(() => null);
      const code = typeof body === "object" && body !== null ? (body as Record<string, unknown>).error : null;
      // Expired: refresh so the page shows the expired state and contact link.
      if (code === "quote_expired") {
        router.refresh();
        return;
      }
      setError(t("errorChanged"));
      return;
    }
    setError(t("errorGeneric"));
  }

  async function handleAccept() {
    const response = await post("accept", {});
    if (!response?.ok) {
      await showError(response);
      return;
    }
    const body = (await response.json()) as { order_token: string };
    router.push(`/order/${body.order_token}`);
  }

  async function handleDecline() {
    const trimmed = reason.trim();
    const response = await post("decline", { reason: trimmed === "" ? null : trimmed });
    if (!response?.ok) {
      await showError(response);
      return;
    }
    router.refresh();
  }

  return (
    <div className="flex flex-col items-center gap-4">
      {mode === "idle" && (
        <>
          <p className="m-0 text-sm text-text-muted">{t("acceptHint")}</p>
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Button type="button" onClick={() => setMode("accepting")}>
              {t("accept")}
            </Button>
            <Button type="button" variant="secondary" onClick={() => setMode("declining")}>
              {t("decline")}
            </Button>
          </div>
        </>
      )}

      {mode === "accepting" && (
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Button type="button" onClick={handleAccept} disabled={submitting} aria-describedby="quote-accept-hint">
            {t("acceptConfirm")}
          </Button>
          <button
            type="button"
            className="text-sm font-semibold text-text-muted underline"
            disabled={submitting}
            onClick={() => setMode("idle")}
          >
            {t("keep")}
          </button>
          <p id="quote-accept-hint" className="m-0 w-full text-sm text-text-muted">
            {t("acceptHint")}
          </p>
        </div>
      )}

      {mode === "declining" && (
        <div className="flex w-full flex-col gap-3 text-left">
          <label htmlFor="quote-decline-reason" className="text-sm font-semibold">
            {t("declineReasonLabel")}
          </label>
          <textarea
            id="quote-decline-reason"
            value={reason}
            maxLength={500}
            rows={3}
            placeholder={t("declineReasonPlaceholder")}
            onChange={(event) => setReason(event.target.value)}
            className="w-full rounded-card border border-text/20 bg-card p-3"
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button type="button" variant="secondary" onClick={handleDecline} disabled={submitting}>
              {t("declineConfirm")}
            </Button>
            <button
              type="button"
              className="text-sm font-semibold text-text-muted underline"
              disabled={submitting}
              onClick={() => setMode("idle")}
            >
              {t("keep")}
            </button>
          </div>
        </div>
      )}

      {error && <Notice>{error}</Notice>}
    </div>
  );
}
