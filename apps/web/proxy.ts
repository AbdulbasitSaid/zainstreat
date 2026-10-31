import createMiddleware from "next-intl/middleware";
import { NextResponse, type NextRequest } from "next/server";
import { routing } from "./i18n/routing";

const intlMiddleware = createMiddleware(routing);

const ADMIN_PREFIX = "/admin";

// The admin is served from cms.<host> (Phase 21 Decision 11): the same app,
// rewritten onto app/admin/**. cms.zainstreat.com in production,
// cms.<ip>.sslip.io on staging, cms.localhost:3000 in dev. On the public
// host, old /admin links redirect there and the admin API proxies are
// hidden.
export default function proxy(request: NextRequest) {
  const host = request.headers.get("host") ?? "";
  const { pathname, search } = request.nextUrl;

  if (host.startsWith("cms.")) {
    // Same-origin route handlers (app/api/admin/*) stay where they are.
    if (pathname.startsWith("/api/")) return NextResponse.next();
    const url = request.nextUrl.clone();
    url.pathname = pathname === "/" ? ADMIN_PREFIX : `${ADMIN_PREFIX}${pathname}`;
    return NextResponse.rewrite(url);
  }

  if (pathname === ADMIN_PREFIX || pathname.startsWith(`${ADMIN_PREFIX}/`)) {
    // Caddy terminates TLS, so the scheme comes from X-Forwarded-Proto.
    const proto =
      request.headers.get("x-forwarded-proto") ?? request.nextUrl.protocol.replace(":", "");
    const adminPath = pathname.slice(ADMIN_PREFIX.length) || "/";
    return NextResponse.redirect(`${proto}://cms.${host}${adminPath}${search}`, 308);
  }

  if (pathname.startsWith("/api/admin/")) {
    return new NextResponse(null, { status: 404 });
  }

  if (pathname.startsWith("/api/")) return NextResponse.next();

  return intlMiddleware(request);
}

export const config = {
  // Runs on /api and /admin too (unlike plain next-intl), so the cms host
  // rewrite and the public-host redirect/404 above can see them.
  matcher: ["/((?!_next|_vercel|.*\\..*).*)"],
};
