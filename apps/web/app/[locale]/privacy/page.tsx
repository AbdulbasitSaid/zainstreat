import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { connection } from "next/server";
import type { ReactNode } from "react";
import { Reveal } from "@/components/reveal";
import { localeAlternates } from "@/lib/seo";
import { WHATSAPP_NUMBER } from "@/lib/whatsapp";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "PrivacyPage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
    alternates: localeAlternates(locale, "/privacy"),
  };
}

// Phase 33 Decision 12 (GDPR Art. 13). Each section is a heading, an
// optional intro paragraph and an optional list.
const SECTIONS = [
  { key: "collect", intro: "collectIntro", items: ["collectOrders", "collectEvents", "collectCatering", "collectContact", "collectTechnical"] },
  { key: "why", items: ["whyContract", "whyQuote", "whyLegal", "whyInterest"], outro: "whyNot" },
  { key: "sharing", intro: "sharingIntro", items: ["sharingHosting", "sharingMail", "sharingPush", "sharingDelivery"], outro: ["sharingWhatsapp", "sharingTransfers"] },
  { key: "retention", items: ["retentionBookkeeping", "retentionShort", "retentionEmail", "retentionLogs", "retentionBackups"] },
  { key: "cookies", intro: "cookiesCopy", items: ["cookiesLocale", "cookiesCart", "cookiesOrder"], outro: "cookiesConsent" },
] as const;

const EMAIL = "contact@zainstreat.com";
const COMPLAINTS_URL = "https://autoriteitpersoonsgegevens.nl/en/contact-the-dutch-dpa";

function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a href={href} target="_blank" rel="noopener noreferrer">
      {children}
    </a>
  );
}

export default async function PrivacyPage() {
  // The KvK number is a runtime env var (as on the invoice), so this page
  // renders per request rather than at build time.
  await connection();
  const kvk = process.env.BUSINESS_KVK?.trim();
  const t = await getTranslations("PrivacyPage");
  const email = (chunks: ReactNode) => <a href={`mailto:${EMAIL}`}>{chunks}</a>;

  return (
    <main className="container max-w-[68ch]">
      <h1>{t("heading")}</h1>
      <p className="text-sm text-text-muted">{t("lastUpdated")}</p>
      <p>{t("intro")}</p>

      <Reveal>
        <section>
          <h2>{t("whoHeading")}</h2>
          <p>{t("whoCopy")}</p>
          {kvk && <p>{t("whoKvk", { number: kvk })}</p>}
          <p>
            {t.rich("whoContact", {
              email,
              whatsapp: (chunks) => <ExternalLink href={`https://wa.me/${WHATSAPP_NUMBER}`}>{chunks}</ExternalLink>,
            })}
          </p>
        </section>
      </Reveal>

      {SECTIONS.map((section) => (
        <Reveal key={section.key}>
          <section>
            <h2>{t(`${section.key}Heading`)}</h2>
            {"intro" in section && <p>{t(section.intro)}</p>}
            <ul>
              {section.items.map((item) => (
                <li key={item}>{t(item)}</li>
              ))}
            </ul>
            {"outro" in section &&
              [section.outro].flat().map((outro) => <p key={outro}>{t(outro)}</p>)}
          </section>
        </Reveal>
      ))}

      <Reveal>
        <section>
          <h2>{t("rightsHeading")}</h2>
          <p>{t("rightsCopy")}</p>
          <p>{t.rich("rightsHow", { email })}</p>
        </section>
      </Reveal>

      <Reveal>
        <section>
          <h2>{t("complaintHeading")}</h2>
          <p>
            {t.rich("complaintCopy", {
              ap: (chunks) => <ExternalLink href={COMPLAINTS_URL}>{chunks}</ExternalLink>,
            })}
          </p>
        </section>
      </Reveal>

      {(["security", "changes"] as const).map((key) => (
        <Reveal key={key}>
          <section>
            <h2>{t(`${key}Heading`)}</h2>
            <p>{t(`${key}Copy`)}</p>
          </section>
        </Reveal>
      ))}
    </main>
  );
}
