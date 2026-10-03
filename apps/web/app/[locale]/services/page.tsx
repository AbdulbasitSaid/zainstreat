import { useTranslations } from "next-intl";
import { PlaceholderImage } from "@/components/placeholder-image";
import { SplitRow } from "@/components/split-row";
import { PageHero } from "@/components/page-hero";
import { ButtonLink } from "@/components/button";

function SplitRowIndex({ children }: { children: string }) {
  return (
    <span
      aria-hidden="true"
      className="mb-2 block text-[clamp(2.25rem,4vw,3.25rem)] leading-none font-bold text-accent-light"
    >
      {children}
    </span>
  );
}

export default function ServicesPage() {
  const t = useTranslations("ServicesPage");
  const supportingClass = "max-w-[48ch] text-[clamp(1.05rem,1.5vw,1.25rem)] text-text-muted";

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">
          {t.rich("heading", { em: (chunks) => <em>{chunks}</em> })}
        </h1>
      </PageHero>

      <SplitRow
        media={<PlaceholderImage label={t("mealsHeading")} />}
        content={
          <div>
            <SplitRowIndex>01</SplitRowIndex>
            <h2>{t("mealsHeading")}</h2>
            <p className={supportingClass}>{t("mealsCopy")}</p>
          </div>
        }
      />

      <SplitRow
        tinted
        reverse
        media={<PlaceholderImage label={t("snacksHeading")} />}
        content={
          <div>
            <SplitRowIndex>02</SplitRowIndex>
            <h2>{t("snacksHeading")}</h2>
            <p className={supportingClass}>{t("snacksCopy")}</p>
          </div>
        }
      />

      <SplitRow
        media={<PlaceholderImage label={t("cateringHeading")} />}
        content={
          <div>
            <SplitRowIndex>03</SplitRowIndex>
            <h2>{t("cateringHeading")}</h2>
            <p className={supportingClass}>{t("cateringCopy")}</p>
            <ButtonLink href="/contact" className="mt-4">
              {t("requestCateringQuote")}
            </ButtonLink>
          </div>
        }
      />

      <SplitRow
        tinted
        reverse
        media={<PlaceholderImage label={t("eventRentalsHeading")} />}
        content={
          <div>
            <SplitRowIndex>04</SplitRowIndex>
            <h2>{t("eventRentalsHeading")}</h2>
            <p className={supportingClass}>{t("eventRentalsCopy")}</p>
            <ButtonLink href="/contact" className="mt-4">
              {t("requestQuote")}
            </ButtonLink>
          </div>
        }
      />
    </main>
  );
}
