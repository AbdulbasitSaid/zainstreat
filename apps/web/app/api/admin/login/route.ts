import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();
  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });

  const responseBody = await apiResponse.text();
  const response = new NextResponse(responseBody, {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });

  for (const cookie of apiResponse.headers.getSetCookie()) {
    response.headers.append("Set-Cookie", cookie);
  }

  return response;
}
