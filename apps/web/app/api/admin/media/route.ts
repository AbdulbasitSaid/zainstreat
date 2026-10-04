import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

export async function POST(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }

  // Unlike every other proxy in this directory, this one forwards a body it
  // never parses: `request.body` carries the multipart stream straight
  // through with the inbound Content-Type (which holds the boundary) —
  // reading it as `.text()` first would corrupt the binary body.
  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/media`, {
    method: "POST",
    headers: {
      "Content-Type": request.headers.get("content-type") ?? "",
      Cookie: request.headers.get("cookie") ?? "",
    },
    body: request.body,
    // `duplex` is required by undici for a streamed request body but isn't
    // in the Fetch API's TS types yet.
    duplex: "half",
  } as RequestInit & { duplex: "half" });

  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: { "Content-Type": apiResponse.headers.get("content-type") ?? "application/json" },
  });
}
