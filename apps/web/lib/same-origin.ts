import { NextResponse, type NextRequest } from "next/server";

/**
 * Rejects cross-site writes to the admin API proxies (Phase 33 Decision 7).
 * The public host and cms. are the same *site*, so SameSite cookies alone
 * don't stop a script on one from posting to the other. Compares hosts,
 * not origins: behind Caddy the request reaches Next over plain http.
 */
export function rejectCrossSite(request: NextRequest): NextResponse | null {
  if (request.method === "GET" || request.method === "HEAD") return null;

  const origin = request.headers.get("origin");
  const host = request.headers.get("host");
  let sameOrigin: boolean;
  if (origin) {
    try {
      sameOrigin = new URL(origin).host === host;
    } catch {
      sameOrigin = false;
    }
  } else {
    // Browsers that omit Origin still send Sec-Fetch-Site on fetches.
    sameOrigin = request.headers.get("sec-fetch-site") === "same-origin";
  }

  return sameOrigin ? null : NextResponse.json({ error: "forbidden" }, { status: 403 });
}
