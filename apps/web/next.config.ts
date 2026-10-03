import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");

const nextConfig: NextConfig = {
  output: "standalone",
  agentRules: false,
  images: {
    // Dev seed data's placeholder host. The production/MinIO host gets
    // added alongside this entry in Phase 11 — don't remove it then.
    remotePatterns: [{ protocol: "https", hostname: "placehold.co" }],
    // placehold.co serves image/svg+xml, which next/image's optimizer
    // blocks by default (SVGs can embed scripts). Sandboxed CSP below is
    // Next's own documented mitigation — allows rendering without
    // executing anything the SVG might contain.
    dangerouslyAllowSVG: true,
    contentDispositionType: "attachment",
    contentSecurityPolicy: "default-src 'self'; script-src 'none'; sandbox;",
  },
};

export default withNextIntl(nextConfig);
