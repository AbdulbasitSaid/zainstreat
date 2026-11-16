import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function GET(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/push/public-key", "GET");
}
