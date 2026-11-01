import { revalidateTag } from "next/cache";
import type { NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

interface PublishResponse {
  published: { kind: string; id: number | null }[];
}

export async function POST(request: NextRequest) {
  const response = await proxyAdminRequest(request, "/api/admin/publish", "POST");

  // Visitors see published site content on their next load instead of
  // after getSiteContent()'s 300 s fallback (Phase 25 plan 9.8). Menu
  // reads are no-store already.
  if (response.ok) {
    const body = (await response.clone().json()) as PublishResponse;
    if (body.published.some((entry) => entry.kind === "site_content")) {
      revalidateTag("site-content", { expire: 0 });
    }
  }
  return response;
}
