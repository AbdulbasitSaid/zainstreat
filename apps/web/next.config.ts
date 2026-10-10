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
      // (requirement.md Decision 2) — DOMAIN is passed into the web
      // service's environment for exactly this (docker-compose.prod.yml).
      { protocol: "http", hostname: "localhost", port: "8080" },
      ...(process.env.DOMAIN
        ? [{ protocol: "https" as const, hostname: `api.${process.env.DOMAIN}` }]
        : []),
    ],
    formats: ["image/avif", "image/webp"],
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
