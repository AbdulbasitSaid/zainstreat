"use client";

import { useState } from "react";
import Link from "next/link";
import Image from "next/image";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { ImageSlot } from "@/components/image-slot";
import { AdminArchivedBadge } from "@/components/admin-archived-badge";
import { formatPrice } from "@/lib/format";
import type { AdminMenuItem } from "@/lib/admin-menu-items";

interface ApiResponse {
  error: string;
}

export function AdminMenuList({ items: initialItems }: { items: AdminMenuItem[] }) {
  const [items, setItems] = useState(initialItems);
  const [confirmingArchiveId, setConfirmingArchiveId] = useState<number | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  function replaceItem(next: AdminMenuItem) {
    setItems((current) => current.map((item) => (item.id === next.id ? next : item)));
  }

  async function handleAvailabilityToggle(item: AdminMenuItem) {
    const next = { ...item, is_available: !item.is_available };
    replaceItem(next); // optimistic
    setBusyId(item.id);
    setNotice(null);

    const response = await fetch(`/api/admin/menu-items/${item.id}/availability`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ is_available: next.is_available, updated_at: item.updated_at }),
    });

    setBusyId(null);

    if (response.ok) {
      replaceItem((await response.json()) as AdminMenuItem);
      return;
    }

    replaceItem(item); // revert
    const body = (await response.json()) as ApiResponse;
    setNotice(
      body.error === "conflict"
        ? "Changed elsewhere — refresh the page and try again."
        : "Could not update availability. Please try again.",
    );
  }

  async function handleArchive(item: AdminMenuItem) {
    setBusyId(item.id);
    setNotice(null);

    const response = await fetch(`/api/admin/menu-items/${item.id}/archive`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: item.updated_at }),
    });

    setBusyId(null);
    setConfirmingArchiveId(null);

    if (response.ok) {
      replaceItem((await response.json()) as AdminMenuItem);
      return;
    }

    const body = (await response.json()) as ApiResponse;
    setNotice(
      body.error === "conflict"
        ? "Changed elsewhere — refresh the page and try again."
        : "Could not archive this item. Please try again.",
    );
  }

  if (items.length === 0) {
    return <p className="m-0 text-text-muted">No menu items yet.</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      {notice && <Notice>{notice}</Notice>}
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Photo</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Category</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Price</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Available</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Status</th>
              <th scope="col" className="py-3 font-semibold">Actions</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.id} className="border-b border-text/10">
                <td className="py-3 pr-4">
                  {item.image_url ? (
                    <Image
                      src={item.image_url}
                      alt={item.name}
                      width={48}
                      height={48}
                      sizes="48px"
                      // See components/menu-item-card.tsx — only dev's
                      // unreachable-host case stays unoptimized.
                      unoptimized={item.image_url.startsWith("http://")}
                      className="h-12 w-12 rounded-card object-cover"
                    />
                  ) : (
                    <ImageSlot label={item.name} className="h-12 min-h-0 w-12 p-0 text-[0.6rem]" />
                  )}
                </td>
                <td className="py-3 pr-4 font-semibold">{item.name}</td>
                <td className="py-3 pr-4">{item.category_name}</td>
                <td className="py-3 pr-4">
                  {item.price !== null
                    ? formatPrice(item.price)
                    : `${item.price_options.length} size${item.price_options.length === 1 ? "" : "s"}`}
                </td>
                <td className="py-3 pr-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={item.is_available}
                      disabled={item.is_archived || busyId === item.id}
                      onChange={() => handleAvailabilityToggle(item)}
                    />
                    <span className="sr-only">Available</span>
                  </label>
                </td>
                <td className="py-3 pr-4">{item.is_archived && <AdminArchivedBadge />}</td>
                <td className="py-3">
                  <div className="flex items-center gap-2">
                    <Link
                      href={`/admin/menu/${item.id}`}
                      className="text-sm font-semibold text-primary underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
                    >
                      Edit
                    </Link>
                    {!item.is_archived &&
                      (confirmingArchiveId === item.id ? (
                        <>
                          <Button
                            type="button"
                            variant="secondary"
                            onClick={() => handleArchive(item)}
                            disabled={busyId === item.id}
                          >
                            Confirm
                          </Button>
                          <Button
                            type="button"
                            variant="secondary"
                            onClick={() => setConfirmingArchiveId(null)}
                            disabled={busyId === item.id}
                          >
                            Cancel
                          </Button>
                        </>
                      ) : (
                        <Button
                          type="button"
                          variant="secondary"
                          onClick={() => setConfirmingArchiveId(item.id)}
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
