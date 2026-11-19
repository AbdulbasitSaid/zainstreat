import type { NextRequest } from "next/server";
import { proxyPublicRequest } from "@/lib/public-proxy";

export async function POST(request: NextRequest) {
  return proxyPublicRequest(request, "/api/delivery/quote", "POST");
}
