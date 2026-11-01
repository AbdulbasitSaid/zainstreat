import type { MetadataRoute } from "next";
import { routing } from "@/i18n/routing";
import { PUBLIC_ROUTES, languageAlternates } from "@/lib/seo";

export default function sitemap(): MetadataRoute.Sitemap {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";

  return routing.locales.flatMap((locale) =>
    PUBLIC_ROUTES.map((route) => ({
      url: `${base}/${locale}${route}`,
      lastModified: new Date(),
      alternates: { languages: languageAlternates(route, base) },
    })),
  );
}
