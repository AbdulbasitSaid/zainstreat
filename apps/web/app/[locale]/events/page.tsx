import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { ButtonAnchor } from "@/components/button";
import { Eyebrow } from "@/components/eyebrow";
import { EventCard, PastEventCard } from "@/components/event-card";
import { Notice } from "@/components/notice";
import { PageHero } from "@/components/page-hero";
import { Reveal } from "@/components/reveal";
import { getEvents } from "@/lib/events-api";
import { localeAlternates } from "@/lib/seo";
import { buildWhatsAppLink } from "@/lib/whatsapp";

export const dynamic = "force-dynamic";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "EventsPage" });
  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
    alternates: localeAlternates(locale, "/events"),
  };
}

// Upcoming and past events (Phase 31 Decision 16).
export default async function EventsPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = await getTranslations("EventsPage");
  const { upcoming, past } = await getEvents(locale);

  return (
    <main className="container">
      <PageHero>
        <Eyebrow>{t("eyebrow")}</Eyebrow>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
        <p className="text-lg text-text-muted">{t("intro")}</p>
      </PageHero>

      <section aria-labelledby="events-upcoming">
        <Reveal>
          <h2 id="events-upcoming">{t("upcoming")}</h2>
          {upcoming.length > 0 ? (
            <ul className={`m-0 grid list-none gap-6 p-0 ${upcoming.length > 1 ? "md:grid-cols-2" : "mx-auto max-w-[760px]"}`}>
              {upcoming.map((event) => (
                <li key={event.slug}>
                  <EventCard event={event} />
                </li>
              ))}
            </ul>
          ) : (
            <div className="flex flex-col items-start gap-4">
              <Notice>{t("noUpcoming")}</Notice>
              <ButtonAnchor
                href={buildWhatsAppLink(t("whatsappMessage"))}
                target="_blank"
                rel="noopener noreferrer"
                variant="outline"
                color="whatsapp"
              >
                {t("noUpcomingCta")}
              </ButtonAnchor>
            </div>
          )}
        </Reveal>
      </section>

      {past.length > 0 && (
        <section aria-labelledby="events-past">
          <Reveal>
            <h2 id="events-past">{t("past")}</h2>
            <p className="text-text-muted">{t("pastIntro")}</p>
            <ul className="m-0 grid list-none gap-8 p-0 sm:grid-cols-2 lg:grid-cols-3">
              {past.map((event) => (
                <li key={event.slug}>
                  <PastEventCard event={event} />
                </li>
              ))}
            </ul>
          </Reveal>
        </section>
      )}
    </main>
  );
}
