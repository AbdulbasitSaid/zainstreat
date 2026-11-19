import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function POST(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/delivery/quote", "POST");
}
