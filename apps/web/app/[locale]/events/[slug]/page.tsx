import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";
import { Eyebrow } from "@/components/eyebrow";
import { EventBookingForm } from "@/components/event-booking-form";
import { EventGallery } from "@/components/event-gallery";
import { EventSeatsPot } from "@/components/event-seats-pot";
import { ImageSlot } from "@/components/image-slot";
import { LiquidCard } from "@/components/liquid-card";
import { Notice } from "@/components/notice";
import { Reveal } from "@/components/reveal";
import { SiteImage } from "@/components/site-image";
import { eventDateParts } from "@/lib/events";
import { getEvent } from "@/lib/events-api";
import { formatPrice } from "@/lib/format";

export const dynamic = "force-dynamic";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string; slug: string }>;
}): Promise<Metadata> {
  const { locale, slug } = await params;
  const event = await getEvent(slug, locale);
  if (!event) return {};
  const t = await getTranslations({ locale, namespace: "EventPage" });
  return {
    title: t("metaTitle", { title: event.title }),
    description: event.summary || undefined,
    alternates: {
      canonical: `/${locale}/events/${slug}`,
      languages: {
        ...Object.fromEntries(routing.locales.map((l) => [l, `/${l}/events/${slug}`])),
        "x-default": `/${routing.defaultLocale}/events/${slug}`,
      },
    },
    openGraph: event.cover_image_url ? { images: [event.cover_image_url] } : undefined,
    robots: event.published ? undefined : { index: false, follow: false },
  };
}

function Chip({ children }: { children: React.ReactNode }) {
  return <span className="rounded-full bg-background-soft px-4 py-1.5 text-sm font-semibold text-primary">{children}</span>;
}

// One event (Phase 31 Decision 16): the story, the plates, the seats pot and
// the booking form; for a past event, the recap and gallery instead.
export default async function EventPage({ params }: { params: Promise<{ locale: string; slug: string }> }) {
  const { locale, slug } = await params;
  const event = await getEvent(slug, locale);
  if (!event) notFound();

  const t = await getTranslations("EventPage");
  const { long, time } = eventDateParts(event.starts_at, locale);
  const taken = event.capacity - event.seats_left;
  const mealCutoff = event.meal_orders_close_at ? eventDateParts(event.meal_orders_close_at, locale).long : null;
  const hasPlates = event.plates.length > 0;

  return (
    <main className="container">
      {!event.published && <Notice className="mt-6">{t("draftNotice")}</Notice>}

      <section className="grid items-center gap-10 lg:grid-cols-[1.1fr_1fr]">
        <Reveal>
          <Link href="/events" className="text-sm font-semibold text-primary no-underline hover:underline">
            ← {t("backToEvents")}
          </Link>
          <div className="mt-4">
            <Eyebrow>{event.is_past ? t("pastEyebrow") : t("freeEntry")}</Eyebrow>
          </div>
          <h1 className="text-[clamp(2.5rem,6vw,4.75rem)] leading-[1.02]">{event.title}</h1>
          {event.summary && <p className="text-lg text-text-muted">{event.summary}</p>}
          <div className="mt-5 flex flex-wrap gap-2">
            <Chip>
              <span className="capitalize">{long}</span>
            </Chip>
            <Chip>{time}</Chip>
            <Chip>{event.area}</Chip>
          </div>
        </Reveal>
        {event.cover_image_url ? (
          <SiteImage src={event.cover_image_url} alt={event.title} priority float className="aspect-[4/3] w-full shadow-xl" />
        ) : (
          <ImageSlot label={event.title} className="aspect-[4/3]" />
        )}
      </section>

      {event.story && (
        <section>
          <Reveal>
            <div className="mx-auto max-w-[65ch] text-lg leading-relaxed whitespace-pre-line">{event.story}</div>
          </Reveal>
        </section>
      )}

      {event.is_past ? (
        <>
          {event.recap && (
            <section>
              <Reveal>
                <h2>{t("recapHeading")}</h2>
                <div className="max-w-[65ch] text-lg leading-relaxed whitespace-pre-line">{event.recap}</div>
              </Reveal>
            </section>
          )}
          {event.photos.length > 0 && (
            <section>
              <Reveal>
                <h2>{t("galleryHeading")}</h2>
                <EventGallery photos={event.photos} title={event.title} />
              </Reveal>
            </section>
          )}
          {!event.recap && event.photos.length === 0 && <Notice>{t("pastNoRecap")}</Notice>}
        </>
      ) : (
        <>
          {hasPlates && (
            <section aria-labelledby="event-plates">
              <Reveal>
                <h2 id="event-plates">{t("platesHeading")}</h2>
                <ul className="m-0 grid list-none gap-5 p-0 sm:grid-cols-2">
                  {event.plates.map((plate, index) => (
                    <li key={plate.id}>
                      <LiquidCard
                        wrapperClassName="h-full"
                        className={`flex h-full flex-col gap-3 rounded-card p-6 shadow-sm ${
                          index === 0 ? "bg-primary text-text-light" : "bg-cream"
                        }`}
                      >
                        <span className="text-2xl font-semibold">{plate.name}</span>
                        {plate.description && (
                          <span className={index === 0 ? "text-on-dark-body" : "text-text-muted"}>{plate.description}</span>
                        )}
                        <span className={`mt-auto text-xl font-semibold ${index === 0 ? "text-accent-light" : "text-primary"}`}>
                          {formatPrice(plate.price)}
                        </span>
                      </LiquidCard>
                    </li>
                  ))}
                </ul>
              </Reveal>
            </section>
          )}

          <section id="book" aria-labelledby="event-book" className="scroll-mt-28">
            <Reveal>
              <div className="grid gap-10 lg:grid-cols-[280px_1fr]">
                <div className="flex flex-col items-center gap-4 lg:sticky lg:top-28 lg:self-start">
                  <EventSeatsPot
                    taken={taken}
                    capacity={event.capacity}
                    label={
                      event.seats_left > 0
                        ? t("seatsLeft", { count: event.seats_left, capacity: event.capacity })
                        : t("full")
                    }
                  />
                  {mealCutoff && event.meals_open && hasPlates && (
                    <p className="m-0 text-center text-sm text-text-muted">{t("mealCutoff", { date: mealCutoff })}</p>
                  )}
                </div>
                <div className="flex flex-col gap-5">
                  <h2 id="event-book" className="m-0">
                    {t("bookHeading")}
                  </h2>
                  <p className="m-0 text-lg text-text-muted">{t("guestWording")}</p>
                  {event.rsvp_open ? (
                    <EventBookingForm
                      slug={event.slug}
                      plates={event.plates}
                      maxPartySize={event.max_party_size}
                      seatsLeft={event.seats_left}
                      mealsOpen={event.meals_open}
                      terms={event.terms}
                    />
                  ) : (
                    <Notice>{t("rsvpClosed")}</Notice>
                  )}
                  <p className="m-0 text-sm text-text-muted">{t("addressPrivate")}</p>
                </div>
              </div>
            </Reveal>
          </section>
        </>
      )}
    </main>
  );
}
