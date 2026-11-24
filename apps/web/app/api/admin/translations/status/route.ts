import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

// Phase 38: DeepL usage and the translation settings.
export async function GET(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/translations/status", "GET");
}
