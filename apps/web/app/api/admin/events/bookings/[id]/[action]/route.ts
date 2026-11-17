import { NextResponse, type NextRequest } from "next/server";
import { proxyAdminRequest } from "@/lib/admin-proxy";

const ACTIONS = new Set(["offer", "cancel"]);

export async function POST(request: NextRequest, { params }: { params: Promise<{ id: string; action: string }> }) {
  const { id, action } = await params;
  if (!ACTIONS.has(action)) return NextResponse.json({ error: "not_found" }, { status: 404 });
  return proxyAdminRequest(request, `/api/admin/events/bookings/${encodeURIComponent(id)}/${action}`, "POST");
}
