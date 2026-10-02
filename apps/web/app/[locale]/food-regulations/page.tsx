import { useTranslations } from "next-intl";

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
    <main className="container">
      <h1>{t("heading")}</h1>

      <section>
        <h2>{t("halalHeading")}</h2>
        <p>{t("halalCopy")}</p>
      </section>

      <section>
        <h2>{t("hygieneHeading")}</h2>
        <p>{t("hygieneCopy")}</p>
      </section>

      <section>
        <h2>{t("ingredientsHeading")}</h2>
        <p>{t("ingredientsCopy")}</p>
      </section>

      <section>
        <h2>{t("allergensHeading")}</h2>
        <p className="notice-banner">{t("allergensIntro")}</p>
        <p>{t("allergensList")}</p>
        <ul>
          {ALLERGENS.map((allergen) => (
            <li key={allergen}>{t(`allergen${allergen}`)}</li>
          ))}
        </ul>
      </section>
    </main>
  );
}
