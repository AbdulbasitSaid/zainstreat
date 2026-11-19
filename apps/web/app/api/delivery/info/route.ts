import type { NextRequest } from "next/server";
import { proxyPublicRequest } from "@/lib/public-proxy";

export async function GET(request: NextRequest) {
  return proxyPublicRequest(request, "/api/delivery/info", "GET");
}
