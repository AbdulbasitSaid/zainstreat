import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ kind: string; id: string }> },
) {
  const { kind, id } = await params;
  return proxyAdminRequest(
    request,
    `/api/admin/drafts/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`,
    "DELETE",
  );
}
