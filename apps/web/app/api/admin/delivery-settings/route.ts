import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function GET(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/delivery-settings", "GET");
}

export async function PUT(request: NextRequest) {
  return proxyAdminRequest(request, "/api/admin/delivery-settings", "PUT");
}
