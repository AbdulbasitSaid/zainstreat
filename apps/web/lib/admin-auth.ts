import { adminApiFetch } from "@/lib/admin-api";

export interface AdminUser {
  id: number;
  name: string;
  email: string;
  role: string;
}

export async function getCurrentAdmin(): Promise<AdminUser | null> {
  try {
    const response = await adminApiFetch("/api/admin/me");
    if (!response.ok) return null;
    return (await response.json()) as AdminUser;
  } catch {
    return null;
  }
}
