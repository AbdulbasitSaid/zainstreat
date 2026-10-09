import { getTranslations } from "next-intl/server";
import { Reveal } from "@/components/reveal";
import { SplitRow } from "@/components/split-row";
import { PageHero } from "@/components/page-hero";
import { SiteImage } from "@/components/site-image";
import { ContactFormsTabs } from "@/components/contact-forms-tabs";
import contact from "@/assets/images/contact.jpg";
import { buildWhatsAppLink } from "@/lib/whatsapp";

export default async function ContactPage({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  const initialTab = tab === "catering" ? "catering" : "general";
  const t = await getTranslations("ContactPage");
  const tw = await getTranslations("Whatsapp");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">
          {t.rich("heading", { em: (chunks) => <em>{chunks}</em> })}
        </h1>
      </PageHero>

      <SplitRow
        media={
          <SiteImage
            src={contact}
            alt={t("mapPlaceholderLabel")}
            className="min-h-[240px]"
          />
        }
        content={
          <ul className="m-0 flex list-none flex-col gap-3.5 p-0 text-[1.05rem] [&_a]:font-semibold [&_a]:text-primary [&_a:hover]:underline">
            <li>{t("phoneLabel")}: <a href="tel:+31630545277">{t("phoneValue")}</a></li>
            <li>{t("emailLabel")}: <a href="mailto:zainstreat@gmail.com">{t("emailValue")}</a></li>
            <li>{t("whatsappLabel")}: <a href={buildWhatsAppLink(tw("prefilledMessage"))} target="_blank" rel="noopener noreferrer">{t("whatsappValue")}</a></li>
            <li>{t("hoursLabel")}: {t("hoursValue")}</li>
            <li>{t("instagramLabel")}: <a href="https://instagram.com/zain_treats" target="_blank" rel="noopener noreferrer">{t("instagramHandle")}</a></li>
            <li>{t("tiktokLabel")}: {t("tiktokHandle")}</li>
          </ul>
        }
      />

      <Reveal>
        <section className="mx-auto max-w-[640px]">
          <ContactFormsTabs initialTab={initialTab} />
        </section>
      </Reveal>
    </main>
  );
}
