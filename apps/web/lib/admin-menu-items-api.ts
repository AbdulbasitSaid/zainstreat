import { adminApiJson } from "@/lib/admin-api";
import type { AdminMenuItem } from "@/lib/admin-menu-items";

export function getAdminMenuItems(): Promise<AdminMenuItem[]> {
  return adminApiJson<AdminMenuItem[]>("/api/admin/menu-items");
}

export function getAdminMenuItem(id: number): Promise<AdminMenuItem> {
  return adminApiJson<AdminMenuItem>(`/api/admin/menu-items/${id}`);
}
