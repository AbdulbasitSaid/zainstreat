import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

// Same-origin proxy for the booking form (Phase 31 Decision 17).
export async function POST(request: NextRequest, { params }: { params: Promise<{ slug: string }> }) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const { slug } = await params;
  // Forwarded so the API's per-IP rate limit sees the visitor, not this proxy.
  const forwardedFor = request.headers.get("x-forwarded-for");

  const apiResponse = await fetch(`${API_BASE_URL}/api/events/${encodeURIComponent(slug)}/bookings`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(forwardedFor ? { "X-Forwarded-For": forwardedFor } : {}),
    },
    body: await request.text(),
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
