import type { Metadata } from "next";
import { routing } from "@/i18n/routing";

// The indexable public pages, "" for home. The sitemap lists exactly these,
// and each one's generateMetadata uses localeAlternates (Phase 24 Decision 8).
export const PUBLIC_ROUTES = ["", "/about", "/services", "/menu", "/contact", "/terms", "/food-regulations"] as const;

export type PublicRoute = (typeof PUBLIC_ROUTES)[number];

// path -> { en: "/en/about", nl: ..., fr: ..., "x-default": "/en/about" },
// prefixed with `base` (absolute for the sitemap, "" for metadata).
export function languageAlternates(path: PublicRoute, base = ""): Record<string, string> {
  return {
    ...Object.fromEntries(routing.locales.map((l) => [l, `${base}/${l}${path}`])),
    "x-default": `${base}/${routing.defaultLocale}${path}`,
  };
}

// Canonical + hreflang for a public page, relative to the layout's metadataBase.
export function localeAlternates(locale: string, path: PublicRoute): Metadata["alternates"] {
  return {
    canonical: `/${locale}${path}`,
    languages: languageAlternates(path),
  };
}
