// Phase 33 Decision 16: the admin Privacy tool's data, mirroring
// apps/api/src/routes/admin/privacy.rs.

export interface PrivacyOrder {
  id: number;
  created_at: string;
  status: string;
  total: string;
}

export interface PrivacyBooking {
  id: number;
  event_id: number;
  event_slug: string;
  created_at: string;
  status: string;
  party_size: number;
}

export interface PrivacyEnquiry {
  id: number;
  created_at: string;
  event_type: string;
  quote_status: string | null;
}

export interface PrivacyContactMessage {
  id: number;
  created_at: string;
  subject: string;
}

export interface PrivacyBlocking {
  orders: number[];
  bookings: number[];
}

export interface PrivacySearchResult {
  orders: PrivacyOrder[];
  bookings: PrivacyBooking[];
  enquiries: PrivacyEnquiry[];
  contact_messages: PrivacyContactMessage[];
  blocking: PrivacyBlocking;
}

export type AnonymiseResult =
  | { kind: "done"; counts: Record<string, number> }
  | { kind: "blocked"; blocking: PrivacyBlocking }
  | { kind: "mismatch" }
  | { kind: "failed" };

export function exportUrl(email: string): string {
  return `/api/admin/privacy/export?email=${encodeURIComponent(email)}`;
}

export async function searchPrivacy(email: string): Promise<PrivacySearchResult | null> {
  const response = await fetch(`/api/admin/privacy?email=${encodeURIComponent(email)}`, {
    cache: "no-store",
  });
  if (!response.ok) return null;
  return (await response.json()) as PrivacySearchResult;
}

export async function anonymiseEmail(email: string, confirmEmail: string): Promise<AnonymiseResult> {
  const response = await fetch("/api/admin/privacy/anonymise", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, confirm_email: confirmEmail }),
  });
  if (response.ok) {
    return { kind: "done", counts: (await response.json()) as Record<string, number> };
  }
  if (response.status === 409) {
    const body = (await response.json()) as { current: PrivacyBlocking };
    return { kind: "blocked", blocking: body.current };
  }
  if (response.status === 400) return { kind: "mismatch" };
  return { kind: "failed" };
}

export function isEmpty(result: PrivacySearchResult): boolean {
  return (
    result.orders.length === 0 &&
    result.bookings.length === 0 &&
    result.enquiries.length === 0 &&
    result.contact_messages.length === 0
  );
}
