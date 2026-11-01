import { draftMode } from "next/headers";
import { NextResponse, type NextRequest } from "next/server";
import { getCurrentAdmin } from "@/lib/admin-auth";
import { isPreviewLocale, normalizePreviewPath } from "@/lib/preview-paths";

/**
 * Turns on Next's draft mode for the live preview (Phase 25 Decision 21),
 * then redirects into /preview/{locale}{path}. cms host only, admin
 * session required; the draft-mode cookie is host-only on cms.
 */
export async function GET(request: NextRequest) {
  // proxy.ts already 404s this on the public host; belt and braces.
  if (!(request.headers.get("host") ?? "").startsWith("cms.")) {
    return new NextResponse(null, { status: 404 });
  }

  const locale = request.nextUrl.searchParams.get("locale");
  const path = normalizePreviewPath(request.nextUrl.searchParams.get("path"));
  if (!isPreviewLocale(locale) || path === null) {
    return NextResponse.json({ error: "bad_request" }, { status: 400 });
  }

  if (!(await getCurrentAdmin())) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  (await draftMode()).enable();
  // Relative, so it stays on whatever host and scheme the browser used.
  return new NextResponse(null, { status: 307, headers: { Location: `/preview/${locale}${path}` } });
}
