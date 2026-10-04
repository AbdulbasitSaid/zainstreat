export const ENQUIRIES_PAGE_SIZE = 25;

export interface CateringEnquirySummary {
  id: number;
  name: string;
  event_type: string;
  event_date: string;
  guest_count: number;
  created_at: string;
}

export interface CateringEnquiryDetail extends CateringEnquirySummary {
  phone: string;
  email: string;
  location: string;
  services_required: string[];
  message: string | null;
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
