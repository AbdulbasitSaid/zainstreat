import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  const body = await request.text();

  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/categories`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Cookie: request.headers.get("cookie") ?? "",
    },
    body,
  });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": "application/json" },
  });
}
