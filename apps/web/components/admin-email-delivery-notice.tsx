import { Notice } from "@/components/notice";
import { buttonClasses } from "@/components/button";
import { formatDateTime } from "@/lib/format";
import { buildWhatsAppLinkTo, toWhatsAppNumber } from "@/lib/whatsapp";

// apps/api's EmailDeliveryStatus (Phase 25 Decision 6a).
export interface EmailDelivery {
  status: "queued" | "sent" | "failed";
  kind: "invoice" | "status" | "quote";
  error: string | null;
  at: string;
  // Failed, or still queued five minutes on (worked out by the API).
  needs_follow_up: boolean;
}

const KIND_LABELS: Record<EmailDelivery["kind"], string> = {
  invoice: "invoice",
  status: "status update",
  quote: "quote",
};

/**
 * "Customer email may not have been delivered" (Phase 25 Decision 6a): the
 * latest customer email failed (e.g. SES sandbox rejected the address), or
 * was never confirmed. Offers WhatsApp as the follow-up.
 */
export function AdminEmailDeliveryNotice({
  delivery,
  phone,
  whatsAppMessage,
}: {
  delivery: EmailDelivery | null;
  phone: string;
  whatsAppMessage: string;
}) {
  if (!delivery?.needs_follow_up) return null;
  const unconfirmed = delivery.status === "queued";

  const whatsAppNumber = toWhatsAppNumber(phone);

  return (
    <Notice className="flex flex-col gap-3">
      <span>
        Customer email {unconfirmed ? "not confirmed" : "may not have been delivered"} (
        {KIND_LABELS[delivery.kind]}, {formatDateTime(delivery.at)}). Follow up on WhatsApp
        {delivery.kind === "quote" ? " or copy the quote link below" : ""}.
      </span>
      {delivery.error && <span className="text-sm text-text-muted">{delivery.error}</span>}
      {whatsAppNumber && (
        <a
          href={buildWhatsAppLinkTo(whatsAppNumber, whatsAppMessage)}
          target="_blank"
          rel="noopener noreferrer"
          className={buttonClasses({ variant: "outline", color: "whatsapp", className: "self-start" })}
        >
          Message on WhatsApp
        </a>
      )}
    </Notice>
  );
}
