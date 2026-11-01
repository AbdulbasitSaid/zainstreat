import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

// Site content has no id: DELETE /api/admin/drafts/site_content.
export async function DELETE(request: NextRequest, { params }: { params: Promise<{ kind: string }> }) {
  const { kind } = await params;
  return proxyAdminRequest(request, `/api/admin/drafts/${encodeURIComponent(kind)}`, "DELETE");
}
