"use client";

import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { formatPrice } from "@/lib/format";
import type { AdminRental, RentalFields } from "@/lib/admin-rentals";

const LINK_CLASSES =
  "text-sm font-semibold text-primary underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2";

function fieldsOf(rental: AdminRental): RentalFields {
  return {
    name: rental.name,
    description: rental.description,
    price: rental.price,
    image_url: rental.image_url,
    is_available: rental.is_available,
    display_order: rental.display_order,
  };
}

/** Phase 36 Decision 9: rentals save live (no draft → publish). */
export function AdminRentalList({ rentals: initialRentals }: { rentals: AdminRental[] }) {
  const [rentals, setRentals] = useState(initialRentals);
  const [confirmingArchiveId, setConfirmingArchiveId] = useState<number | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function send(rental: AdminRental, path: string, method: "PUT" | "POST", body: object) {
    setBusyId(rental.id);
    setNotice(null);
    const response = await fetch(path, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...body, updated_at: rental.updated_at }),
    });
    setBusyId(null);
    setConfirmingArchiveId(null);

    if (response.ok) return (await response.json()) as AdminRental;
    setNotice(
      response.status === 409
        ? "Changed elsewhere — refresh the page and try again."
        : "Could not save this rental. Please try again.",
    );
    return null;
  }

  async function handleToggle(rental: AdminRental) {
    const updated = await send(rental, `/api/admin/rentals/${rental.id}`, "PUT", {
      ...fieldsOf(rental),
      is_available: !rental.is_available,
    });
    if (updated) setRentals((current) => current.map((r) => (r.id === updated.id ? updated : r)));
  }

  async function handleArchive(rental: AdminRental) {
    const archived = await send(rental, `/api/admin/rentals/${rental.id}/archive`, "POST", {});
    if (archived) setRentals((current) => current.filter((r) => r.id !== archived.id));
  }

  if (rentals.length === 0) {
    return <p className="m-0 text-text-muted">No rentals yet.</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      {notice && <Notice>{notice}</Notice>}
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
              <th scope="col" className="py-3 pr-4 font-semibold">Order</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Price per event</th>
              <th scope="col" className="py-3 pr-4 font-semibold">Shown</th>
              <th scope="col" className="py-3 font-semibold">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rentals.map((rental) => (
              <tr key={rental.id} className="border-b border-text/10">
                <td className="py-3 pr-4 text-text-muted">{rental.display_order}</td>
                <td className="py-3 pr-4 font-semibold">{rental.name}</td>
                <td className="py-3 pr-4">{formatPrice(rental.price)}</td>
                <td className="py-3 pr-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={rental.is_available}
                      disabled={busyId === rental.id}
                      onChange={() => handleToggle(rental)}
                    />
                    <span>{rental.is_available ? "Available" : "Hidden"}</span>
                  </label>
                </td>
                <td className="py-3">
                  <div className="flex items-center gap-2">
                    <Link href={`/rentals/${rental.id}`} className={LINK_CLASSES}>
                      Edit
                    </Link>
                    {confirmingArchiveId === rental.id ? (
                      <>
                        <Button
                          type="button"
                          variant="secondary"
                          onClick={() => handleArchive(rental)}
                          disabled={busyId === rental.id}
                        >
                          Confirm
                        </Button>
                        <Button
                          type="button"
                          variant="secondary"
                          onClick={() => setConfirmingArchiveId(null)}
                          disabled={busyId === rental.id}
                        >
                          Cancel
                        </Button>
                      </>
                    ) : (
                      <Button type="button" variant="secondary" onClick={() => setConfirmingArchiveId(rental.id)}>
                        Archive
                      </Button>
                    )}
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
