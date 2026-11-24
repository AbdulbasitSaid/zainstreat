import { NextResponse, type NextRequest } from "next/server";
import type { AdminAttendees } from "@/lib/admin-events";
import { attendeePdfFilename, loadPdfLogo, renderAttendeePdf } from "@/lib/attendee-pdf";
import { getSiteContent } from "@/lib/api";
import { siteImage } from "@/lib/site-content";

const API_BASE_URL = process.env.API_BASE_URL;

// react-pdf needs Node (fonts from disk, Buffers).
export const runtime = "nodejs";

// Phase 38 Decision 27: the attendee list as a real PDF download. The API
// checks the admin session; its 401 or 404 passes straight through.
export async function GET(request: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  if (!API_BASE_URL) {
    return NextResponse.json({ error: "internal_server_error" }, { status: 500 });
  }
  const { id } = await params;
  const apiResponse = await fetch(`${API_BASE_URL}/api/admin/events/${encodeURIComponent(id)}/attendees`, {
    headers: { Cookie: request.headers.get("cookie") ?? "" },
    cache: "no-store",
  });
  if (!apiResponse.ok) {
    return NextResponse.json({ error: "attendees_unavailable" }, { status: apiResponse.status });
  }
  const data = (await apiResponse.json()) as AdminAttendees;
  const logo = await loadPdfLogo(siteImage(await getSiteContent(), "logo"));
  const pdf = await renderAttendeePdf(data, logo);
  return new NextResponse(new Uint8Array(pdf), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="${attendeePdfFilename(data)}"`,
      "Cache-Control": "no-store",
    },
  });
}
