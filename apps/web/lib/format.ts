export function parsePrice(price: string): number {
  return Number.parseFloat(price);
}

export function formatPrice(price: string): string {
  const value = parsePrice(price);
  return `€${value.toLocaleString("nl-NL", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}

export function formatPriceOrNull(price: string | null): string | null {
  return price === null ? null : formatPrice(price);
}

// Explicit timeZone is load-bearing: the server container runs in UTC and
// the browser in the visitor's zone, so an implicit zone would produce a
// hydration mismatch and a wrong "placed today". en-GB (not formatPrice's
// nl-NL) because the admin surface is English-only.
const DATE_TIME_FORMAT = new Intl.DateTimeFormat("en-GB", {
  timeZone: "Europe/Amsterdam",
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

export function formatDateTime(iso: string): string {
  return DATE_TIME_FORMAT.format(new Date(iso));
}

// A plain date ("2026-12-02") read as UTC, so it's the same day everywhere;
// for the admin, like formatDateTime (Phase 37's rental date).
const PLAIN_DATE_FORMAT = new Intl.DateTimeFormat("en-GB", {
  timeZone: "UTC",
  weekday: "short",
  day: "numeric",
  month: "short",
  year: "numeric",
});

export function formatDate(date: string): string {
  return PLAIN_DATE_FORMAT.format(new Date(`${date}T00:00:00Z`));
}
