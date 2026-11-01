import type { QuoteDisplayStatus } from "@/lib/quotes";

const LABELS: Record<QuoteDisplayStatus, string> = {
  draft: "Draft",
  sent: "Sent",
  expired: "Expired",
  accepted: "Accepted",
  declined: "Declined",
};

// Same colour tokens as AdminStatusBadge; no new ones.
const CLASSES: Record<QuoteDisplayStatus, string> = {
  draft: "bg-background-soft text-text-muted",
  sent: "bg-cream text-text",
  // Waiting on the owner to extend and resend.
  expired: "bg-accent-light text-primary-dark",
  accepted: "bg-whatsapp/15 text-whatsapp-dark",
  declined: "bg-text/10 text-text-muted",
};

export function AdminQuoteStatusBadge({ status }: { status: QuoteDisplayStatus }) {
  return (
    <span className={`inline-block rounded-full px-3 py-1 text-xs font-semibold ${CLASSES[status]}`}>
      {LABELS[status]}
    </span>
  );
}
