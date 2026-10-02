import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { Reveal } from "@/components/reveal";
import { DecorativeShape } from "@/components/decorative-shape";
import { ImageSlot } from "@/components/image-slot";

export default function Home() {
  const t = useTranslations("HomePage");

  return (
    <main>
      <section
        className="container"
        style={{ position: "relative", overflow: "hidden" }}
      >
        <DecorativeShape
          style={{ width: "220px", height: "220px", top: "-60px", right: "-60px" }}
        />
        <DecorativeShape
          style={{ width: "120px", height: "120px", bottom: "-30px", left: "-30px" }}
        />
        <Reveal>
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
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <ul className="trust-indicators">
            <li>{t("trustHalal")}</li>
            <li>{t("trustFresh")}</li>
            <li>{t("trustHygienic")}</li>
            <li>{t("trustReliable")}</li>
          </ul>
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
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
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <h2>{t("whyChooseUs")}</h2>
        </Reveal>
      </section>

      <section className="container" style={{ position: "relative", overflow: "hidden" }}>
        <DecorativeShape
          style={{ width: "160px", height: "160px", top: "-40px", left: "50%" }}
        />
        <Reveal>
          <div className="cta-banner">
            <h2>{t("cateringCtaHeadline")}</h2>
            <Link href="/contact" role="button">{t("requestCateringQuote")}</Link>
          </div>
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <p>{t("aboutPreview")}</p>
          <ImageSlot label={t("aboutPreview")} />
          <Link href="/about">{t("learnMore")}</Link>
        </Reveal>
      </section>
    </main>
  );
}
