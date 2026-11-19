"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { AddressCombobox } from "@/components/address-combobox";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import type { AdminDeliverySettings } from "@/lib/admin-orders";
import { ESTIMATED_BY_LABELS } from "@/lib/admin-orders";
import { fetchQuote, type AddressSuggestion, type QuoteResult } from "@/lib/delivery";
import { formatPrice } from "@/lib/format";

interface FieldErrorItem {
  field: string;
  message: string;
}

function isValidationBody(body: unknown): body is { fields: FieldErrorItem[] } {
  return typeof body === "object" && body !== null && "fields" in body && Array.isArray(body.fields);
}

const MONEY_PATTERN = /^\d{1,3}(\.\d{1,2})?$/;

const ERROR_TEXT: Record<string, string> = {
  rate_per_km: "Enter a rate from 0 to 100, with at most two decimals.",
  minimum_fee: "Enter a minimum from 0 to 500, with at most two decimals.",
  max_distance_km: "Leave empty for no limit, or enter 1 to 500 km.",
  origin_address_id: "That address couldn't be found. Pick it from the suggestions again.",
  delivery_enabled: "Set the business address first.",
};

const QUOTE_FAILURE_TEXT: Record<Exclude<QuoteResult, { ok: true }>["reason"], string> = {
  out_of_range: "Beyond your maximum distance — customers there can't choose delivery.",
  unavailable: "Set the business address first.",
  lookup_down: "Address lookup isn't answering right now. Try again in a minute.",
  not_found: "That address couldn't be found.",
  error: "Couldn't calculate. Try again.",
};

const SUGGEST_LABELS = { searching: "Searching…", noMatches: "No Dutch address matches." };

/**
 * Phase 37 Decisions 6, 15: the delivery fee settings. Live on save (no
 * draft → publish), guarded by `updated_at` like rentals. The business
 * address is private: it never reaches customers.
 */
