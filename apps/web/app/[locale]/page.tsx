import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { Reveal } from "@/components/reveal";
import { DecorativeShape } from "@/components/decorative-shape";
import { ImageSlot } from "@/components/image-slot";

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
          <div className="split-row">
            <div>
              <span className="eyebrow">{t("heroEyebrow")}</span>
              <h1 className="hero-heading">
                {t.rich("heroHeadline", { em: (chunks) => <em>{chunks}</em> })}
              </h1>
              <p className="hero-supporting">{t("heroSupporting")}</p>
              <div className="hero-actions">
                <Link href="/contact" role="button">{t("orderNow")}</Link>
                <Link href="/contact" role="button" className="secondary">{t("bookCatering")}</Link>
                <a
                  href="https://wa.me/31630545277"
                  role="button"
                  className="outline"
                  style={{ borderColor: "var(--color-whatsapp-dark)", color: "var(--color-whatsapp-dark)" }}
                >
                  {t("whatsappUs")}
                </a>
              </div>
            </div>
            <ImageSlot label={t.markup("heroHeadline", { em: (chunks) => chunks })} />
          </div>
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <div className="category-row">
            <div className="category-row-item">
              <ImageSlot label={t("servicesMeals")} />
              <h3>{t("servicesMeals")}</h3>
              <Link href="/services">{t("learnMore")}</Link>
            </div>
            <div className="category-row-item">
              <ImageSlot label={t("servicesSnacks")} />
              <h3>{t("servicesSnacks")}</h3>
              <Link href="/services">{t("learnMore")}</Link>
            </div>
            <div className="category-row-item">
              <ImageSlot label={t("servicesCatering")} />
              <h3>{t("servicesCatering")}</h3>
              <Link href="/services">{t("learnMore")}</Link>
            </div>
            <div className="category-row-item">
              <ImageSlot label={t("servicesEventRentals")} />
              <h3>{t("servicesEventRentals")}</h3>
              <Link href="/services">{t("learnMore")}</Link>
            </div>
          </div>
        </Reveal>
      </section>

      <section className="container">
        <Reveal>
          <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
            <span className="eyebrow">{t("whyChooseUsEyebrow")}</span>
            <h2>
              {t.rich("whyChooseUs", { em: (chunks) => <em>{chunks}</em> })}
            </h2>
          </div>
          <div className="promise-grid">
            <div className="promise-grid-item">
              <span className="promise-icon"><ShieldCheckIcon /></span>
              <h3>{t("trustHalal")}</h3>
              <p>{t("trustHalalDesc")}</p>
            </div>
            <div className="promise-grid-item">
              <span className="promise-icon"><SparkleIcon /></span>
              <h3>{t("trustFresh")}</h3>
              <p>{t("trustFreshDesc")}</p>
            </div>
            <div className="promise-grid-item">
              <span className="promise-icon"><DropletIcon /></span>
              <h3>{t("trustHygienic")}</h3>
              <p>{t("trustHygienicDesc")}</p>
            </div>
            <div className="promise-grid-item">
              <span className="promise-icon"><ClockIcon /></span>
              <h3>{t("trustReliable")}</h3>
              <p>{t("trustReliableDesc")}</p>
            </div>
          </div>
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
          <div className="split-row">
            <ImageSlot label={t("aboutPreview")} />
            <div>
              <p className="hero-supporting">{t("aboutPreview")}</p>
              <Link href="/about" role="button" className="outline">{t("learnMore")}</Link>
            </div>
          </div>
        </Reveal>
      </section>
    </main>
  );
}
