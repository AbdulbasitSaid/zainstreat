import { isOrderToken, type FulfilmentType } from "@/lib/orders";

// The stored quote statuses plus "expired", which the API computes for a
// sent quote past its valid-until date (Phase 23 Decision 3).
export const QUOTE_DISPLAY_STATUSES = ["draft", "sent", "expired", "accepted", "declined"] as const;

export type QuoteDisplayStatus = (typeof QUOTE_DISPLAY_STATUSES)[number];

// Quote tokens are UUIDs, the same shape as order tokens.
export function isQuoteToken(value: string): boolean {
  return isOrderToken(value);
}

export interface QuoteItem {
  description: string;
  quantity: number;
  unit_price: string;
  subtotal: string;
}

// GET /api/quotes/by-token/{token}. No email or phone: the token is the only
// thing protecting the page.
export interface PublicQuote {
  id: number;
  // Sent back on accept/decline, so a stale page can't accept a revised
  // quote (Phase 23 Decision 13).
  version: number;
  display_status: QuoteDisplayStatus;
  customer_name: string;
  event_type: string;
  event_date: string;
  guest_count: number;
  delivery_type: FulfilmentType;
  delivery_address: string | null;
  notes: string | null;
  items: QuoteItem[];
  total: string;
  valid_until: string;
  order_token: string | null;
}

export interface AdminQuote {
  id: number;
  status: "draft" | "sent" | "accepted" | "declined";
  display_status: QuoteDisplayStatus;
  version: number;
  // The customer's link, in the enquiry's language (Phase 23 Decision 14).
  quote_url: string;
  delivery_type: FulfilmentType;
  delivery_address: string | null;
  notes: string | null;
  total: string;
  valid_until: string;
  sent_at: string | null;
  accepted_at: string | null;
  declined_at: string | null;
  decline_reason: string | null;
  order_id: number | null;
  order_reference: string | null;
  items: QuoteItem[];
}

export interface SendQuoteResponse {
  quote: AdminQuote;
  // false when email is switched off; admin then offers the share panel.
  email_queued: boolean;
}

type ShareLocale = "en" | "nl";

// The prefilled WhatsApp message when the owner shares a quote, in the
// enquiry's language: specs/2026-11-01-phase-23-catering-quotations/copy.md §4.
// Admin sits outside next-intl, so the two variants live here.
const SHARE_MESSAGES: Record<ShareLocale, (name: string, id: number, date: string, url: string) => string> = {
  en: (name, id, date, url) =>
    `Hi ${name}, here is your quote #${id} from Zain's Treat n More for your event on ${date}: ${url}`,
  nl: (name, id, date, url) =>
    `Hoi ${name}, hier is je offerte #${id} van Zain's Treat n More voor je evenement op ${date}: ${url}`,
};

// "2026-12-05" -> "05-12-2026", the format the emails use.
export function formatQuoteDate(isoDate: string): string {
  const [year, month, day] = isoDate.split("-");
  return `${day}-${month}-${year}`;
}

export function quoteShareMessage(
  locale: string,
  customerName: string,
  quoteId: number,
  eventDate: string,
  url: string,
): string {
  const message = SHARE_MESSAGES[locale === "nl" ? "nl" : "en"];
  return message(customerName, quoteId, formatQuoteDate(eventDate), url);
}

// Prices typed in admin, as whole cents, so the live total doesn't drift the
// way float sums do. null for anything that isn't a plain amount with at
// most two decimals.
export function toCents(value: string): number | null {
  const match = /^(\d+)(?:[.,](\d{1,2}))?$/.exec(value.trim());
  if (!match) return null;
  return Number(match[1]) * 100 + Number((match[2] ?? "").padEnd(2, "0"));
}

export function centsToAmount(cents: number): string {
  return (cents / 100).toFixed(2);
}
