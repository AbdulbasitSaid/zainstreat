import Link from "next/link";
import type { Metadata } from "next";
import { AdminEventForm } from "@/components/admin-event-form";

export const metadata: Metadata = {
  title: "New Event | Zain's Admin",
};

export default function NewAdminEventPage() {
  return (
    <div className="flex flex-col gap-6">
      <Link href="/events" className="text-sm font-semibold text-primary underline">
        ‹ Back to events
      </Link>
      <h1 className="m-0">New event</h1>
      <p className="m-0 text-text-muted">It&apos;s saved as a draft. Nobody can see it until you publish it.</p>
      <AdminEventForm event={null} initial={null} />
    </div>
  );
}
