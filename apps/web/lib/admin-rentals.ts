// Phase 36 Decisions 8, 9 (apps/api's AdminRentalResponse / RentalFields).
import type { Translations, TranslationStatusPair } from "@/lib/admin-translations";

export interface AdminRental {
  id: number;
  name: string;
  description: string | null;
  price: string;
  image_url: string | null;
  is_available: boolean;
  display_order: number;
  is_archived: boolean;
  updated_at: string;
  // Phase 38: nl/fr, and why DeepL didn't fill them on this save, if not.
  translations: Translations;
  translation_status: TranslationStatusPair;
  translation_notice?: "not_configured" | "disabled" | "paused" | "too_long" | "usage_unknown" | "failed";
}

export interface RentalFields {
  name: string;
  description: string | null;
  price: string;
  image_url: string | null;
  is_available: boolean;
  display_order: number;
  translations: Translations;
}
