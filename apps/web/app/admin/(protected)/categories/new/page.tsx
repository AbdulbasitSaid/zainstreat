import Link from "next/link";
import type { Metadata } from "next";
import { AdminCategoryForm } from "@/components/admin-category-form";

export const metadata: Metadata = {
  title: "New Category | Zain's Admin",
};

export default function NewAdminCategoryPage() {
  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/categories" className="text-sm font-semibold text-primary underline">
        ‹ Back to categories
      </Link>
      <h1 className="m-0">New Category</h1>
      <AdminCategoryForm />
    </div>
  );
}
