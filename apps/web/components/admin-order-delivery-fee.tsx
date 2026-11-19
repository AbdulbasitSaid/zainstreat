"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { AddressCombobox } from "@/components/address-combobox";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { fetchQuote, type AddressSuggestion, type DeliveryQuote } from "@/lib/delivery";
import { formatPrice } from "@/lib/format";

const MONEY_PATTERN = /^\d{1,3}(\.\d{1,2})?$/;

/**
 * Phase 37 Decision 17 (and open risk 6): set or correct the delivery fee
 * on an unpaid delivery order. Look the address up to calculate it, or type
 * a fee. Saving recomputes the total and emails the customer the updated
 * invoice.
 */
export function AdminOrderDeliveryFee({
  orderId,
  updatedAt,
  pending,
  currentFee,
}: {
  orderId: number;
  updatedAt: string;
  pending: boolean;
  currentFee: string;
}) {
  const router = useRouter();
  const [open, setOpen] = useState(pending);
  const [address, setAddress] = useState<AddressSuggestion | null>(null);
  const [quote, setQuote] = useState<DeliveryQuote | null>(null);
  const [fee, setFee] = useState(pending ? "" : currentFee);
  const [notice, setNotice] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function pick(suggestion: AddressSuggestion | null) {
    setAddress(suggestion);
    setQuote(null);
    if (!suggestion) return;
    const result = await fetchQuote("/api/admin/delivery", suggestion.id);
    if (result.ok) {
      setQuote(result.quote);
      setFee(result.quote.fee);
      setNotice(null);
    } else {
      setNotice(
        result.reason === "lookup_down"
          ? "Address lookup isn't answering. Type the fee instead."
          : result.reason === "unavailable"
            ? "Set the business address in Settings first, or type the fee."
            : "Couldn't calculate for that address. Type the fee instead.",
      );
    }
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const normalized = fee.trim().replace(",", ".");
    if (!MONEY_PATTERN.test(normalized)) {
      setNotice("Enter a fee from 0 to 500, with at most two decimals.");
      return;
    }
    setNotice(null);
    setSubmitting(true);
    const response = await fetch(`/api/admin/orders/${orderId}/delivery-fee`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: updatedAt, fee: normalized, address_id: quote ? address?.id : null }),
    });
    setSubmitting(false);
    if (response.ok) {
      setOpen(false);
      router.refresh();
      return;
    }
    setNotice(
      response.status === 409
        ? "This order changed since you opened it (or the customer has said they paid). Reload the page."
        : "Could not save the fee. Please try again.",
    );
  }

  if (!open) {
    return (
      <button type="button" onClick={() => setOpen(true)} className="self-start text-sm text-primary underline">
        Change delivery fee
      </button>
    );
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="flex max-w-[32rem] flex-col gap-4 rounded-card bg-background-soft p-5">
      <h2 className="m-0 text-lg">{pending ? "Delivery fee needed" : "Change delivery fee"}</h2>
      <p className="m-0 text-sm text-text-muted">
        {pending
          ? "Address lookup was down when this order was placed, so the customer can't pay yet. "
          : ""}
        Saving emails the customer an updated invoice with the new total.
      </p>
      {notice && <Notice>{notice}</Notice>}
      <AddressCombobox
        id={`order-${orderId}-fee-address`}
        apiBase="/api/admin/delivery"
        labels={{
          label: "Look up the address (optional)",
          hint: "Pick the customer's address to calculate the fee.",
          searching: "Searching…",
          noMatches: "No Dutch address matches.",
        }}
        onSelect={(suggestion) => void pick(suggestion)}
      />
      {quote && (
        <p className="m-0 text-sm" aria-live="polite">
          {quote.distance_km} km → {formatPrice(quote.fee)}
        </p>
      )}
      <div>
        <label htmlFor={`order-${orderId}-fee`} className="mb-1.5 block text-sm font-semibold">
          Delivery fee (€)
        </label>
        <input
          id={`order-${orderId}-fee`}
          inputMode="decimal"
          className="field max-w-[10rem]"
          value={fee}
          onChange={(event) => setFee(event.target.value)}
        />
      </div>
      <div className="flex gap-3">
        <Button type="submit" disabled={submitting}>
          {submitting ? "Saving…" : "Save and email the customer"}
        </Button>
        {!pending && (
          <Button type="button" variant="secondary" onClick={() => setOpen(false)}>
            Cancel
          </Button>
        )}
      </div>
    </form>
  );
}
