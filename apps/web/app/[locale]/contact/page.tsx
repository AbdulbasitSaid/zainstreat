import { useTranslations } from "next-intl";
import { Reveal } from "@/components/reveal";
import { SplitRow } from "@/components/split-row";
import { PageHero } from "@/components/page-hero";
import { ImageSlot } from "@/components/image-slot";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";

export default function ContactPage() {
  const t = useTranslations("ContactPage");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">
          {t.rich("heading", { em: (chunks) => <em>{chunks}</em> })}
        </h1>
      </PageHero>

      <SplitRow
        media={
          <ImageSlot
            label={t("mapPlaceholderLabel")}
            className="min-h-[240px]"
          />
        }
        content={
          <ul className="m-0 flex list-none flex-col gap-3.5 p-0 text-[1.05rem] [&_a]:font-semibold [&_a]:text-primary [&_a:hover]:underline">
            <li>{t("phoneLabel")}: <a href="tel:+31630545277">{t("phoneValue")}</a></li>
            <li>{t("emailLabel")}: {t("emailPlaceholder")}</li>
            <li>{t("whatsappLabel")}: <a href="https://wa.me/31630545277" target="_blank" rel="noopener noreferrer">{t("whatsappValue")}</a></li>
            <li>{t("addressLabel")}: {t("addressPlaceholder")}</li>
            <li>{t("hoursLabel")}: {t("hoursPlaceholder")}</li>
            <li>{t("instagramLabel")}: <a href="https://instagram.com/zain_treats" target="_blank" rel="noopener noreferrer">{t("instagramHandle")}</a></li>
            <li>{t("tiktokLabel")}: {t("tiktokHandle")}</li>
          </ul>
        }
      />

      <Reveal>
        <section className="mx-auto max-w-[640px]">
          <form>
            <div className="mb-5">
              <label htmlFor="contact-name" className={FIELD_LABEL_CLASS}>{t("formName")}</label>
              <input id="contact-name" name="name" type="text" className="field" />
            </div>

            <div className="mb-5">
              <label htmlFor="contact-email" className={FIELD_LABEL_CLASS}>{t("formEmail")}</label>
              <input id="contact-email" name="email" type="email" className="field" />
            </div>

            <div className="mb-5">
              <label htmlFor="contact-phone" className={FIELD_LABEL_CLASS}>{t("formPhone")}</label>
              <input id="contact-phone" name="phone" type="tel" className="field" />
            </div>

            <div className="mb-5">
              <label htmlFor="contact-subject" className={FIELD_LABEL_CLASS}>{t("formSubject")}</label>
              <select id="contact-subject" name="subject" className="field">
                <option value="general">{t("formSubjectGeneral")}</option>
                <option value="catering">{t("formSubjectCatering")}</option>
                <option value="eventRental">{t("formSubjectEventRental")}</option>
                <option value="menu">{t("formSubjectMenu")}</option>
                <option value="order">{t("formSubjectOrder")}</option>
                <option value="other">{t("formSubjectOther")}</option>
              </select>
            </div>

            <div className="mb-5">
              <label htmlFor="contact-message" className={FIELD_LABEL_CLASS}>{t("formMessage")}</label>
              <textarea id="contact-message" name="message" rows={5} className="field" />
            </div>

            <Button type="submit" disabled className="mb-4">
              {t("sendMessage")}
            </Button>
            <Notice>{t("formComingSoon")}</Notice>
          </form>
        </section>
      </Reveal>
    </main>
  );
}
