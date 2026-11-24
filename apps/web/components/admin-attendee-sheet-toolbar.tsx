import Link from "next/link";
import { ButtonAnchor } from "@/components/button";

/**
 * Phase 38 Decisions 15, 27: on screen only. "Download PDF" is the real
 * file (attendees.pdf); printing this page still works for a quick copy.
 */
export function AdminAttendeeSheetToolbar({ eventId }: { eventId: number }) {
  return (
    <div className="flex flex-wrap items-center gap-3 print:hidden">
      <ButtonAnchor href={`/api/admin/events/${eventId}/attendees.pdf`} download>
        Download PDF
      </ButtonAnchor>
      <a
        href={`/api/admin/events/${eventId}/attendees.csv`}
        className="text-sm font-semibold text-primary underline"
      >
        Download CSV
      </a>
      <Link href={`/events/${eventId}?tab=bookings`} className="text-sm font-semibold text-primary underline">
        Back to bookings
      </Link>
    </div>
  );
}
