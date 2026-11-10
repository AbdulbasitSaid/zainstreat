import type { Metadata } from "next";
import { getLocale, getTranslations } from "next-intl/server";
import { Reveal } from "@/components/reveal";
import { SiteImage } from "@/components/site-image";
import { SplitRow } from "@/components/split-row";
import { Eyebrow } from "@/components/eyebrow";
import { ButtonAnchor, ButtonLink } from "@/components/button";
import { MenuBentoGrid } from "@/components/menu-bento-grid";
import { LiquidCard } from "@/components/liquid-card";
import { BoilingSlideshow, type BoilingSlide } from "@/components/boiling-slideshow";
import { Link } from "@/i18n/navigation";
import { getMenuItems, getSiteContent } from "@/lib/api";
import { getHomeText } from "@/lib/home-text";
import { siteImage } from "@/lib/site-content";
import type { AppLocale } from "@/i18n/routing";
import { buildWhatsAppLink } from "@/lib/whatsapp";
import hero from "@/assets/images/hero.jpg";
import categoryMeals from "@/assets/images/category-meals.jpg";
import categorySnacks from "@/assets/images/category-snacks.jpg";
import categoryCatering from "@/assets/images/category-catering.jpg";
import categoryEventRentals from "@/assets/images/category-event-rentals.jpg";
import aboutHero from "@/assets/images/about-hero.jpg";
import { localeAlternates } from "@/lib/seo";

const MAX_FEATURED_ITEMS = 6;

function ShieldCheckIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <path d="M12 3 5 5.8v5.4c0 4.6 3 8.8 7 10.3 4-1.5 7-5.7 7-10.3V5.8L12 3Z" />
      <path d="m9 12 2 2 4-4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SparkleIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 2.5c.6 3.6 2 6.1 6 7-4 .9-5.4 3.4-6 7-.6-3.6-2-6.1-6-7 4-.9 5.4-3.4 6-7Z" />
    </svg>
  );
}

function DropletIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <path d="M12 3s6 6.8 6 11.2A6 6 0 0 1 6 14.2C6 9.8 12 3 12 3Z" />
    </svg>
  );
}

function ClockIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3.2 2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

const PROMISE_ITEMS = [
  { Icon: ShieldCheckIcon, headingKey: "trustHalal", descKey: "trustHalalDesc" },
  { Icon: SparkleIcon, headingKey: "trustFresh", descKey: "trustFreshDesc" },
  { Icon: DropletIcon, headingKey: "trustHygienic", descKey: "trustHygienicDesc" },
  { Icon: ClockIcon, headingKey: "trustReliable", descKey: "trustReliableDesc" },
] as const;

// The first two fill the collage's left column, the last two its right.
const CATEGORY_ITEMS = [
  { labelKey: "servicesMeals", slot: "category_meals", src: categoryMeals, tileClass: "aspect-square w-3/4 self-end" },
  { labelKey: "servicesSnacks", slot: "category_snacks", src: categorySnacks, tileClass: "aspect-[4/3]" },
  { labelKey: "servicesCatering", slot: "category_catering", src: categoryCatering, tileClass: "aspect-[4/3]" },
  {
    labelKey: "servicesEventRentals",
    slot: "category_event_rentals",
    src: categoryEventRentals,
    tileClass: "aspect-[4/5] w-3/4",
  },
] as const;

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "HomePage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
    alternates: localeAlternates(locale, ""),
  };
}

