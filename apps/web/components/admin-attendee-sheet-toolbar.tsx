"use client";

import Link from "next/link";
import { Button } from "@/components/button";

/** Phase 38 Decision 15: on screen only; "Download PDF" is the browser's Save as PDF. */
export function AdminAttendeeSheetToolbar({ eventId }: { eventId: number }) {
  return (
    <div className="flex flex-wrap items-center gap-3 print:hidden">
      <Button type="button" onClick={() => window.print()}>
        Download PDF
      </Button>
      <a
        href={`/api/admin/events/${eventId}/attendees.csv`}
        className="text-sm font-semibold text-primary underline"
      >
        Download CSV
      </a>
      <Link href={`/events/${eventId}?tab=bookings`} className="text-sm font-semibold text-primary underline">
        Back to bookings
      </Link>
      <p className="m-0 w-full text-sm text-text-muted">
        In the print window, choose &ldquo;Save as PDF&rdquo; and turn on background graphics for the colours.
      </p>
    </div>
  );
}