export function AdminDeliverySettingsForm({ initialValue }: { initialValue: AdminDeliverySettings }) {
  const router = useRouter();
  const [origin, setOrigin] = useState<AddressSuggestion | null>(null);
  const [rate, setRate] = useState(initialValue.rate_per_km);
  const [minimum, setMinimum] = useState(initialValue.minimum_fee);
  const [maxDistance, setMaxDistance] = useState(initialValue.max_distance_km ?? "");
  const [enabled, setEnabled] = useState(initialValue.delivery_enabled);
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [testResult, setTestResult] = useState<QuoteResult | "loading" | null>(null);

  const originLocated = initialValue.origin_located || origin !== null;

  function fieldError(field: string) {
    return fieldErrors.some((error) => error.field === field) ? ERROR_TEXT[field] : undefined;
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setNotice(null);
    setSaved(false);
    const normalizedRate = rate.trim().replace(",", ".");
    const normalizedMinimum = minimum.trim().replace(",", ".");
    const normalizedMax = maxDistance.trim().replace(",", ".");
    const local: FieldErrorItem[] = [];
    if (!MONEY_PATTERN.test(normalizedRate)) local.push({ field: "rate_per_km", message: "invalid" });
    if (!MONEY_PATTERN.test(normalizedMinimum)) local.push({ field: "minimum_fee", message: "invalid" });
    if (normalizedMax && !/^\d{1,3}(\.\d)?$/.test(normalizedMax)) local.push({ field: "max_distance_km", message: "invalid" });
    if (enabled && !originLocated) local.push({ field: "delivery_enabled", message: "origin_required" });
    setFieldErrors(local);
    if (local.length > 0) return;
    setSubmitting(true);

    const response = await fetch("/api/admin/delivery-settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        updated_at: initialValue.updated_at,
        origin_address_id: origin?.id ?? null,
        rate_per_km: normalizedRate,
        minimum_fee: normalizedMinimum,
        max_distance_km: normalizedMax || null,
        delivery_enabled: enabled,
      }),
    });
    setSubmitting(false);

    if (response.ok) {
      setSaved(true);
      setOrigin(null);
      router.refresh();
      return;
    }
    const body: unknown = await response.json().catch(() => null);
    if (isValidationBody(body)) {
      setFieldErrors(body.fields);
    } else if (response.status === 409) {
      setNotice("Delivery settings were changed elsewhere since you opened this page. Reload to see the latest.");
    } else if (response.status === 503) {
      setNotice("Address lookup isn't answering right now. Try saving again in a minute.");
    } else {
      setNotice("Could not save the delivery settings. Please try again.");
    }
  }

  async function testAddress(suggestion: AddressSuggestion | null) {
    if (!suggestion) {
      setTestResult(null);
      return;
    }
    setTestResult("loading");
    setTestResult(await fetchQuote("/api/admin/delivery", suggestion.id));
  }

  return (
    <div className="flex max-w-[32rem] flex-col gap-8">
      <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-5">
        {notice && <Notice>{notice}</Notice>}
        {saved && <p role="status" className="m-0 font-semibold text-primary">Saved. Customers see the changes straight away.</p>}

        <div>
          <p className="m-0 mb-1.5 text-sm font-semibold">Business address</p>
          <p className="m-0 text-sm">
            {initialValue.origin_address ?? "Not set yet."}
            {initialValue.origin_address && !initialValue.origin_located && (
              <span className="text-primary-dark"> — couldn&apos;t find this on the map. Pick it below.</span>
            )}
          </p>
          <p className="m-0 mt-1 text-xs text-text-muted">
            Private — customers never see this address. Deliveries are measured from here.
          </p>
        </div>

        <div>
          <AddressCombobox
            id="delivery-origin"
            apiBase="/api/admin/delivery"
            labels={{ label: "Change the business address", hint: "Type the street and house number.", ...SUGGEST_LABELS }}
            invalid={!!fieldError("origin_address_id")}
            onSelect={setOrigin}
          />
          {fieldError("origin_address_id") && (
            <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("origin_address_id")}</p>
          )}
        </div>

        <div className="grid gap-5 sm:grid-cols-3">
          <div>
            <label htmlFor="delivery-rate" className="mb-1.5 block text-sm font-semibold">Rate per km (€)</label>
            <input
              id="delivery-rate"
              inputMode="decimal"
              className="field"
              value={rate}
              aria-invalid={fieldError("rate_per_km") ? true : undefined}
              onChange={(event) => setRate(event.target.value)}
            />
          </div>
          <div>
            <label htmlFor="delivery-minimum" className="mb-1.5 block text-sm font-semibold">Minimum fee (€)</label>
            <input
              id="delivery-minimum"
              inputMode="decimal"
              className="field"
              value={minimum}
              aria-invalid={fieldError("minimum_fee") ? true : undefined}
              onChange={(event) => setMinimum(event.target.value)}
            />
          </div>
          <div>
            <label htmlFor="delivery-max" className="mb-1.5 block text-sm font-semibold">Max distance (km)</label>
            <input
              id="delivery-max"
              inputMode="decimal"
              className="field"
              placeholder="No limit"
              value={maxDistance}
              aria-invalid={fieldError("max_distance_km") ? true : undefined}
              onChange={(event) => setMaxDistance(event.target.value)}
            />
          </div>
        </div>
        {["rate_per_km", "minimum_fee", "max_distance_km"].map(
          (field) => fieldError(field) && <p key={field} className="m-0 -mt-3 text-sm text-primary-dark">{fieldError(field)}</p>,
        )}
        <p className="m-0 -mt-2 text-xs text-text-muted">
          Fee = one-way driving km × rate, never below the minimum. Pickup is always free.
        </p>

        <div>
          <label className="flex items-center gap-2 font-semibold">
            <input
              type="checkbox"
              checked={enabled}
              disabled={!originLocated && !enabled}
              onChange={(event) => setEnabled(event.target.checked)}
            />
            Offer delivery at checkout
          </label>
          {!originLocated && <p className="m-0 mt-1 text-sm text-text-muted">Set the business address first.</p>}
          {fieldError("delivery_enabled") && (
            <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("delivery_enabled")}</p>
          )}
        </div>

        <Button type="submit" disabled={submitting} className="self-start">
          {submitting ? "Saving…" : "Save delivery settings"}
        </Button>
      </form>

      <section className="flex flex-col gap-3 rounded-card bg-background-soft p-5">
        <h3 className="m-0 text-lg">Test an address</h3>
        <p className="m-0 text-sm text-text-muted">
          See what a customer would pay, using the saved settings. Works while delivery is off.
        </p>
        <AddressCombobox
          id="delivery-test"
          apiBase="/api/admin/delivery"
          labels={{ label: "Customer address", hint: "Type a street and house number.", ...SUGGEST_LABELS }}
          onSelect={(suggestion) => void testAddress(suggestion)}
        />
        <div aria-live="polite">
          {testResult === "loading" && <p className="m-0 text-sm text-text-muted">Calculating…</p>}
          {testResult && testResult !== "loading" && testResult.ok && (
            <p className="m-0 font-semibold">
              {testResult.quote.distance_km} km · {formatPrice(testResult.quote.fee)}
              <span className="font-normal text-text-muted"> ({ESTIMATED_BY_LABELS[testResult.quote.estimated_by]})</span>
            </p>
          )}
          {testResult && testResult !== "loading" && !testResult.ok && (
            <p className="m-0 text-sm text-primary-dark">{QUOTE_FAILURE_TEXT[testResult.reason]}</p>
          )}
        </div>
      </section>
    </div>
  );
}