export default async function Home() {
  // Admin overrides of the homepage text and images (Phase 25 Decisions 12–13).
  const [locale, content] = await Promise.all([getLocale(), getSiteContent()]);
  const t = await getHomeText(locale as AppLocale, content);
  const tw = await getTranslations("Whatsapp");
  const heroHeadingClass = "mb-4 text-[clamp(2.5rem,5vw,4.25rem)]";
  const heroSupportingClass = "max-w-[48ch] text-[clamp(1.05rem,1.5vw,1.25rem)] text-text-muted";

  const menuItems = await getMenuItems();
  const featuredItems = menuItems
    .filter((item) => item.is_featured && item.is_available)
    .slice(0, MAX_FEATURED_ITEMS);

  // The hero simmers through the featured dishes' photos, each photo once
  // (splashing a dish into itself would show nothing); with none, it falls
  // back to the (admin-replaceable) hero image.
  const featuredSlides: BoilingSlide[] = featuredItems.flatMap((item, i) =>
    item.image_url && featuredItems.findIndex((other) => other.image_url === item.image_url) === i
      ? [{ src: item.image_url, alt: item.name }]
      : [],
  );
  const heroSlides: BoilingSlide[] =
    featuredSlides.length > 0
      ? featuredSlides
      : [{ src: siteImage(content, "hero") ?? hero, alt: t.markup("heroHeadline", { em: (chunks) => chunks }) }];

  return (
    <main>
      <SplitRow
        className="container"
        content={
          <div>
            <Eyebrow>{t("heroEyebrow")}</Eyebrow>
            <h1 className={heroHeadingClass}>
              {t.rich("heroHeadline", { em: (chunks) => <em>{chunks}</em> })}
            </h1>
            <p className={heroSupportingClass}>{t("heroSupporting")}</p>
            <div className="mt-8 flex flex-wrap gap-3">
              <ButtonLink href="/menu">{t("orderNow")}</ButtonLink>
              <ButtonLink href="/contact" variant="secondary">{t("bookCatering")}</ButtonLink>
              <ButtonAnchor
                href={buildWhatsAppLink(tw("prefilledMessage"))}
                target="_blank"
                rel="noopener noreferrer"
                variant="outline"
                color="whatsapp"
              >
                {t("whatsappUs")}
              </ButtonAnchor>
            </div>
          </div>
        }
        media={<BoilingSlideshow slides={heroSlides} className="h-[clamp(340px,62vh,640px)]" />}
      />

      <SplitRow
        className="container"
        reverse
        media={
          // Staggered collage: the second column starts lower, and each
          // column pairs a small tile with a large one.
          <div className="grid grid-cols-2 gap-3 sm:gap-4">
            {[CATEGORY_ITEMS.slice(0, 2), CATEGORY_ITEMS.slice(2)].map((column, columnIndex) => (
              <div key={columnIndex} className={`flex flex-col gap-3 sm:gap-4 ${columnIndex === 1 ? "pt-10 sm:pt-14" : ""}`}>
                {column.map(({ labelKey, slot, src, tileClass }) => (
                  <Link key={labelKey} href="/services" className={`group relative block ${tileClass}`}>
                    <LiquidCard
                      wrapperClassName="h-full transition-transform duration-300 ease-out-expo group-hover:-translate-y-1"
                      className="h-full rounded-card shadow-sm"
                    >
                      <SiteImage src={siteImage(content, slot) ?? src} alt={t(labelKey)} className="h-full min-h-0" />
                      <span className="absolute bottom-2.5 left-2.5 rounded-full bg-background/90 px-3 py-1 text-[0.8rem] font-semibold text-primary shadow-sm">
                        {t(labelKey)}
                      </span>
                    </LiquidCard>
                  </Link>
                ))}
              </div>
            ))}
          </div>
        }
        content={
          <div>
            <Eyebrow>{t("servicesEyebrow")}</Eyebrow>
            <h2>{t.rich("servicesHeading", { em: (chunks) => <em>{chunks}</em> })}</h2>
            <p className={heroSupportingClass}>{t("servicesSupporting")}</p>
            <div className="mt-8">
              <ButtonLink href="/services">{t("exploreServices")}</ButtonLink>
            </div>
          </div>
        }
      />

      {featuredItems.length > 0 && (
        <section className="container">
          <Reveal>
            <div className="mb-10 text-center">
              <Eyebrow>{t("featuredMenuEyebrow")}</Eyebrow>
              <h2>{t("featuredMenuHeading")}</h2>
            </div>
            <MenuBentoGrid items={featuredItems} />
            <div className="mt-10 text-center">
              <ButtonLink href="/menu" variant="outline">
                {t("viewFullMenu")}
              </ButtonLink>
            </div>
          </Reveal>
        </section>
      )}

      <section className="container">
        <Reveal>
          <div className="mb-10 text-center">
            <Eyebrow>{t("whyChooseUsEyebrow")}</Eyebrow>
            <h2>{t.rich("whyChooseUs", { em: (chunks) => <em>{chunks}</em> })}</h2>
          </div>
          <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-8">
            {PROMISE_ITEMS.map(({ Icon, headingKey, descKey }) => (
              <LiquidCard
                key={headingKey}
                wrapperClassName="transition-transform duration-300 ease-out-expo hover:-translate-y-1"
                className="flex h-full flex-col gap-3 rounded-card border border-text/10 bg-background p-5 shadow-sm transition-shadow duration-300 ease-out-expo hover:shadow-[0_10px_24px_rgba(43,17,20,0.08)]"
                strokeClassName="stroke-text/10"
              >
                <span className="flex h-12 w-12 items-center justify-center rounded-full border border-accent-light text-primary [&_svg]:h-[1.4rem] [&_svg]:w-[1.4rem]">
                  <Icon />
                </span>
                <h3 className="m-0 text-[1.05rem]">{t(headingKey)}</h3>
                <p className="m-0 text-[0.95rem] text-text-muted">{t(descKey)}</p>
              </LiquidCard>
            ))}
          </div>
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <div className="rounded-card bg-gradient-to-br from-primary to-primary-dark p-[clamp(2.5rem,5vw,4rem)] text-center text-text-light">
            <h2 className="text-text-light">{t("cateringCtaHeadline")}</h2>
            <ButtonLink href="/contact" variant="invert">
              {t("requestCateringQuote")}
            </ButtonLink>
          </div>
        </Reveal>
      </section>

      <SplitRow
        className="container"
        media={<SiteImage src={siteImage(content, "about_hero") ?? aboutHero} alt={t("aboutPreview")} />}
        content={
          <div>
            <p className={heroSupportingClass}>{t("aboutPreview")}</p>
            <ButtonLink href="/about" variant="outline">
              {t("learnMore")}
            </ButtonLink>
          </div>
        }
      />
    </main>
  );
}
