import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { Reveal } from "@/components/reveal";
import { ImageSlot } from "@/components/image-slot";

export default function ServicesPage() {
  const t = useTranslations("ServicesPage");

  return (
    <main className="container">
      <section className="page-hero">
        <Reveal>
          <h1 className="hero-heading">
            {t.rich("heading", { em: (chunks) => <em>{chunks}</em> })}
          </h1>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <div className="split-row">
            <div>
              <span className="split-row-index" aria-hidden="true">01</span>
              <h2>{t("mealsHeading")}</h2>
              <p className="hero-supporting">{t("mealsCopy")}</p>
            </div>
            <ImageSlot label={t("mealsHeading")} />
          </div>
        </Reveal>
      </section>

      <section className="section-band">
        <Reveal>
          <div className="split-row split-row--reverse">
            <ImageSlot label={t("snacksHeading")} />
            <div>
              <span className="split-row-index" aria-hidden="true">02</span>
              <h2>{t("snacksHeading")}</h2>
              <p className="hero-supporting">{t("snacksCopy")}</p>
            </div>
          </div>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <div className="split-row">
            <div>
              <span className="split-row-index" aria-hidden="true">03</span>
              <h2>{t("cateringHeading")}</h2>
              <p className="hero-supporting">{t("cateringCopy")}</p>
              <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
            </div>
            <ImageSlot label={t("cateringHeading")} />
          </div>
        </Reveal>
      </section>

      <section className="section-band">
        <Reveal>
          <div className="split-row split-row--reverse">
            <ImageSlot label={t("eventRentalsHeading")} />
            <div>
              <span className="split-row-index" aria-hidden="true">04</span>
              <h2>{t("eventRentalsHeading")}</h2>
              <p className="hero-supporting">{t("eventRentalsCopy")}</p>
              <Link href="/contact" role="button">{t("requestQuote")}</Link>
            </div>
          </div>
        </Reveal>
      </section>
    </main>
  );
}
