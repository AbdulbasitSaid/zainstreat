import { useTranslations } from "next-intl";
import { LocaleToggle } from "@/components/locale-toggle";

export default function Home() {
  const t = useTranslations("HomePage");

  return (
    <main className="container">
      <article>
        <h1>{t("title")}</h1>
        <p>{t("subtitle")}</p>
        <LocaleToggle />
      </article>
    </main>
  );
}
