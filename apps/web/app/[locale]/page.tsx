import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export default function Home() {
  const t = useTranslations("HomePage");

  return (
    <main>
      <section className="container">
        <h1>{t("heroHeadline")}</h1>
        <p>{t("heroSupporting")}</p>
        <p>
          <Link href="/contact" role="button">{t("orderNow")}</Link>{" "}
          <Link href="/contact" role="button" className="secondary">{t("bookCatering")}</Link>{" "}
          <a
            href="https://wa.me/31630545277"
            role="button"
            className="outline"
            style={{ borderColor: "var(--color-whatsapp)", color: "var(--color-whatsapp)" }}
          >
            {t("whatsappUs")}
          </a>
        </p>
      </section>

      <section className="container">
        <ul className="trust-indicators">
          <li>{t("trustHalal")}</li>
          <li>{t("trustFresh")}</li>
          <li>{t("trustHygienic")}</li>
          <li>{t("trustReliable")}</li>
        </ul>
      </section>

      <section className="container">
        <div className="services-overview">
          <article>
            <h3>{t("servicesMeals")}</h3>
            <Link href="/services">{t("learnMore")}</Link>
          </article>
          <article>
            <h3>{t("servicesSnacks")}</h3>
            <Link href="/services">{t("learnMore")}</Link>
          </article>
          <article>
            <h3>{t("servicesCatering")}</h3>
            <Link href="/services">{t("learnMore")}</Link>
          </article>
          <article>
            <h3>{t("servicesEventRentals")}</h3>
            <Link href="/services">{t("learnMore")}</Link>
          </article>
        </div>
      </section>

      <section className="container">
        <h2>{t("whyChooseUs")}</h2>
      </section>

      <section className="container">
        <div className="cta-banner">
          <h2>{t("cateringCtaHeadline")}</h2>
          <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
        </div>
      </section>

      <section className="container">
        <p>{t("aboutPreview")}</p>
        <Link href="/about">{t("learnMore")}</Link>
      </section>
    </main>
  );
}
