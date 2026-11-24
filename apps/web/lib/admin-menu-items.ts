import type { Translations, TranslationStatusPair } from "@/lib/admin-translations";

export interface AdminPriceOption {
  id: number;
  label: string;
  price: string;
  translations: Translations;
}

export interface AdminMenuItem {
  id: number;
  category_id: number;
  category_name: string;
  name: string;
  description: string | null;
  price: string | null;
  image_url: string | null;
  is_available: boolean;
  is_featured: boolean;
  min_quantity: number;
  is_archived: boolean;
  price_options: AdminPriceOption[];
  updated_at: string;
  // Phase 25 Decision 15: new items start unpublished; a published item's
  // edit waits in `draft` until it's published.
  is_published: boolean;
  has_draft: boolean;
  draft: MenuItemFields | null;
  draft_updated_at: string | null;
  // Phase 38: live nl/fr, and the state of the version the form edits.
  translations: Translations;
  translation_status: TranslationStatusPair;
}

// The editable content: the create body, the draft-save body and the
// stored draft (apps/api's MenuItemFields).
export interface MenuItemFields {
  category_id: number;
  name: string;
  description: string | null;
  price: string | null;
  price_options: { label: string; price: string; translations?: Translations }[];
  image_url: string | null;
  is_featured: boolean;
  // Phase 36 Decision 5; drafts saved before Phase 36 don't have it.
  min_quantity?: number;
  // Phase 38; drafts saved before Phase 38 don't have it.
  translations?: Translations;
}
