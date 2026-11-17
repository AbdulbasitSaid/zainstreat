import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { AdminEventActions } from "@/components/admin-event-actions";
import { AdminEventBookings } from "@/components/admin-event-bookings";
import { AdminEventForm } from "@/components/admin-event-form";
import { AdminEventPast } from "@/components/admin-event-past";
import { AdminEventShare } from "@/components/admin-event-share";
import { Notice } from "@/components/notice";
import { getAdminEvent, getAdminEventBookings } from "@/lib/admin-events-api";
import { toInput } from "@/lib/admin-events";
import { formatDateTime } from "@/lib/format";
import { previewEnableUrl } from "@/lib/preview-paths";

export const metadata: Metadata = {
  title: "Event | Zain's Admin",
};

const TABS = [
  { key: "details", label: "Details" },
  { key: "bookings", label: "Bookings" },
  { key: "share", label: "Share" },
  { key: "past", label: "Past event" },
] as const;

type Tab = (typeof TABS)[number]["key"];

function isTab(value: string | undefined): value is Tab {
  return TABS.some((tab) => tab.key === value);
}

export default async function AdminEventPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ tab?: string }>;
}) {
  const { id } = await params;
  const { tab: rawTab } = await searchParams;
  const eventId = Number.parseInt(id, 10);
  if (!Number.isInteger(eventId) || eventId < 1) notFound();

  const event = await getAdminEvent(eventId);
  if (!event) notFound();
  const tab: Tab = isTab(rawTab) ? rawTab : "details";
  const bookings = tab === "bookings" ? await getAdminEventBookings(eventId) : null;

  return (
    <div className="flex flex-col gap-6">
      <Link href="/events" className="text-sm font-semibold text-primary underline">
        ‹ Back to events
      </Link>

      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="m-0">{event.content.en.title}</h1>
          <p className="m-0 text-text-muted">
            {formatDateTime(event.starts_at_utc)} · {event.seats_taken} / {event.capacity} seats ·{" "}
            {event.archived ? "Archived" : event.published ? "Published" : "Draft"}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          {/* Draft-mode preview on the cms. host (Phase 25 Decision 21). */}
          <a href={previewEnableUrl("en", `/events/${event.slug}`)} target="_blank" rel="noopener noreferrer" className="text-sm font-semibold text-primary underline">
            Preview
          </a>
          <AdminEventActions id={event.id} published={event.published} archived={event.archived} />
        </div>
      </div>

      {!event.published && !event.archived && (
        <Notice>
          This event is a draft. Check the plate prices, set the address and the text, then press Publish.
          {!event.address.trim() && " The address is still empty."}
        </Notice>
      )}

      <nav aria-label="Event sections" className="flex flex-wrap gap-1 border-b border-text/10">
        {TABS.map(({ key, label }) => (
          <Link
            key={key}
            href={`/events/${event.id}?tab=${key}`}
            aria-current={tab === key ? "page" : undefined}
            className={`-mb-px border-b-2 px-4 py-2 text-sm font-semibold no-underline ${
              tab === key ? "border-primary text-primary" : "border-transparent text-text hover:text-primary"
            }`}
          >
            {label}
          </Link>
        ))}
      </nav>

      {tab === "details" && <AdminEventForm key={event.id} event={event} initial={toInput(event)} />}
      {tab === "bookings" && bookings && <AdminEventBookings eventId={event.id} initial={bookings} />}
      {tab === "share" && <AdminEventShare slug={event.slug} title={event.content.en.title} published={event.published && !event.archived} />}
      {tab === "past" && <AdminEventPast event={event} />}
    </div>
  );
}
