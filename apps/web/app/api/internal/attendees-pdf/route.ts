import { createHash, timingSafeEqual } from "node:crypto";
import { NextResponse, type NextRequest } from "next/server";
import type { AdminAttendees } from "@/lib/admin-events";
import { attendeePdfFilename, loadPdfLogo, renderAttendeePdf } from "@/lib/attendee-pdf";
import { getSiteContent } from "@/lib/api";
import { siteImage } from "@/lib/site-content";

export const runtime = "nodejs";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isAttendees(body: unknown): body is AdminAttendees {
  if (!isRecord(body) || !isRecord(body.event)) return false;
  const { event } = body;
  return (
    typeof event.slug === "string" &&
    typeof event.title === "string" &&
    typeof event.starts_at_label === "string" &&
    typeof event.area === "string" &&
    Array.isArray(body.guests) &&
    body.guests.every(isRecord) &&
    Array.isArray(body.plate_counts) &&
    body.plate_counts.every(isRecord) &&
    typeof body.confirmed_guests === "number"
  );
}

// Hashing first gives equal lengths, which timingSafeEqual requires.
function sameToken(given: string, expected: string): boolean {
  const digest = (value: string) => createHash("sha256").update(value).digest();
  return timingSafeEqual(digest(given), digest(expected));
}

/**
 * Phase 38 Decision 29: the API posts the attendee JSON it is emailing and
 * gets the PDF back. Render only, no database. Off (404) without
 * INTERNAL_RENDER_TOKEN; Caddy also refuses /api/internal/* publicly.
 */
export async function POST(request: NextRequest) {
  const token = process.env.INTERNAL_RENDER_TOKEN;
  if (!token) return NextResponse.json({ error: "not_found" }, { status: 404 });
  const given = request.headers.get("authorization")?.replace(/^Bearer /, "") ?? "";
  if (!sameToken(given, token)) return NextResponse.json({ error: "unauthorized" }, { status: 401 });

  const body: unknown = await request.json().catch(() => null);
  if (!isAttendees(body)) return NextResponse.json({ error: "invalid_body" }, { status: 400 });

  const logo = await loadPdfLogo(siteImage(await getSiteContent(), "logo"));
  const pdf = await renderAttendeePdf(body, logo);
  return new NextResponse(new Uint8Array(pdf), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="${attendeePdfFilename(body)}"`,
    },
  });
}
