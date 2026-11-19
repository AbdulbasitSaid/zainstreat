import type { Metadata } from "next";
import Image from "next/image";
import { getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { AddRentalToCart } from "@/components/add-to-cart-controls";
import { ButtonAnchor } from "@/components/button";
import { ImageSlot } from "@/components/image-slot";
import { Notice } from "@/components/notice";
import { Reveal } from "@/components/reveal";
import { getRentals } from "@/lib/api";
import { formatPrice } from "@/lib/format";
import { localeAlternates } from "@/lib/seo";
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
  const [t, rentals] = await Promise.all([getTranslations("RentalsPage"), getRentals()]);

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
          <ul className="m-0 grid list-none grid-cols-1 gap-5 p-0 sm:grid-cols-2 lg:grid-cols-3">
            {rentals.map((rental) => (
              <li key={rental.id} className="flex flex-col overflow-hidden rounded-card bg-cream shadow-sm">
                <div className="relative h-48 w-full shrink-0">
                  {rental.image_url ? (
                    <Image
                      src={rental.image_url}
                      alt={rental.name}
                      fill
                      sizes="(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 400px"
                      // Same dev-only exception as the menu cards.
                      unoptimized={rental.image_url.startsWith("http://")}
                      className="object-cover"
                    />
                  ) : (
                    <ImageSlot label={rental.name} className="h-full min-h-0 rounded-none border-0" />
                  )}
                </div>
                <div className="flex flex-1 flex-col gap-1.5 p-5">
                  <h2 className="m-0 text-[1.15rem]">{rental.name}</h2>
                  {rental.description && <p className="m-0 text-[0.95rem] text-text-muted">{rental.description}</p>}
                  <div className="mt-auto flex items-center justify-between gap-3 pt-2">
                    <p className="m-0 text-[1.05rem] font-semibold text-primary">
                      {t("pricePerEvent", { price: formatPrice(rental.price) })}
                    </p>
                    <AddRentalToCart rental={rental} />
                  </div>
                </div>
              </li>
            ))}
          </ul>
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
