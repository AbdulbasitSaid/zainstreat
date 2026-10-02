import { useTranslations } from "next-intl";
import { Reveal } from "@/components/reveal";

export default function ContactPage() {
  const t = useTranslations("ContactPage");

  return (
    <main className="container">
      <section className="page-hero">
        <Reveal>
          <h1 className="hero-heading">
            {t.rich("heading", { em: (chunks) => <em>{chunks}</em> })}
          </h1>
        </Reveal>
      </section>

      <Reveal>
        <div className="contact-grid">
          <section>
            <ul className="contact-details">
              <li>{t("phoneLabel")}: <a href="tel:+31630545277">{t("phoneValue")}</a></li>
              <li>{t("emailLabel")}: {t("emailPlaceholder")}</li>
              <li>{t("whatsappLabel")}: <a href="https://wa.me/31630545277" target="_blank" rel="noopener noreferrer">{t("whatsappValue")}</a></li>
              <li>{t("addressLabel")}: {t("addressPlaceholder")}</li>
              <li>{t("hoursLabel")}: {t("hoursPlaceholder")}</li>
              <li>{t("instagramLabel")}: <a href="https://instagram.com/zain_treats" target="_blank" rel="noopener noreferrer">{t("instagramHandle")}</a></li>
              <li>{t("tiktokLabel")}: {t("tiktokHandle")}</li>
            </ul>
          </section>

          <section>
            <div
              aria-label={t("mapPlaceholderLabel")}
              className="image-slot"
              style={{ minHeight: "240px" }}
            >
              {t("mapPlaceholderLabel")}
            </div>
          </section>
        </div>
      </Reveal>

      <Reveal>
        <section style={{ maxWidth: "640px", marginInline: "auto" }}>
          <form>
            <label htmlFor="contact-name">{t("formName")}</label>
            <input id="contact-name" name="name" type="text" />

            <label htmlFor="contact-email">{t("formEmail")}</label>
            <input id="contact-email" name="email" type="email" />

            <label htmlFor="contact-phone">{t("formPhone")}</label>
            <input id="contact-phone" name="phone" type="tel" />

            <label htmlFor="contact-subject">{t("formSubject")}</label>
            <select id="contact-subject" name="subject">
              <option value="general">{t("formSubjectGeneral")}</option>
              <option value="catering">{t("formSubjectCatering")}</option>
              <option value="eventRental">{t("formSubjectEventRental")}</option>
              <option value="menu">{t("formSubjectMenu")}</option>
              <option value="order">{t("formSubjectOrder")}</option>
              <option value="other">{t("formSubjectOther")}</option>
            </select>

            <label htmlFor="contact-message">{t("formMessage")}</label>
            <textarea id="contact-message" name="message" rows={5} />

            <button type="submit" disabled>
              {t("sendMessage")}
            </button>
            <p className="notice-banner">{t("formComingSoon")}</p>
          </form>
        </section>
      </Reveal>
    </main>
  );
}
