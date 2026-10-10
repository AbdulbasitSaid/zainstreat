import type { Metadata } from "next";
import { useTranslations } from "next-intl";
import { getTranslations } from "next-intl/server";
import { Reveal } from "@/components/reveal";
import { Notice } from "@/components/notice";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "TermsPage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}

// Legal page: calm fade-up only — no decorative shapes, no stagger.
const SECTIONS = [
  "ordering",
  "payment",
  "cancellation",
  "refunds",
  "delivery",
  "catering",
  "eventBookings",
  "customerResponsibilities",
  "websiteUsage",
  "changes",
] as const;

export default function TermsPage() {
  const t = useTranslations("TermsPage");

  return (
    <main className="container max-w-[68ch]">
      <h1>{t("heading")}</h1>
      <Notice>{t("legalReviewNotice")}</Notice>

      {SECTIONS.map((section) => (
        <Reveal key={section}>
          <section>
            <h2>{t(`${section}Heading`)}</h2>
            <p>{t(`${section}Copy`)}</p>
          </section>
        </Reveal>
      ))}
    </main>
  );
}
