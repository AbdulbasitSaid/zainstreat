import { defineRouting } from "next-intl/routing";

export const routing = defineRouting({
  locales: ["en", "nl", "fr"],
  defaultLocale: "en",
  // hreflang comes from page metadata and the sitemap (lib/seo.ts) instead;
  // the middleware's Link header would add a second x-default pointing at "/".
  alternateLinks: false,
});

export type AppLocale = (typeof routing.locales)[number];
