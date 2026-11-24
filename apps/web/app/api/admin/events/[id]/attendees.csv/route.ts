import { NextResponse, type NextRequest } from "next/server";

const API_BASE_URL = process.env.API_BASE_URL;

// The attendee list download (Phase 38 Decision 14). Not proxyAdminRequest:
// the body is CSV, and the filename header has to come through.
export async function GET(request: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }
  const { id } = await params;
  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/events/${encodeURIComponent(id)}/attendees.csv`, {
    headers: { Cookie: request.headers.get("cookie") ?? "" },
    cache: "no-store",
  });
  // Bytes, not text(): decoding would drop the BOM Excel needs for accents.
  return new NextResponse(await apiResponse.arrayBuffer(), {
    status: apiResponse.status,
    headers: {
      "Content-Type": apiResponse.headers.get("content-type") ?? "text/csv; charset=utf-8",
      "Content-Disposition": apiResponse.headers.get("content-disposition") ?? "attachment",
    },
  });
}
