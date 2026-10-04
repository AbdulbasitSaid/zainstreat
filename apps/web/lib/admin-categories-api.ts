import { adminApiJson } from "@/lib/admin-api";
import type { AdminCategory } from "@/lib/admin-categories";

export function getAdminCategories(): Promise<AdminCategory[]> {
  return adminApiJson<AdminCategory[]>("/api/admin/categories");
}
