import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { Reveal } from "@/components/reveal";
import { ImageSlot } from "@/components/image-slot";

export default function ServicesPage() {
  const t = useTranslations("ServicesPage");

  return (
    <main className="container">
      <h1 className="hero-heading" style={{ textAlign: "center" }}>{t("heading")}</h1>

      <section>
        <Reveal>
          <div className="split-row">
            <div>
              <h2>{t("mealsHeading")}</h2>
              <p className="hero-supporting">{t("mealsCopy")}</p>
            </div>
            <ImageSlot label={t("mealsHeading")} />
          </div>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <div className="split-row split-row--reverse">
            <ImageSlot label={t("snacksHeading")} />
            <div>
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
              <h2>{t("cateringHeading")}</h2>
              <p className="hero-supporting">{t("cateringCopy")}</p>
              <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
            </div>
            <ImageSlot label={t("cateringHeading")} />
          </div>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <div className="split-row split-row--reverse">
            <ImageSlot label={t("eventRentalsHeading")} />
            <div>
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
