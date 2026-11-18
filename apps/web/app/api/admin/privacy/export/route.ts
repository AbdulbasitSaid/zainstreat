import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

// The GDPR export download (Phase 33 Decision 16). Not proxyAdminRequest:
// the filename header has to come through, like the guest list CSV.
export async function GET(request: NextRequest) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }
  const email = request.nextUrl.searchParams.get("email") ?? "";
  const apiResponse = await fetch(
    `${API_BASE_URL}/api/admin/privacy/export?email=${encodeURIComponent(email)}`,
    { headers: { Cookie: request.headers.get("cookie") ?? "" }, cache: "no-store" },
  );
  return new NextResponse(await apiResponse.text(), {
    status: apiResponse.status,
    headers: {
      "Content-Type": "application/json",
      ...(apiResponse.ok
        ? { "Content-Disposition": apiResponse.headers.get("content-disposition") ?? "attachment" }
        : {}),
    },
  });
}
