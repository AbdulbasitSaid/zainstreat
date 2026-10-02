import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export default function ServicesPage() {
  const t = useTranslations("ServicesPage");

  return (
    <main className="container">
      <h1>{t("heading")}</h1>

      <section>
        <h2>{t("mealsHeading")}</h2>
        <p>{t("mealsCopy")}</p>
      </section>

      <section>
        <h2>{t("snacksHeading")}</h2>
        <p>{t("snacksCopy")}</p>
      </section>

      <section>
        <h2>{t("cateringHeading")}</h2>
        <p>{t("cateringCopy")}</p>
        <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
      </section>

      <section>
        <h2>{t("eventRentalsHeading")}</h2>
        <p>{t("eventRentalsCopy")}</p>
        <Link href="/contact" role="button">{t("requestQuote")}</Link>
      </section>
    </main>
  );
}
