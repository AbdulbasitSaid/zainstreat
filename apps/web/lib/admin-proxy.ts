import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

/**
 * Forwards a same-origin admin request to the Rust API with the session
 * cookie, like the hand-written app/api/admin/* proxies (Phase 25's draft
 * and publish endpoints all share this shape).
 */
export async function proxyAdminRequest(
  request: NextRequest,
  path: string,
  method: "GET" | "PUT" | "POST" | "DELETE",
): Promise<NextResponse> {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const apiResponse = await fetch(`${API_BASE_URL}${path}`, {
    method,
    headers: {
      "Content-Type": "application/json",
      Cookie: request.headers.get("cookie") ?? "",
    },
    body: method === "PUT" || method === "POST" ? await request.text() : undefined,
    cache: "no-store",
  });

  // 204 must not carry a body.
  if (apiResponse.status === 204) return new NextResponse(null, { status: 204 });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
