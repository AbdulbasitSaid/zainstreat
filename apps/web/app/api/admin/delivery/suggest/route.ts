import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function GET(request: NextRequest) {
  const q = request.nextUrl.searchParams.get("q") ?? "";
  return proxyAdminRequest(request, `/api/admin/delivery/suggest?q=${encodeURIComponent(q)}`, "GET");
}
