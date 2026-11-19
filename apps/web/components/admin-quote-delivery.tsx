"use client";

import { useState } from "react";
import { AddressCombobox } from "@/components/address-combobox";
import { fetchQuote, type DeliveryQuote } from "@/lib/delivery";

/**
 * Phase 37 Decision 14: "Calculate delivery" in the quote editor. The owner
 * picks the venue or delivery address; the editor then adds or updates a
 * "Delivery (… km)" line at the fee, which stays editable.
 */
export function AdminQuoteDelivery({
  address,
  onCalculated,
}: {
  address: string;
  onCalculated: (quote: DeliveryQuote) => void;
}) {
  const [open, setOpen] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  if (!open) {
    return (
      <button type="button" onClick={() => setOpen(true)} className="self-start text-sm font-semibold text-primary underline">
        Calculate delivery
      </button>
    );
  }

  return (
    <div className="flex flex-col gap-2 rounded-card bg-background-soft p-4">
      <AddressCombobox
        id="quote-delivery-address"
        apiBase="/api/admin/delivery"
        initialQuery={address}
        labels={{
          label: "Find the address",
          hint: "Pick the match to add a delivery line.",
          searching: "Searching…",
          noMatches: "No Dutch address matches.",
        }}
        onSelect={async (suggestion) => {
          if (!suggestion) return;
          setMessage("Calculating…");
          const result = await fetchQuote("/api/admin/delivery", suggestion.id);
          if (result.ok) {
            onCalculated(result.quote);
            setMessage(`Delivery line set: ${result.quote.distance_km} km.`);
          } else {
            setMessage(
              result.reason === "unavailable"
                ? "Set the business address in Settings first."
                : result.reason === "out_of_range"
                  ? "Beyond the maximum delivery distance in Settings."
                  : "Couldn't calculate for that address.",
            );
          }
        }}
      />
      {message && (
        <p className="m-0 text-sm text-text-muted" aria-live="polite">
          {message}
        </p>
      )}
    </div>
  );
}
