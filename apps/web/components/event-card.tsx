import Image from "next/image";
import { useLocale, useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { ButtonLink } from "@/components/button";
import { ImageSlot } from "@/components/image-slot";
import { LiquidCard } from "@/components/liquid-card";
import { eventDateParts, type EventCardData } from "@/lib/events";

// The calendar-page badge on a card's photo: "27" over "DEC".
function DateBadge({ iso }: { iso: string }) {
  const locale = useLocale();
  const { day, month } = eventDateParts(iso, locale);
  return (
    <span className="absolute top-4 left-4 flex size-16 flex-col items-center justify-center rounded-full bg-cream text-primary shadow-md">
      <span className="text-2xl leading-none font-semibold">{day}</span>
      <span className="text-[0.7rem] font-semibold tracking-[0.14em] uppercase">{month}</span>
    </span>
  );
}

function Cover({ event, sizes }: { event: EventCardData; sizes: string }) {
  return event.cover_image_url ? (
    <Image
      src={event.cover_image_url}
      alt=""
      fill
      sizes={sizes}
      // Same dev-only bypass as menu-item-card.tsx: the web container can't
      // reach the API at the browser-facing localhost:8080.
      unoptimized={event.cover_image_url.startsWith("http://")}
      className="object-cover transition-transform duration-700 ease-out-expo group-hover/event:scale-105"
    />
  ) : (
    <ImageSlot label={event.title} className="h-full min-h-0 rounded-none border-0" />
  );
}

// An upcoming event, as a jelly card (Phase 31 Decision 16).
export function EventCard({ event }: { event: EventCardData }) {
  const t = useTranslations("EventsPage");
  const locale = useLocale();
  const { weekday, time } = eventDateParts(event.starts_at, locale);
  const full = event.seats_left === 0;

  return (
    <LiquidCard
      wrapperClassName="h-full"
      className="group/event flex h-full flex-col overflow-hidden rounded-card bg-cream shadow-sm transition-shadow duration-300 ease-out-expo hover:shadow-[0_18px_36px_rgba(43,17,20,0.16)]"
    >
      <div className="relative aspect-[16/10] w-full shrink-0">
        <Cover event={event} sizes="(max-width: 768px) 100vw, 640px" />
        <DateBadge iso={event.starts_at} />
      </div>
      <div className="flex flex-1 flex-col gap-3 p-6">
        <p className="m-0 text-sm font-semibold tracking-wide text-primary capitalize">
          {weekday} · {time} · {event.area}
        </p>
        <h3 className="m-0 text-[clamp(1.5rem,3vw,2rem)]">
          <Link href={`/events/${event.slug}`} className="text-text no-underline hover:text-primary">
            {event.title}
          </Link>
        </h3>
        {event.summary && <p className="m-0 text-text-muted">{event.summary}</p>}
        <div className="mt-auto flex flex-wrap items-center justify-between gap-3 pt-2">
          <span
            className={`rounded-full px-3 py-1 text-xs font-semibold tracking-wider uppercase ${
              full ? "bg-text text-white" : "bg-background-soft text-primary"
            }`}
          >
            {!event.rsvp_open
              ? t("bookingClosed")
              : full
                ? t("waitlistOpen")
                : t("seatsLeft", { count: event.seats_left })}
          </span>
          {event.rsvp_open && (
            <ButtonLink href={`/events/${event.slug}#book`}>{full ? t("joinWaitlist") : t("bookSeat")}</ButtonLink>
          )}
        </div>
      </div>
    </LiquidCard>
  );
}

// A past event: quieter, photo first, links to its recap and gallery.
export function PastEventCard({ event }: { event: EventCardData }) {
  const t = useTranslations("EventsPage");
  const locale = useLocale();
  const { long } = eventDateParts(event.starts_at, locale);

  return (
    <Link
      href={`/events/${event.slug}`}
      className="group/event flex flex-col gap-3 rounded-card text-text no-underline focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-offset-4 focus-visible:outline-accent"
    >
      <div className="relative aspect-[4/3] overflow-hidden rounded-card saturate-[0.8] transition-[filter] duration-500 group-hover/event:saturate-100">
        <Cover event={event} sizes="(max-width: 768px) 100vw, 400px" />
      </div>
      <div className="flex flex-col gap-1">
        <span className="text-sm text-text-muted">{long}</span>
        <span className="text-xl font-semibold group-hover/event:text-primary">{event.title}</span>
        {event.has_gallery && <span className="text-sm font-semibold text-primary">{t("seePhotos")} →</span>}
      </div>
    </Link>
  );
}
