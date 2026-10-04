import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminCategories } from "@/lib/admin-categories-api";
import { AdminCategoryForm } from "@/components/admin-category-form";

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

  // No dedicated `GET /api/admin/categories/{id}` — the admin list is a
  // small, unpaginated table (same precedent as the public category list),
  // so filtering the one already-fetched array is simpler than a new route.
  const categories = await getAdminCategories();
  const category = categories.find((c) => c.id === categoryId);
  if (!category) {
    notFound();
  }

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/categories" className="text-sm font-semibold text-primary underline">
        ‹ Back to categories
      </Link>
      <h1 className="m-0">Edit {category.name}</h1>
      <AdminCategoryForm initialValue={category} />
    </div>
  );
}
