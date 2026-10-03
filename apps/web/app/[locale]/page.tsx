import { getTranslations } from "next-intl/server";
import { Reveal } from "@/components/reveal";
import { DecorativeShape } from "@/components/decorative-shape";
import { ImageSlot } from "@/components/image-slot";
import { SplitRow } from "@/components/split-row";
import { Eyebrow } from "@/components/eyebrow";
import { ButtonLink, buttonClasses } from "@/components/button";
import { MenuItemCard } from "@/components/menu-item-card";
import { Link } from "@/i18n/navigation";
import { getMenuItems } from "@/lib/api";

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

const CATEGORY_ITEMS = [
  { labelKey: "servicesMeals" },
  { labelKey: "servicesSnacks" },
  { labelKey: "servicesCatering" },
  { labelKey: "servicesEventRentals" },
] as const;

export default async function Home() {
  const t = await getTranslations("HomePage");
  const heroHeadingClass = "mb-4 text-[clamp(2.5rem,5vw,4.25rem)]";
  const heroSupportingClass = "max-w-[48ch] text-[clamp(1.05rem,1.5vw,1.25rem)] text-text-muted";

  const menuItems = await getMenuItems();
  const featuredItems = menuItems
    .filter((item) => item.is_featured && item.is_available)
    .slice(0, MAX_FEATURED_ITEMS);

  return (
    <main>
      <SplitRow
        className="container relative overflow-hidden"
        decoration={
          <>
            <DecorativeShape style={{ width: "220px", height: "220px", top: "-60px", right: "-60px" }} />
            <DecorativeShape style={{ width: "120px", height: "120px", bottom: "-30px", left: "-30px" }} />
          </>
        }
        content={
          <div>
            <Eyebrow>{t("heroEyebrow")}</Eyebrow>
            <h1 className={heroHeadingClass}>
              {t.rich("heroHeadline", { em: (chunks) => <em>{chunks}</em> })}
            </h1>
            <p className={heroSupportingClass}>{t("heroSupporting")}</p>
            <div className="mt-8 flex flex-wrap gap-3">
              <ButtonLink href="/contact">{t("orderNow")}</ButtonLink>
              <ButtonLink href="/contact" variant="secondary">{t("bookCatering")}</ButtonLink>
              <a
                href="https://wa.me/31630545277"
                role="button"
                className={buttonClasses({ variant: "outline", color: "whatsapp" })}
              >
                {t("whatsappUs")}
              </a>
            </div>
          </div>
        }
        media={<ImageSlot label={t.markup("heroHeadline", { em: (chunks) => chunks })} />}
      />

      <section className="container">
        <Reveal>
          <div className="flex flex-wrap justify-center gap-x-10 gap-y-8">
            {CATEGORY_ITEMS.map(({ labelKey }) => (
              <div key={labelKey} className="flex w-[150px] flex-col items-center gap-3.5 text-center">
                <ImageSlot
                  label={t(labelKey)}
                  className="h-[150px] w-[150px] min-h-0 rounded-full p-2 text-[0.72rem]"
                />
                <h3 className="m-0 text-base">{t(labelKey)}</h3>
                <Link href="/services" className="text-[0.85rem] font-semibold text-primary">
                  {t("learnMore")}
                </Link>
              </div>
            ))}
          </div>
        </Reveal>
      </section>

      {featuredItems.length > 0 && (
        <section className="container">
          <Reveal>
            <div className="mb-10 text-center">
              <Eyebrow>{t("featuredMenuEyebrow")}</Eyebrow>
              <h2>{t("featuredMenuHeading")}</h2>
            </div>
            <div className="grid grid-cols-[repeat(auto-fit,minmax(260px,1fr))] gap-6">
              {featuredItems.map((item) => (
                <MenuItemCard key={item.id} item={item} />
              ))}
            </div>
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
              <div
                key={headingKey}
                className="flex flex-col gap-3 rounded-card border border-text/10 bg-background p-5 shadow-sm transition-[transform,box-shadow] duration-300 ease-out-expo hover:-translate-y-1 hover:shadow-[0_10px_24px_rgba(43,17,20,0.08)]"
              >
                <span className="flex h-12 w-12 items-center justify-center rounded-full border border-accent-light text-primary [&_svg]:h-[1.4rem] [&_svg]:w-[1.4rem]">
                  <Icon />
                </span>
                <h3 className="m-0 text-[1.05rem]">{t(headingKey)}</h3>
                <p className="m-0 text-[0.95rem] text-text-muted">{t(descKey)}</p>
              </div>
            ))}
          </div>
        </Reveal>
      </section>

      <section className="container relative overflow-hidden">
        <DecorativeShape style={{ width: "160px", height: "160px", top: "-40px", left: "50%" }} />
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
        media={<ImageSlot label={t("aboutPreview")} />}
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
