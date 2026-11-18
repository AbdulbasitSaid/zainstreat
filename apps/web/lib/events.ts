import type { FieldError, OrderResponse } from "@/lib/orders";

// GET /api/events (Phase 31 Decision 12). "Past" is computed by the API
// from the clock.
export interface EventCardData {
  slug: string;
  title: string;
  summary: string;
  starts_at: string;
  ends_at: string | null;
  area: string;
  cover_image_url: string | null;
  capacity: number;
  seats_left: number;
  rsvp_open: boolean;
  has_gallery: boolean;
}

export interface EventList {
  upcoming: EventCardData[];
  past: EventCardData[];
}

// A plate's option group, e.g. "Protein: assorted meat or fish" (Phase 32),
// with the fewest and most picks per guest (Phase 35 Decision 7).
export interface EventOptionGroup {
  key: string;
  label: string;
  choices: { key: string; label: string }[];
  min: number;
  max: number;
}

export type OptionRuleKey = "chooseOne" | "chooseExactly" | "chooseUpTo" | "chooseRange" | "chooseOptional";

// The `EventOptions` message and values that state a group's limits
// (Phase 35 Decision 10): "Choose your protein", "Side: choose up to 3".
export function optionRule(group: Pick<EventOptionGroup, "label" | "min" | "max">): {
  key: OptionRuleKey;
  values: { group: string; count: number; min: number; max: number };
} {
  const values = { group: group.label, count: group.max, min: group.min, max: group.max };
  if (group.min === 1 && group.max === 1) return { key: "chooseOne", values: { ...values, group: group.label.toLowerCase() } };
  if (group.min === group.max) return { key: "chooseExactly", values };
  if (group.min === 0) return { key: "chooseOptional", values };
  if (group.min === 1) return { key: "chooseUpTo", values };
  return { key: "chooseRange", values };
}

// Exactly-one groups are radios; every other group is limited checkboxes.
export function isPickOne(group: Pick<EventOptionGroup, "min" | "max">): boolean {
  return group.min === 1 && group.max === 1;
}

export interface EventPlate {
  id: number;
  name: string;
  description: string;
  price: string;
  options: EventOptionGroup[];
}

export interface EventPhoto {
  image_url: string;
  caption: string | null;
}

// GET /api/events/{slug}. Never carries the private address (Decision 10).
export interface PublicEvent {
  slug: string;
  title: string;
  summary: string;
  story: string;
  terms: string;
  recap: string;
  starts_at: string;
  ends_at: string | null;
  area: string;
  cover_image_url: string | null;
  capacity: number;
  max_party_size: number;
  seats_left: number;
  meal_orders_close_at: string | null;
  // The earlier of the meal and booking cutoffs (Phase 35 Decision 4).
  bookings_close_at: string | null;
  meals_open: boolean;
  rsvp_open: boolean;
  is_past: boolean;
  published: boolean;
  plates: EventPlate[];
  photos: EventPhoto[];
}

export const BOOKING_STATUSES = ["waitlisted", "awaiting_payment", "confirmed", "cancelled"] as const;
export type BookingStatus = (typeof BOOKING_STATUSES)[number];

export interface BookingGuest {
  plate_name: string | null;
  // "Fried plantain, Chicken"; empty when the plate has no options.
  choices: string;
  unit_price: string | null;
}

// GET /api/events/bookings/{token} (Decision 11).
export interface PublicBooking {
  token: string;
  status: BookingStatus;
  cancel_reason: "expired" | "admin" | "guest" | null;
  customer_name: string;
  party_size: number;
  notes: string | null;
  created_at: string;
  hold_expires_at: string | null;
  guest_can_cancel: boolean;
  event: {
    slug: string;
    title: string;
    starts_at: string;
    area: string;
    cover_image_url: string | null;
    terms: string;
  };
  guests: BookingGuest[];
  total: string;
  // Only once confirmed.
  address: string | null;
  // The meals' order: the invoice, payment reference and "I've paid".
  order: OrderResponse | null;
}

export interface CreateBookingRequest {
  customer_name: string;
  customer_email: string;
  customer_phone: string;
  // One plate per person (Phase 35 Decision 3). `choices` maps each of the
  // plate's group keys to the picked choice keys.
  guests: { plate_id: number; choices: Record<string, string[]> }[];
  notes: string | null;
  locale: string;
  accept_terms: boolean;
  join_waitlist: boolean;
  website: string;
}

export type CreateBookingResult =
  | { ok: true; token: string; status: BookingStatus }
  | { ok: false; kind: "not_enough_seats"; seatsLeft: number }
  | { ok: false; kind: "booking_closed" }
  | { ok: false; kind: "validation_error"; fields: FieldError[] }
  | { ok: false; kind: "unknown" };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isFieldError(value: unknown): value is FieldError {
  return isRecord(value) && typeof value.field === "string" && typeof value.message === "string";
}

export async function createBooking(slug: string, payload: CreateBookingRequest): Promise<CreateBookingResult> {
  const response = await fetch(`/api/events/${encodeURIComponent(slug)}/bookings`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  const body: unknown = await response.json().catch(() => null);

  if (response.ok && isRecord(body) && typeof body.token === "string") {
    return { ok: true, token: body.token, status: body.status as BookingStatus };
  }
  if (response.status === 409 && isRecord(body)) {
    if (body.error === "not_enough_seats" && typeof body.seats_left === "number") {
      return { ok: false, kind: "not_enough_seats", seatsLeft: body.seats_left };
    }
    if (body.error === "booking_closed") return { ok: false, kind: "booking_closed" };
  }
  if (response.status === 400 && isRecord(body) && Array.isArray(body.fields) && body.fields.every(isFieldError)) {
    return { ok: false, kind: "validation_error", fields: body.fields };
  }
  return { ok: false, kind: "unknown" };
}

// "27 Dec" + "14:00" in the visitor's locale, always on Amsterdam time
// (the server renders in UTC; see lib/format.ts).
export function eventDateParts(iso: string, locale: string) {
  const date = new Date(iso);
  const zone = "Europe/Amsterdam";
  return {
    day: new Intl.DateTimeFormat(locale, { timeZone: zone, day: "numeric" }).format(date),
    month: new Intl.DateTimeFormat(locale, { timeZone: zone, month: "short" }).format(date).replace(".", ""),
    weekday: new Intl.DateTimeFormat(locale, { timeZone: zone, weekday: "long" }).format(date),
    long: new Intl.DateTimeFormat(locale, { timeZone: zone, dateStyle: "full" }).format(date),
    time: new Intl.DateTimeFormat(locale, { timeZone: zone, hour: "2-digit", minute: "2-digit" }).format(date),
  };
}
