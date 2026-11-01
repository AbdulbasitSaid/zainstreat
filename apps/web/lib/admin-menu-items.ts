export interface AdminPriceOption {
  id: number;
  label: string;
  price: string;
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
  is_archived: boolean;
  price_options: AdminPriceOption[];
  updated_at: string;
  // Phase 25 Decision 15: new items start unpublished; a published item's
  // edit waits in `draft` until it's published.
  is_published: boolean;
  has_draft: boolean;
  draft: MenuItemFields | null;
  draft_updated_at: string | null;
}

// The editable content: the create body, the draft-save body and the
// stored draft (apps/api's MenuItemFields).
export interface MenuItemFields {
  category_id: number;
  name: string;
  description: string | null;
  price: string | null;
  price_options: { label: string; price: string }[];
  image_url: string | null;
  is_featured: boolean;
}
