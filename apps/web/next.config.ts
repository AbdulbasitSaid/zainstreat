import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");

const nextConfig: NextConfig = {
  output: "standalone",
  agentRules: false,
  // The admin runs on cms.localhost:3000 in dev (proxy.ts, Phase 21); let
  // it load dev-only assets (HMR) like the main localhost origin does.
  allowedDevOrigins: ["cms.localhost"],
  images: {
    remotePatterns: [
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
    // No dangerouslyAllowSVG: the optimizer rejects SVGs, which closes the
    // SVG remote-code-execution advisory class (Phase 33 Decision 1).
  },
};

export default withNextIntl(nextConfig);
