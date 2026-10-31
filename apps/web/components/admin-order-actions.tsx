"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import type { OrderStatus } from "@/lib/orders";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";

const ACTION_LABELS: Partial<Record<OrderStatus, string>> = {
  processing: "Confirm payment",
  ready_for_collection: "Ready for collection",
  out_for_delivery: "Mark on the way",
  completed: "Mark completed",
};

// Replaces Phase 13's free status dropdown: only the next valid step(s) for
// this order's pickup/delivery branch are offered (Phase 20 Decision 12).
// `next` comes from the API's `next_statuses`, never worked out here.
export function AdminOrderActions({
  orderId,
  status,
  next,
  completedByCustomer = false,
}: {
  orderId: number;
  status: OrderStatus;
  next: OrderStatus[];
  completedByCustomer?: boolean;
}) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [confirmingCancel, setConfirmingCancel] = useState(false);
  const [confirmingReopen, setConfirmingReopen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (next.length === 0) {
    return null;
  }

  async function moveTo(target: OrderStatus) {
    setError(null);
    setSaving(true);

    const response = await fetch(`/api/admin/orders/${orderId}/status`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: target }),
    });

    setSaving(false);
    setConfirmingCancel(false);
    setConfirmingReopen(false);

    if (response.status === 409) {
      setError("This order has moved on since the page loaded. Refresh to see its current status.");
      return;
    }
    if (!response.ok) {
      setError("Could not update the order. Please try again.");
      return;
    }

    router.refresh();
  }

  // A customer-completed order's only next step is going back to its ready
  // status, in case they confirmed too early (Phase 22 Decision 8). Secondary
  // and two clicks, like cancel, and the customer isn't emailed.
  if (status === "completed" && completedByCustomer) {
    const [reopenTo] = next;
    return (
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-3">
          {confirmingReopen ? (
            <>
              <Button type="button" variant="secondary" disabled={saving} onClick={() => moveTo(reopenTo)}>
                Confirm reopen?
              </Button>
              <button
                type="button"
                className="text-sm font-semibold text-text-muted underline"
                disabled={saving}
                onClick={() => setConfirmingReopen(false)}
              >
                Keep completed
              </button>
            </>
          ) : (
            <Button type="button" variant="secondary" disabled={saving} onClick={() => setConfirmingReopen(true)}>
              Reopen (customer marked it received)
            </Button>
          )}
        </div>
        {error && <Notice>{error}</Notice>}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {status === "awaiting_payment" && (
        <p className="m-0 text-sm text-text-muted">
          The customer hasn&apos;t said they&apos;ve paid yet. Only confirm once the transfer is in the bank.
        </p>
      )}
      <div className="flex flex-wrap items-center gap-3">
        {next
          .filter((target) => target !== "cancelled")
          .map((target) => (
            <Button key={target} type="button" disabled={saving} onClick={() => moveTo(target)}>
              {ACTION_LABELS[target] ?? target}
            </Button>
          ))}
        {next.includes("cancelled") &&
          (confirmingCancel ? (
            <>
              <Button type="button" variant="secondary" disabled={saving} onClick={() => moveTo("cancelled")}>
                Confirm cancel?
              </Button>
              <button
                type="button"
                className="text-sm font-semibold text-text-muted underline"
                disabled={saving}
                onClick={() => setConfirmingCancel(false)}
              >
                Keep order
              </button>
            </>
          ) : (
            <Button type="button" variant="secondary" disabled={saving} onClick={() => setConfirmingCancel(true)}>
              Cancel order
            </Button>
          ))}
      </div>
      {error && <Notice>{error}</Notice>}
    </div>
  );
}
