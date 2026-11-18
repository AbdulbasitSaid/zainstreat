import Link from "next/link";
import type { Metadata } from "next";
import { AdminRentalForm } from "@/components/admin-rental-form";

export const metadata: Metadata = {
  title: "New Rental | Zain's Admin",
};

export default function NewAdminRentalPage() {
  return (
    <div className="flex flex-col gap-8">
      <Link href="/rentals" className="text-sm font-semibold text-primary underline">
        ‹ Back to rentals
      </Link>
      <h1 className="m-0">New Rental</h1>
      <AdminRentalForm />
    </div>
  );
}
