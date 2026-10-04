"use client";

import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminArchivedBadge } from "@/components/admin-archived-badge";
import type { AdminCategory } from "@/lib/admin-categories";

interface ApiResponse {
  error: string;
  items?: { id: number; name: string }[];
}

export function AdminCategoryList({ categories: initialCategories }: { categories: AdminCategory[] }) {
  const [categories, setCategories] = useState(initialCategories);
  const [confirmingArchiveId, setConfirmingArchiveId] = useState<number | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function handleArchive(category: AdminCategory) {
    setBusyId(category.id);
    setNotice(null);

    const response = await fetch(`/api/admin/categories/${category.id}/archive`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: category.updated_at }),
    });

    setBusyId(null);
    setConfirmingArchiveId(null);

    if (response.ok) {
      const updated = (await response.json()) as AdminCategory;
      setCategories((current) => current.map((c) => (c.id === updated.id ? updated : c)));
      return;
    }

    const body = (await response.json()) as ApiResponse;
    if (body.error === "category_has_active_items" && body.items) {
      const names = body.items.map((item) => item.name).join(", ");
      setNotice(`Can't archive — ${names} still use${body.items.length === 1 ? "s" : ""} this category.`);
    } else if (body.error === "conflict") {
      setNotice("Changed elsewhere — refresh the page and try again.");
    } else {
      setNotice("Could not archive this category. Please try again.");
    }
  }

  if (categories.length === 0) {
    return <p className="m-0 text-text-muted">No categories yet.</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      {notice && <Notice>{notice}</Notice>}
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Description</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Status</th>
              <th scope="col" className="py-3 font-semibold">Actions</th>
            </tr>
          </thead>
          <tbody>
            {categories.map((category) => (
              <tr key={category.id} className="border-b border-text/10">
                <td className="py-3 pr-4 font-semibold">{category.name}</td>
                <td className="py-3 pr-4 text-text-muted">{category.description ?? "—"}</td>
                <td className="py-3 pr-4">{category.is_archived && <AdminArchivedBadge />}</td>
                <td className="py-3">
                  <div className="flex items-center gap-2">
                    <Link
                      href={`/admin/categories/${category.id}`}
                      className="text-sm font-semibold text-primary underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                    >
                      Rename
                    </Link>
                    {!category.is_archived &&
                      (confirmingArchiveId === category.id ? (
                        <>
                          <Button
                            type="button"
                            variant="secondary"
                            onClick={() => handleArchive(category)}
                            disabled={busyId === category.id}
                          >
                            Confirm
                          </Button>
                          <Button
                            type="button"
                            variant="secondary"
                            onClick={() => setConfirmingArchiveId(null)}
                            disabled={busyId === category.id}
                          >
                            Cancel
                          </Button>
                        </>
                      ) : (
                        <Button
                          type="button"
                          variant="secondary"
                          onClick={() => setConfirmingArchiveId(category.id)}
                        >
                          Archive
                        </Button>
                      ))}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
