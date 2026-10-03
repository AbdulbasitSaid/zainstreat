import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/logout`, {
    method: "POST",
    headers: { Cookie: request.headers.get("cookie") ?? "" },
  });

  const response = new NextResponse(null, { status: apiResponse.status });

  for (const cookie of apiResponse.headers.getSetCookie()) {
    response.headers.append("Set-Cookie", cookie);
  }

  return response;
}
