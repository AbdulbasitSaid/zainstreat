import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();

  const apiResponse = await fetch(`${API_BASE_URL}/api/orders`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });

  const responseBody = await apiResponse.text();
  return new NextResponse(responseBody, {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
