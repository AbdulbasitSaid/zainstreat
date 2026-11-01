import type { DraftKind } from "@/lib/admin-drafts";

// GET /api/admin/changes (Phase 25 Decision 20).
export interface ChangedField {
  field: string;
  before: string | boolean | null;
  after: string | boolean | null;
}

export interface ChangeEntry {
  kind: DraftKind;
  id: number | null;
  label: string;
  changed_fields: ChangedField[];
  draft_updated_at: string | null;
  is_new: boolean;
  category_id: number | null;
}

export const KIND_LABELS: Record<DraftKind, string> = {
  site_content: "Site content",
  menu_item: "Menu item",
  category: "Category",
};

/** The public page an entry's preview opens on. */
export function changePreviewPath(entry: ChangeEntry): string {
  return entry.kind === "site_content" || entry.category_id === null ? "" : `/menu?category=${entry.category_id}`;
}

export function isImageField(field: string): boolean {
  return field === "image_url" || field.startsWith("images.");
}
