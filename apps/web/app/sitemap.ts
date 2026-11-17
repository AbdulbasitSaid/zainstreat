import type { MetadataRoute } from "next";
import { routing } from "@/i18n/routing";
import { getEvents } from "@/lib/events-api";
import { PUBLIC_ROUTES, languageAlternates } from "@/lib/seo";

// Rebuilt on request, so newly published events appear (Phase 31 Decision 16).
export const dynamic = "force-dynamic";

async function eventSlugs(): Promise<string[]> {
  try {
    const { upcoming, past } = await getEvents(routing.defaultLocale);
    return [...upcoming, ...past].map((event) => event.slug);
  } catch {
    // The sitemap still lists the static pages if the API is down.
    return [];
  }
}

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";
  const slugs = await eventSlugs();

  const pages = routing.locales.flatMap((locale) =>
    PUBLIC_ROUTES.map((route) => ({
      url: `${base}/${locale}${route}`,
      lastModified: new Date(),
      alternates: { languages: languageAlternates(route, base) },
    })),
  );
  const events = routing.locales.flatMap((locale) =>
    slugs.map((slug) => ({
      url: `${base}/${locale}/events/${slug}`,
      lastModified: new Date(),
      alternates: {
        languages: {
          ...Object.fromEntries(routing.locales.map((l) => [l, `${base}/${l}/events/${slug}`])),
          "x-default": `${base}/${routing.defaultLocale}/events/${slug}`,
        },
      },
    })),
  );
  return [...pages, ...events];
}
