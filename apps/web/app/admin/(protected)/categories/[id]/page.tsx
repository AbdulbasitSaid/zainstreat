import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminCategory } from "@/lib/admin-categories-api";
import { AdminCategoryForm } from "@/components/admin-category-form";
import { AdminPreviewPanel } from "@/components/admin-preview-panel";
import { AdminArchivedBadge, AdminDraftBadge } from "@/components/admin-archived-badge";

export const metadata: Metadata = {
  title: "Edit Category | Zain's Admin",
};

export default async function EditAdminCategoryPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const categoryId = Number.parseInt(id, 10);
  if (!Number.isInteger(categoryId) || categoryId < 1) {
    notFound();
  }

  const category = await getAdminCategory(categoryId).catch(() => null);
  if (!category) {
    notFound();
  }

  return (
    <AdminPreviewPanel previewPath={`/menu?category=${category.id}`}>
      <div className="flex flex-col gap-8">
        <Link href="/categories" className="text-sm font-semibold text-primary underline">
          ‹ Back to categories
        </Link>
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="m-0">Edit {category.name}</h1>
          {category.is_archived ? (
            <AdminArchivedBadge />
          ) : !category.is_published ? (
            <AdminDraftBadge unpublished />
          ) : (
            category.has_draft && <AdminDraftBadge />
          )}
        </div>
        <AdminCategoryForm key={`${category.updated_at}|${category.draft_updated_at ?? ""}`} initialValue={category} />
      </div>
    </AdminPreviewPanel>
  );
}
