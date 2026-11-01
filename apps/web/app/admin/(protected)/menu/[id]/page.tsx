import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminMenuItem } from "@/lib/admin-menu-items-api";
import { getAdminCategories } from "@/lib/admin-categories-api";
import { AdminMenuItemForm } from "@/components/admin-menu-item-form";
import { AdminPreviewPanel } from "@/components/admin-preview-panel";
import { AdminArchivedBadge, AdminDraftBadge } from "@/components/admin-archived-badge";

export const metadata: Metadata = {
  title: "Edit Menu Item | Zain's Admin",
};

export default async function EditAdminMenuItemPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const itemId = Number.parseInt(id, 10);
  if (!Number.isInteger(itemId) || itemId < 1) {
    notFound();
  }

  const [item, categories] = await Promise.all([getAdminMenuItem(itemId), getAdminCategories()]);
  // Archived categories are rejected server-side (requirement.md Decision 7
  // addendum) — hidden here too, except the item's own current category so
  // an archived item editing an already-archived category still shows it.
  const selectableCategories = categories.filter(
    (category) => !category.is_archived || category.id === item.category_id,
  );

  const shown = item.draft ?? item;
  // The preview opens where the change shows: the homepage for a featured
  // item, otherwise its menu category (Phase 25 Decision 24).
  const previewPath = shown.is_featured ? "" : `/menu?category=${shown.category_id}`;

  return (
    <AdminPreviewPanel previewPath={previewPath}>
      <div className="flex flex-col gap-8">
        <Link href="/menu" className="text-sm font-semibold text-primary underline">
          ‹ Back to menu
        </Link>
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="m-0">Edit {item.name}</h1>
          {item.is_archived ? (
            <AdminArchivedBadge />
          ) : !item.is_published ? (
            <AdminDraftBadge unpublished />
          ) : (
            item.has_draft && <AdminDraftBadge />
          )}
        </div>
        {/* Remounts from fresh values after a publish or discard. */}
        <AdminMenuItemForm
          key={`${item.updated_at}|${item.draft_updated_at ?? ""}`}
          categories={selectableCategories}
          initialValue={item}
        />
      </div>
    </AdminPreviewPanel>
  );
}
