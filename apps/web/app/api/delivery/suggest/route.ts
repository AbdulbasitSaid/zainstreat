import type { NextRequest } from "next/server";
import { proxyPublicRequest } from "@/lib/public-proxy";

export async function GET(request: NextRequest) {
  const q = request.nextUrl.searchParams.get("q") ?? "";
  return proxyPublicRequest(request, `/api/delivery/suggest?q=${encodeURIComponent(q)}`, "GET");
}
