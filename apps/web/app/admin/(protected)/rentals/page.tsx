import Link from "next/link";
import type { Metadata } from "next";
import { getAdminRentals } from "@/lib/admin-rentals-api";
import { AdminRentalList } from "@/components/admin-rental-list";
import { buttonClasses } from "@/components/button";

export const metadata: Metadata = {
  title: "Rentals | Zain's Admin",
};

export default async function AdminRentalsPage() {
  const rentals = await getAdminRentals();

  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="m-0">Rentals</h1>
          <p className="m-0 mt-1 text-text-muted">
            Shown on the Rentals page; customers reserve them on WhatsApp. Changes are live as soon as you save.
          </p>
        </div>
        <Link href="/rentals/new" className={buttonClasses({})}>
          New rental
        </Link>
      </div>

      <AdminRentalList rentals={rentals} />
    </div>
  );
}
