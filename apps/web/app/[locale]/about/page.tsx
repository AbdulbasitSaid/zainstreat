import { useTranslations } from "next-intl";

export default function AboutPage() {
  const t = useTranslations("AboutPage");

  return (
    <main className="container">
      <section>
        <h1>{t("whoWeAre")}</h1>
      </section>

      <section>
        <h2>{t("ourMission")}</h2>
        <p>{t("missionCopy")}</p>
      </section>

      <section>
        <h2>{t("ourValues")}</h2>
        <ul>
          <li>{t("value1")}</li>
          <li>{t("value2")}</li>
          <li>{t("value3")}</li>
          <li>{t("value4")}</li>
          <li>{t("value5")}</li>
          <li>{t("value6")}</li>
        </ul>
      </section>

      <section>
        <h2>{t("whatWeOffer")}</h2>
        <ul>
          <li>{t("offer1")}</li>
          <li>{t("offer2")}</li>
          <li>{t("offer3")}</li>
          <li>{t("offer4")}</li>
          <li>{t("offer5")}</li>
        </ul>
      </section>
    </main>
  );
}
