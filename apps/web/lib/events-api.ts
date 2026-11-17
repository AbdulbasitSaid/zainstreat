import { isOrderToken } from "@/lib/orders";
import type { EventList, PublicBooking, PublicEvent } from "@/lib/events";
import { previewFetchInit } from "@/lib/preview";

// Server-only fetchers for the public event pages (Phase 31); the types and
// client helpers stay in lib/events.ts.
const API_BASE_URL = process.env.API_BASE_URL;

function apiBase(): string {
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }
  return API_BASE_URL;
}

export async function getEvents(locale: string): Promise<EventList> {
  const response = await fetch(`${apiBase()}/api/events?locale=${locale}`, { cache: "no-store" });
  if (!response.ok) throw new Error(`Events request failed with status ${response.status}`);
  return (await response.json()) as EventList;
}

// In draft mode (the cms. preview host), unpublished events show too
// (Decision 18); without an admin session the preview call 401s and this
// falls back to the published event.
export async function getEvent(slug: string, locale: string): Promise<PublicEvent | null> {
  const path = `/events/${encodeURIComponent(slug)}?locale=${locale}`;
  const init = await previewFetchInit();
  if (init) {
    const preview = await fetch(`${apiBase()}/api/admin/preview${path}`, init);
    if (preview.ok) return (await preview.json()) as PublicEvent;
  }
  const response = await fetch(`${apiBase()}/api${path}`, { cache: "no-store" });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Event request failed with status ${response.status}`);
  return (await response.json()) as PublicEvent;
}

export async function getBooking(token: string, locale: string): Promise<PublicBooking | null> {
  if (!isOrderToken(token)) return null;
  const response = await fetch(`${apiBase()}/api/events/bookings/${token}?locale=${locale}`, { cache: "no-store" });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Booking request failed with status ${response.status}`);
  return (await response.json()) as PublicBooking;
}
