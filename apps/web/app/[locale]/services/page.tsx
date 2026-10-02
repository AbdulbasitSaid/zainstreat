import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { Reveal } from "@/components/reveal";
import { ImageSlot } from "@/components/image-slot";

export default function ServicesPage() {
  const t = useTranslations("ServicesPage");

  return (
    <main className="container">
      <h1>{t("heading")}</h1>

      <section>
        <Reveal>
          <h2>{t("mealsHeading")}</h2>
          <p>{t("mealsCopy")}</p>
          <ImageSlot label={t("mealsHeading")} />
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("snacksHeading")}</h2>
          <p>{t("snacksCopy")}</p>
          <ImageSlot label={t("snacksHeading")} />
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("cateringHeading")}</h2>
          <p>{t("cateringCopy")}</p>
          <ImageSlot label={t("cateringHeading")} />
          <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("eventRentalsHeading")}</h2>
          <p>{t("eventRentalsCopy")}</p>
          <ImageSlot label={t("eventRentalsHeading")} />
          <Link href="/contact" role="button">{t("requestQuote")}</Link>
        </Reveal>
      </section>
    </main>
  );
}
