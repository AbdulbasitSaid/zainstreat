import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminMenuItem } from "@/lib/admin-menu-items-api";
import { getAdminCategories } from "@/lib/admin-categories-api";
import { AdminMenuItemForm } from "@/components/admin-menu-item-form";

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

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/menu" className="text-sm font-semibold text-primary underline">
        ‹ Back to menu
      </Link>
      <h1 className="m-0">Edit {item.name}</h1>
      <AdminMenuItemForm categories={selectableCategories} initialValue={item} />
    </div>
  );
}
