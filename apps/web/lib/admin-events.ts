// The admin Events panel's shapes (Phase 31 Decision 18), mirroring
// apps/api/src/routes/admin/events.rs. Times are Europe/Amsterdam
// wall-clock strings (`YYYY-MM-DDTHH:MM`), what <input type="datetime-local"> uses.

export const EVENT_LOCALES = ["en", "nl", "fr"] as const;
export type EventLocale = (typeof EVENT_LOCALES)[number];

export const EVENT_TEXT_KEYS = ["title", "summary", "story", "terms", "recap"] as const;
export type EventTextKey = (typeof EVENT_TEXT_KEYS)[number];
export type EventContent = Record<EventLocale, Record<EventTextKey, string>>;

export const PLATE_TEXT_KEYS = ["name", "description"] as const;
export type PlateTextKey = (typeof PLATE_TEXT_KEYS)[number];
export type PlateContent = Record<EventLocale, Record<PlateTextKey, string>>;

export interface AdminPlate {
  id: number;
  content: PlateContent;
  price: string;
  is_available: boolean;
}

export interface AdminPhoto {
  id: number;
  image_url: string;
  caption: string | null;
}

export interface AdminEvent {
  id: number;
  slug: string;
  content: EventContent;
  starts_at: string;
  ends_at: string | null;
  meal_orders_close_at: string | null;
  rsvp_closes_at: string | null;
  starts_at_utc: string;
  area: string;
  address: string;
  capacity: number;
  max_party_size: number;
  payment_hold_hours: number;
  cover_image_url: string | null;
  published: boolean;
  archived: boolean;
  is_past: boolean;
  has_bookings: boolean;
  seats_taken: number;
  plates: AdminPlate[];
  photos: AdminPhoto[];
}

export interface AdminEventSummary {
  id: number;
  slug: string;
  title: string;
  starts_at: string;
  published: boolean;
  archived: boolean;
  is_past: boolean;
  capacity: number;
  seats_taken: number;
  waitlist_count: number;
  paid_total: string;
}

export interface PlateInput {
  id: number | null;
  content: PlateContent;
  price: string;
  is_available: boolean;
}

export interface EventInput {
  slug: string | null;
  content: EventContent;
  starts_at: string;
  ends_at: string | null;
  area: string;
  address: string;
  capacity: number;
  max_party_size: number;
  meal_orders_close_at: string | null;
  rsvp_closes_at: string | null;
  payment_hold_hours: number;
  cover_image_url: string | null;
  plates: PlateInput[];
}

export type AdminBookingStatus = "waitlisted" | "awaiting_payment" | "confirmed" | "cancelled";

export interface AdminBooking {
  id: number;
  token: string;
  status: AdminBookingStatus;
  cancel_reason: "expired" | "admin" | "guest" | null;
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  party_size: number;
  notes: string | null;
  locale: string;
  created_at: string;
  hold_expires_at: string | null;
  order_id: number | null;
  order_status: string | null;
  payment_reference: string | null;
  total: string;
  guests: { plate_name: string | null }[];
}

export interface PlateCount {
  plate_name: string | null;
  confirmed: number;
  held: number;
}

export interface AdminBookings {
  bookings: AdminBooking[];
  plate_counts: PlateCount[];
  capacity: number;
  seats_taken: number;
}

export const BOOKING_STATUS_LABELS: Record<AdminBookingStatus, string> = {
  waitlisted: "Waitlist",
  awaiting_payment: "Awaiting payment",
  confirmed: "Confirmed",
  cancelled: "Cancelled",
};

export function emptyContent<K extends string>(keys: readonly K[]): Record<EventLocale, Record<K, string>> {
  const fields = () => Object.fromEntries(keys.map((key) => [key, ""])) as Record<K, string>;
  return { en: fields(), nl: fields(), fr: fields() };
}

export function toInput(event: AdminEvent): EventInput {
  return {
    slug: event.slug,
    content: event.content,
    starts_at: event.starts_at,
    ends_at: event.ends_at,
    area: event.area,
    address: event.address,
    capacity: event.capacity,
    max_party_size: event.max_party_size,
    meal_orders_close_at: event.meal_orders_close_at,
    rsvp_closes_at: event.rsvp_closes_at,
    payment_hold_hours: event.payment_hold_hours,
    cover_image_url: event.cover_image_url,
    plates: event.plates.map((plate) => ({
      id: plate.id,
      content: plate.content,
      price: plate.price,
      is_available: plate.is_available,
    })),
  };
}

// The public site's origin, from the cms. host the admin is served on
// (Phase 21 Decision 11): cms.zainstreat.com → zainstreat.com.
export function publicOrigin(): string {
  return `${window.location.protocol}//${window.location.host.replace(/^cms\./, "")}`;
}
