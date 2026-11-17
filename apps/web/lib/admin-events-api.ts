import { adminApiFetch, adminApiJson } from "@/lib/admin-api";
import type { AdminBookings, AdminEvent, AdminEventSummary } from "@/lib/admin-events";

export function getAdminEvents(): Promise<AdminEventSummary[]> {
  return adminApiJson<AdminEventSummary[]>("/api/admin/events");
}

// null for an unknown id, so the page can 404.
export async function getAdminEvent(id: number): Promise<AdminEvent | null> {
  const response = await adminApiFetch(`/api/admin/events/${id}`);
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Admin event request failed with status ${response.status}`);
  return (await response.json()) as AdminEvent;
}

export function getAdminEventBookings(id: number): Promise<AdminBookings> {
  return adminApiJson<AdminBookings>(`/api/admin/events/${id}/bookings`);
}
