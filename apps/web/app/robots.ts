import type { MetadataRoute } from "next";

export default function robots(): MetadataRoute.Robots {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";

  return {
    // "/*/order/" matches the per-order token pages (/en/order/<token>), not
    // the /en/order checkout page (Phase 20 Decision 4).
    rules: { userAgent: "*", allow: "/", disallow: ["/admin", "/api", "/*/order/"] },
    sitemap: `${base}/sitemap.xml`,
  };
}
