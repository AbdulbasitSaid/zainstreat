import Link from "next/link";
import type { Metadata } from "next";
import { getAdminCategories } from "@/lib/admin-categories-api";
import { AdminMenuItemForm } from "@/components/admin-menu-item-form";

export const metadata: Metadata = {
  title: "New Menu Item | Zain's Admin",
};

export default async function NewAdminMenuItemPage() {
  const categories = await getAdminCategories();
  // Archived categories are rejected server-side anyway (requirement.md
  // Decision 7 addendum) — don't even offer them here.
  const activeCategories = categories.filter((category) => !category.is_archived);

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/menu" className="text-sm font-semibold text-primary underline">
        ‹ Back to menu
      </Link>
      <h1 className="m-0">New Menu Item</h1>
      <AdminMenuItemForm categories={activeCategories} />
    </div>
  );
}
