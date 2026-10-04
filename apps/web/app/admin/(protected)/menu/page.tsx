import Link from "next/link";
import type { Metadata } from "next";
import { getAdminMenuItems } from "@/lib/admin-menu-items-api";
import { AdminMenuList } from "@/components/admin-menu-list";
import { buttonClasses } from "@/components/button";

export const metadata: Metadata = {
  title: "Menu | Zain's Admin",
};

export default async function AdminMenuPage() {
  const items = await getAdminMenuItems();

  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Menu</h1>
        {/* Styled like Button but a plain next/link — admin is English-only (requirement.md Decision 15). */}
        <Link href="/admin/menu/new" className={buttonClasses({})}>
          New item
        </Link>
      </div>

      <AdminMenuList items={items} />
    </div>
  );
}
