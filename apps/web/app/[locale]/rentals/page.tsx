import type { Metadata } from "next";
import { getLocale, getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { ButtonAnchor } from "@/components/button";
import { RentalBentoGrid } from "@/components/menu-bento-grid";
import { Notice } from "@/components/notice";
import { Reveal } from "@/components/reveal";
import { getRentals } from "@/lib/api";
import { localeAlternates } from "@/lib/seo";
import type { AppLocale } from "@/i18n/routing";
import { buildWhatsAppLink } from "@/lib/whatsapp";

// The owner's edits in admin are live at once (Phase 36 Decision 9).
export const dynamic = "force-dynamic";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "RentalsPage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
    alternates: localeAlternates(locale, "/rentals"),
  };
}

// Phase 36 Decision 10, ordered through the cart since Phase 37 Decision 2.
export default async function RentalsPage() {
  const locale = (await getLocale()) as AppLocale;
  const [t, rentals] = await Promise.all([getTranslations("RentalsPage"), getRentals(locale)]);

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
        <p className="max-w-[48ch] text-[clamp(1.05rem,1.5vw,1.25rem)] text-text-muted">{t("intro")}</p>
      </PageHero>

      <Reveal>
        {rentals.length === 0 ? (
          <Notice>{t("empty")}</Notice>
        ) : (
          <RentalBentoGrid rentals={rentals} />
        )}
      </Reveal>

      <section className="my-section flex flex-col items-start gap-4 rounded-card bg-background-soft p-6 sm:p-8">
        <h2 className="m-0">{t("bookingHeading")}</h2>
        <p className="m-0 max-w-[56ch] text-text-muted">{t("bookingNote")}</p>
        <ButtonAnchor
          href={buildWhatsAppLink(t("whatsappGeneral"))}
          target="_blank"
          rel="noopener noreferrer"
          color="whatsapp"
        >
          {t("whatsappButton")}
        </ButtonAnchor>
      </section>
    </main>
  );
}
