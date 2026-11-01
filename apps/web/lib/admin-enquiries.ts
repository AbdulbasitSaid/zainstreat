import type { EmailDelivery } from "@/components/admin-email-delivery-notice";
import type { AdminQuote, QuoteDisplayStatus } from "@/lib/quotes";

export const ENQUIRIES_PAGE_SIZE = 25;

export interface CateringEnquirySummary {
  id: number;
  name: string;
  event_type: string;
  event_date: string;
  guest_count: number;
  created_at: string;
  // null until the owner starts a quote (Phase 23 Decision 8).
  quote_status: QuoteDisplayStatus | null;
}

export interface CateringEnquiryDetail extends Omit<CateringEnquirySummary, "quote_status"> {
  phone: string;
  email: string;
  location: string;
  services_required: string[];
  message: string | null;
  locale: string;
  quote: AdminQuote | null;
  // The quote email's outcome (Phase 25 Decision 6a).
  customer_email_delivery: EmailDelivery | null;
}

export interface CateringEnquiryList {
  enquiries: CateringEnquirySummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface ContactMessageSummary {
  id: number;
  name: string;
  subject: string;
  created_at: string;
}

export interface ContactMessageDetail extends ContactMessageSummary {
  email: string;
  phone: string;
  message: string;
}

export interface ContactMessageList {
  messages: ContactMessageSummary[];
  total: number;
  limit: number;
  offset: number;
}
