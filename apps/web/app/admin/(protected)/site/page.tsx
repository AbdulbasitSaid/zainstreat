import type { Metadata } from "next";
import { adminApiJson } from "@/lib/admin-api";
import { AdminPreviewPanel } from "@/components/admin-preview-panel";
import {
  AdminSiteContentEditor,
  type AdminSiteContentState,
} from "@/components/admin-site-content-editor";
import type { AppLocale } from "@/i18n/routing";
import { HOME_TEXT_KEYS, type HomeTextKey, type SiteImageSlot } from "@/lib/site-content";
import en from "@/messages/en.json";
import nl from "@/messages/nl.json";
import fr from "@/messages/fr.json";
import hero from "@/assets/images/hero.jpg";
import categoryMeals from "@/assets/images/category-meals.jpg";
import categorySnacks from "@/assets/images/category-snacks.jpg";
import categoryCatering from "@/assets/images/category-catering.jpg";
import categoryEventRentals from "@/assets/images/category-event-rentals.jpg";
import aboutHero from "@/assets/images/about-hero.jpg";
import aboutMission from "@/assets/images/about-mission.jpg";
import contact from "@/assets/images/contact.jpg";
import logo from "@/public/logo.png";

export const metadata: Metadata = {
  title: "Site content | Zain's Admin",
};

const DEFAULT_IMAGES: Record<SiteImageSlot, string> = {
  logo: logo.src,
  hero: hero.src,
  category_meals: categoryMeals.src,
  category_snacks: categorySnacks.src,
  category_catering: categoryCatering.src,
  category_event_rentals: categoryEventRentals.src,
  about_hero: aboutHero.src,
  about_mission: aboutMission.src,
  contact: contact.src,
};

function homeTexts(messages: typeof en): Record<HomeTextKey, string> {
  return Object.fromEntries(HOME_TEXT_KEYS.map((key) => [key, messages.HomePage[key]])) as Record<
    HomeTextKey,
    string
  >;
}

const DEFAULT_TEXTS: Record<AppLocale, Record<HomeTextKey, string>> = {
  en: homeTexts(en),
  nl: homeTexts(nl),
  fr: homeTexts(fr),
};

export default async function AdminSiteContentPage() {
  const initial = await adminApiJson<AdminSiteContentState>("/api/admin/site-content");

  return (
    <AdminPreviewPanel previewPath="">
      <div className="flex flex-col gap-8">
        <h1 className="m-0">Site content</h1>
        {/* Remounts from fresh values after a publish or discard. */}
        <AdminSiteContentEditor
          key={initial.draft_updated_at ?? "published"}
          initial={initial}
          defaultImages={DEFAULT_IMAGES}
          defaultTexts={DEFAULT_TEXTS}
        />
      </div>
    </AdminPreviewPanel>
  );
}
