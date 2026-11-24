import { NextResponse, type NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

// Phase 38 Decisions 22, 24: close and reopen bookings by hand.
const ACTIONS = new Set(["publish", "unpublish", "archive", "unarchive", "close", "reopen"]);

export async function POST(request: NextRequest, { params }: { params: Promise<{ id: string; action: string }> }) {
  const { id, action } = await params;
  if (!ACTIONS.has(action)) return NextResponse.json({ error: "not_found" }, { status: 404 });
  return proxyAdminRequest(request, `/api/admin/events/${encodeURIComponent(id)}/${action}`, "POST");
}
