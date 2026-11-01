import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function PUT(request: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return proxyAdminRequest(request, `/api/admin/categories/${encodeURIComponent(id)}/draft`, "PUT");
}
