import Link from "next/link";

const TAG_CLASSES = "inline-block rounded-full bg-background-soft px-3 py-1 text-xs font-semibold text-primary";

// Marks an order that pays for an event booking's meals (Phase 31
// Decision 20); with `eventId`, it links to the event's bookings.
export function AdminEventTag({ eventId, title }: { eventId?: number; title?: string | null }) {
  if (eventId === undefined) {
    return <span className={TAG_CLASSES}>Event</span>;
  }
  return (
    <Link href={`/events/${eventId}?tab=bookings`} className={`${TAG_CLASSES} underline`}>
      Event: {title ?? `#${eventId}`}
    </Link>
  );
}
