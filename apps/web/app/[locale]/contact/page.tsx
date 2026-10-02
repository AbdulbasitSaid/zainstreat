import { useTranslations } from "next-intl";

export default function ContactPage() {
  const t = useTranslations("ContactPage");

  return (
    <main className="container">
      <h1>{t("heading")}</h1>

      <section>
        <ul>
          <li>{t("phoneLabel")}: {t("phonePlaceholder")}</li>
          <li>{t("emailLabel")}: {t("emailPlaceholder")}</li>
          <li>{t("whatsappLabel")}: {t("whatsappPlaceholder")}</li>
          <li>{t("addressLabel")}: {t("addressPlaceholder")}</li>
          <li>{t("hoursLabel")}: {t("hoursPlaceholder")}</li>
        </ul>
      </section>

      <section>
        <div
          aria-label={t("mapPlaceholderLabel")}
          style={{
            background: "var(--color-background-soft)",
            minHeight: "240px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          {t("mapPlaceholderLabel")}
        </div>
      </section>

      <section>
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
    </main>
  );
}
