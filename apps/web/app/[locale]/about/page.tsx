import { useTranslations } from "next-intl";
import { Reveal } from "@/components/reveal";
import { DecorativeShape } from "@/components/decorative-shape";
import { ImageSlot } from "@/components/image-slot";

export default function AboutPage() {
  const t = useTranslations("AboutPage");

  return (
    <main className="container">
      <section style={{ position: "relative", overflow: "hidden" }}>
        <DecorativeShape
          style={{ width: "160px", height: "160px", top: "-30px", right: "-40px" }}
        />
        <Reveal>
          <div className="split-row">
            <div>
              <h1 className="hero-heading">{t("whoWeAre")}</h1>
            </div>
            <ImageSlot label={t("whoWeAre")} />
          </div>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <div className="split-row split-row--reverse">
            <ImageSlot label={t("ourMission")} />
            <div>
              <h2>{t("ourMission")}</h2>
              <p className="hero-supporting">{t("missionCopy")}</p>
            </div>
          </div>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("ourValues")}</h2>
          <ul className="check-grid">
            <li>{t("value1")}</li>
            <li>{t("value2")}</li>
            <li>{t("value3")}</li>
            <li>{t("value4")}</li>
            <li>{t("value5")}</li>
            <li>{t("value6")}</li>
          </ul>
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("whatWeOffer")}</h2>
          <ul className="check-grid">
            <li>{t("offer1")}</li>
            <li>{t("offer2")}</li>
            <li>{t("offer3")}</li>
            <li>{t("offer4")}</li>
            <li>{t("offer5")}</li>
          </ul>
        </Reveal>
      </section>
    </main>
  );
}
