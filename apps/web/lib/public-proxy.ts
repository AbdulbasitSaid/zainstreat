import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

/**
 * Forwards a public request to the Rust API, passing the visitor's IP on
 * for its per-IP rate limits (Phase 37's delivery calculator; the older
 * public proxies are hand-written with the same shape).
 */
export async function proxyPublicRequest(
  request: NextRequest,
  path: string,
  method: "GET" | "POST",
): Promise<NextResponse> {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const forwardedFor = request.headers.get("x-forwarded-for");
  const apiResponse = await fetch(`${API_BASE_URL}${path}`, {
    method,
    headers: {
      "Content-Type": "application/json",
      ...(forwardedFor ? { "X-Forwarded-For": forwardedFor } : {}),
    },
    body: method === "POST" ? await request.text() : undefined,
    cache: "no-store",
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
