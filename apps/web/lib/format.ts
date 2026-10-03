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
