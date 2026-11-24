import type { Translations, TranslationStatusPair } from "@/lib/admin-translations";

export interface AdminCategory {
  id: number;
  name: string;
  description: string | null;
  is_archived: boolean;
  updated_at: string;
  // Phase 25 Decision 15, as for menu items.
  is_published: boolean;
  has_draft: boolean;
  draft: CategoryFields | null;
  draft_updated_at: string | null;
  // Phase 38: live nl/fr, and the state of the version the form edits.
  translations: Translations;
  translation_status: TranslationStatusPair;
}

export interface CategoryFields {
  name: string;
  description: string | null;
  translations?: Translations;
}
