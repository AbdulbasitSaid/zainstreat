import { NextResponse, type NextRequest } from "next/server";
import { isOrderToken } from "@/lib/orders";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ token: string }> },
) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const { token } = await params;
  if (!isOrderToken(token)) {
    return NextResponse.json({ error: "not_found" }, { status: 404 });
  }

  // Forwarded so the API's per-IP rate limit sees the visitor, not this proxy.
  const forwardedFor = request.headers.get("x-forwarded-for");

  const apiResponse = await fetch(`${API_BASE_URL}/api/orders/by-token/${token}/payment-claim`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(forwardedFor ? { "X-Forwarded-For": forwardedFor } : {}),
    },
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
