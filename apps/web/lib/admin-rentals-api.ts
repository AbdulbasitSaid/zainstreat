import { adminApiJson } from "@/lib/admin-api";
import type { AdminRental } from "@/lib/admin-rentals";

export function getAdminRentals(): Promise<AdminRental[]> {
  return adminApiJson<AdminRental[]>("/api/admin/rentals");
}

export function getAdminRental(id: number): Promise<AdminRental> {
  return adminApiJson<AdminRental>(`/api/admin/rentals/${id}`);
}
