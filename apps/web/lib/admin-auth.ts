import { cookies } from "next/headers";

const API_BASE_URL = process.env.API_BASE_URL;

export interface AdminUser {
  id: number;
  name: string;
  email: string;
  role: string;
}

export async function getCurrentAdmin(): Promise<AdminUser | null> {
  if (!API_BASE_URL) return null;

  const cookieHeader = (await cookies()).toString();
  if (!cookieHeader) return null;

  const response = await fetch(`${API_BASE_URL}/api/admin/me`, {
    headers: { Cookie: cookieHeader },
    cache: "no-store",
  });

  if (!response.ok) return null;
  return (await response.json()) as AdminUser;
}
