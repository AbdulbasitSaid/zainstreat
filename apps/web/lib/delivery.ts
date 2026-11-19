// Phase 37: the delivery calculator (requirement.md Decisions 8–12, 17).
// The same calls serve checkout (`/api/delivery`) and the admin
// (`/api/admin/delivery`), which may quote while delivery is switched off.

export type DeliveryApiBase = "/api/delivery" | "/api/admin/delivery";

export interface DeliveryInfo {
  enabled: boolean;
  rate_per_km: string;
  minimum_fee: string;
  max_distance_km: string | null;
}

export interface AddressSuggestion {
  id: string;
  label: string;
}

export type EstimatedBy = "route" | "straight_line" | "manual";

export interface DeliveryQuote {
  address_id: string;
  address: string;
  distance_km: string;
  fee: string;
  estimated_by: EstimatedBy;
}

export type SuggestResult =
  | { ok: true; suggestions: AddressSuggestion[] }
  // PDOK down, or the request failed: checkout falls back to a typed address.
  | { ok: false; reason: "lookup_down" | "aborted" };

export type QuoteFailure = "out_of_range" | "unavailable" | "lookup_down" | "not_found" | "error";

export type QuoteResult = { ok: true; quote: DeliveryQuote } | { ok: false; reason: QuoteFailure };

export const SUGGEST_MIN_CHARS = 3;

function errorCode(body: unknown): string | null {
  if (typeof body !== "object" || body === null) return null;
  const error = (body as Record<string, unknown>).error;
  return typeof error === "string" ? error : null;
}

function isSuggestion(value: unknown): value is AddressSuggestion {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === "string" && typeof v.label === "string";
}

export async function fetchSuggestions(
  base: DeliveryApiBase,
  query: string,
  signal: AbortSignal,
): Promise<SuggestResult> {
  try {
    const response = await fetch(`${base}/suggest?q=${encodeURIComponent(query)}`, { signal });
    const body: unknown = await response.json().catch(() => null);
    if (response.ok && Array.isArray(body) && body.every(isSuggestion)) {
      return { ok: true, suggestions: body };
    }
    // A rate-limited or rejected query isn't an outage; show no matches.
    if (response.status === 400 || response.status === 429) return { ok: true, suggestions: [] };
    return { ok: false, reason: "lookup_down" };
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return { ok: false, reason: "aborted" };
    return { ok: false, reason: "lookup_down" };
  }
}

export async function fetchQuote(base: DeliveryApiBase, addressId: string): Promise<QuoteResult> {
  try {
    const response = await fetch(`${base}/quote`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ address_id: addressId }),
    });
    const body: unknown = await response.json().catch(() => null);
    if (response.ok) return { ok: true, quote: body as DeliveryQuote };
    switch (errorCode(body)) {
      case "out_of_range":
        return { ok: false, reason: "out_of_range" };
      case "delivery_unavailable":
        return { ok: false, reason: "unavailable" };
      case "address_lookup_down":
        return { ok: false, reason: "lookup_down" };
      case "not_found":
        return { ok: false, reason: "not_found" };
      default:
        return { ok: false, reason: "error" };
    }
  } catch {
    return { ok: false, reason: "error" };
  }
}
