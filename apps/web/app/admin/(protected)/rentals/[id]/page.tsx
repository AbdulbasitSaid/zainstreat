import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminRental } from "@/lib/admin-rentals-api";
import { AdminRentalForm } from "@/components/admin-rental-form";
import { AdminArchivedBadge } from "@/components/admin-archived-badge";

export const metadata: Metadata = {
  title: "Edit Rental | Zain's Admin",
};

export default async function EditAdminRentalPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const rentalId = Number.parseInt(id, 10);
  if (!Number.isInteger(rentalId) || rentalId < 1) {
    notFound();
  }

  const rental = await getAdminRental(rentalId).catch(() => null);
  if (!rental) {
    notFound();
  }

  return (
    <div className="flex flex-col gap-8">
      <Link href="/rentals" className="text-sm font-semibold text-primary underline">
        ‹ Back to rentals
      </Link>
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="m-0">Edit {rental.name}</h1>
        {rental.is_archived && <AdminArchivedBadge />}
      </div>
      <AdminRentalForm key={rental.updated_at} initialValue={rental} />
    </div>
  );
}
