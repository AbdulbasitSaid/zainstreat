import Link from "next/link";
import type { Metadata } from "next";
import { getAdminCategories } from "@/lib/admin-categories-api";
import { AdminCategoryList } from "@/components/admin-category-list";
import { buttonClasses } from "@/components/button";

export const metadata: Metadata = {
  title: "Categories | Zain's Admin",
};

export default async function AdminCategoriesPage() {
  const categories = await getAdminCategories();

  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Categories</h1>
        {/* Styled like Button but a plain next/link — admin is English-only (requirement.md Decision 15). */}
        <Link href="/admin/categories/new" className={buttonClasses({})}>
          New category
        </Link>
      </div>

      <AdminCategoryList categories={categories} />
    </div>
  );
}
