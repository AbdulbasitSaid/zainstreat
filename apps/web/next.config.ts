import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");

const nextConfig: NextConfig = {
  output: "standalone",
  agentRules: false,
  images: {
    remotePatterns: [
      { protocol: "https", hostname: "placehold.co" },
      // Dev: the API container's published port, same convention default
      // as API_PORT everywhere else. Prod: api.$DOMAIN, Caddy-fronted
      // (requirement.md Decision 2). DOMAIN must be set at `next build`
      // (Dockerfile build arg from CI's vars.DOMAIN, Phase 19): standalone
      // output saves this config at build time, so a runtime-only DOMAIN
      // never reaches the allowlist.
      { protocol: "http", hostname: "localhost", port: "8080" },
      ...(process.env.DOMAIN
        ? [{ protocol: "https" as const, hostname: `api.${process.env.DOMAIN}` }]
        : []),
    ],
    // WebP only: AVIF encoding through sharp is CPU/memory-heavy and was
    // a likely trigger of the 2026-10-11 outage (Phase 18 Decision 5).
    formats: ["image/webp"],
    // 30 days. Menu photos are served immutable by the API
    // (apps/api/src/routes/media.rs) and static images only change on a
    // deploy, so re-encoding sooner is wasted CPU (Phase 18).
    minimumCacheTTL: 2592000,
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
