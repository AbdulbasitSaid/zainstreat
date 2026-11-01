import type { AppLocale } from "@/i18n/routing";

// Phase 25 Decisions 11–13. Mirrors apps/api/src/site_content.rs; a missing
// or null value falls back to the bundled default, so an empty document
// renders the site exactly as before.

export const SITE_IMAGE_SLOTS = [
  "logo",
  "hero",
  "category_meals",
  "category_snacks",
  "category_catering",
  "category_event_rentals",
  "about_hero",
  "about_mission",
  "contact",
] as const;
export type SiteImageSlot = (typeof SITE_IMAGE_SLOTS)[number];

// The editable `HomePage` message keys: all but metaTitle/metaDescription
// (SEO, Phase 26). `pnpm check:messages` keeps this list, en.json's
// HomePage and the Rust HOME_TEXT_KEYS in step.
export const HOME_TEXT_KEYS = [
  "heroEyebrow",
  "heroHeadline",
  "heroSupporting",
  "orderNow",
  "bookCatering",
  "whatsappUs",
  "trustHalal",
  "trustFresh",
  "trustHygienic",
  "trustReliable",
  "trustHalalDesc",
  "trustFreshDesc",
  "trustHygienicDesc",
  "trustReliableDesc",
  "servicesMeals",
  "servicesSnacks",
  "servicesCatering",
  "servicesEventRentals",
  "whyChooseUsEyebrow",
  "whyChooseUs",
  "cateringCtaHeadline",
  "requestCateringQuote",
  "aboutPreview",
  "learnMore",
  "featuredMenuEyebrow",
  "featuredMenuHeading",
  "viewFullMenu",
] as const;
export type HomeTextKey = (typeof HOME_TEXT_KEYS)[number];

// Rendered with t.rich, so these may carry <em>…</em>.
export const RICH_HOME_TEXT_KEYS: readonly HomeTextKey[] = ["heroHeadline", "whyChooseUs"];

export interface SiteContent {
  images?: Partial<Record<SiteImageSlot, string | null>>;
  home_text?: Partial<Record<AppLocale, Partial<Record<HomeTextKey, string>>>>;
}

export const EMPTY_SITE_CONTENT: SiteContent = {};

// Crop aspect per slot for AdminImageUpload: `null` uploads the file as-is
// (the logo keeps its transparency).
export const SITE_IMAGE_ASPECT: Record<SiteImageSlot, number | null> = {
  logo: null,
  hero: 4 / 3,
  category_meals: 1,
  category_snacks: 1,
  category_catering: 1,
  category_event_rentals: 1,
  about_hero: 4 / 3,
  about_mission: 4 / 3,
  contact: 4 / 3,
};

export const SITE_IMAGE_LABELS: Record<SiteImageSlot, string> = {
  logo: "Logo",
  hero: "Homepage hero",
  category_meals: "Category: Meals",
  category_snacks: "Category: Snacks",
  category_catering: "Category: Catering",
  category_event_rentals: "Category: Event rentals",
  about_hero: "Kitchen photo (about + homepage)",
  about_mission: "About: mission",
  contact: "Contact page",
};

export function siteImage(content: SiteContent, slot: SiteImageSlot): string | null {
  return content.images?.[slot] ?? null;
}
