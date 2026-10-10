import type { MetadataRoute } from "next";
import { routing } from "@/i18n/routing";

const ROUTES = ["", "/about", "/services", "/menu", "/contact", "/terms", "/food-regulations"];

export default function sitemap(): MetadataRoute.Sitemap {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";

  return routing.locales.flatMap((locale) =>
    ROUTES.map((route) => ({
      url: `${base}/${locale}${route}`,
      lastModified: new Date(),
    })),
  );
}
