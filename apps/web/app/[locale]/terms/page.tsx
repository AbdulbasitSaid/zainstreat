import { useTranslations } from "next-intl";

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
    <main className="container">
      <h1>{t("heading")}</h1>
      <p className="notice-banner">{t("legalReviewNotice")}</p>

      {SECTIONS.map((section) => (
        <section key={section}>
          <h2>{t(`${section}Heading`)}</h2>
          <p>{t(`${section}Copy`)}</p>
        </section>
      ))}
    </main>
  );
}
