import Link from "next/link";
import type { Metadata } from "next";
import { buttonClasses } from "@/components/button";
import { getAdminEvents } from "@/lib/admin-events-api";
import { formatDateTime, formatPrice } from "@/lib/format";
import type { AdminEventSummary } from "@/lib/admin-events";

export const metadata: Metadata = {
  title: "Events | Zain's Admin",
};

function StatusBadge({ event }: { event: AdminEventSummary }) {
  const [label, classes] = event.archived
    ? ["Archived", "bg-text/10 text-text-muted"]
    : event.published
      ? ["Published", "bg-whatsapp/15 text-whatsapp-dark"]
      : ["Draft", "bg-accent-light text-primary-dark"];
  return <span className={`rounded-full px-3 py-1 text-xs font-semibold ${classes}`}>{label}</span>;
}

function EventRow({ event }: { event: AdminEventSummary }) {
  return (
    <li className={`rounded-card border border-text/10 ${event.archived ? "opacity-60" : ""}`}>
      <Link href={`/events/${event.id}`} className="flex flex-wrap items-center gap-x-6 gap-y-2 p-4 text-text no-underline hover:bg-background-soft">
        <span className="flex min-w-[14rem] flex-1 flex-col">
          <span className="text-lg font-semibold">{event.title}</span>
          <span className="text-sm text-text-muted">{formatDateTime(event.starts_at)}</span>
        </span>
        <StatusBadge event={event} />
        <span className="text-sm">
          <span className="font-semibold">{event.seats_taken}</span> / {event.capacity} seats
        </span>
        {event.waitlist_count > 0 && <span className="text-sm text-primary">{event.waitlist_count} waiting</span>}
        <span className="text-sm text-text-muted">{formatPrice(event.paid_total)} paid</span>
      </Link>
    </li>
  );
}

// Upcoming first (soonest first), then past (newest first) — Phase 31 Decision 18.
export default async function AdminEventsPage() {
  const events = await getAdminEvents();
  const upcoming = events.filter((event) => !event.is_past);
  const past = events.filter((event) => event.is_past);

  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="m-0">Events</h1>
        <Link href="/events/new" className={buttonClasses({})}>
          New event
        </Link>
      </div>

      <section className="flex flex-col gap-3">
        <h2 className="m-0 text-xl">Upcoming</h2>
        {upcoming.length === 0 ? (
          <p className="m-0 text-text-muted">No upcoming events.</p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-3 p-0">
            {upcoming.map((event) => <EventRow key={event.id} event={event} />)}
          </ul>
        )}
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="m-0 text-xl">Past</h2>
        <p className="m-0 text-sm text-text-muted">
          Published past events show on the public Events page. Add a recap and photos from each event&apos;s Past event tab.
        </p>
        {past.length === 0 ? (
          <p className="m-0 text-text-muted">No past events yet.</p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-3 p-0">
            {past.map((event) => <EventRow key={event.id} event={event} />)}
          </ul>
        )}
      </section>
    </div>
  );
}
