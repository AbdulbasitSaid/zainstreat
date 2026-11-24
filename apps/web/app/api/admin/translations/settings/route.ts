import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function PUT(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/translations/settings", "PUT");
}
