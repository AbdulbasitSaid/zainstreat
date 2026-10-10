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
  const t = await getTranslations({ locale, namespace: "FoodRegulationsPage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}

// Legal page: calm fade-up only — no decorative shapes, no stagger.
const ALLERGENS = [
  "Milk",
  "Eggs",
  "Wheat",
  "Nuts",
  "Soy",
] as const;

export default function FoodRegulationsPage() {
  const t = useTranslations("FoodRegulationsPage");

  return (
    <main className="container max-w-[68ch]">
      <h1>{t("heading")}</h1>

      <Reveal>
        <section>
          <h2>{t("halalHeading")}</h2>
          <p>{t("halalCopy")}</p>
        </section>
      </Reveal>

      <Reveal>
        <section>
          <h2>{t("hygieneHeading")}</h2>
          <p>{t("hygieneCopy")}</p>
        </section>
      </Reveal>

      <Reveal>
        <section>
          <h2>{t("ingredientsHeading")}</h2>
          <p>{t("ingredientsCopy")}</p>
        </section>
      </Reveal>

      <Reveal>
        <section>
          <h2>{t("allergensHeading")}</h2>
          <Notice>{t("allergensIntro")}</Notice>
          <p>{t("allergensList")}</p>
          <ul className="list-disc space-y-1 pl-6">
            {ALLERGENS.map((allergen) => (
              <li key={allergen}>{t(`allergen${allergen}`)}</li>
            ))}
          </ul>
        </section>
      </Reveal>
    </main>
  );
}
