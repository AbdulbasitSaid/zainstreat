import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

// Phase 33 Decision 16: everything held for one email address.
export async function GET(request: NextRequest) {
  const email = request.nextUrl.searchParams.get("email") ?? "";
  return proxyAdminRequest(request, `/api/admin/privacy?email=${encodeURIComponent(email)}`, "GET");
}
