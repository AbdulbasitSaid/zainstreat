import { useTranslations } from "next-intl";
import { Reveal } from "@/components/reveal";
import { DecorativeShape } from "@/components/decorative-shape";
import { ImageSlot } from "@/components/image-slot";
import { SplitRow } from "@/components/split-row";
import { CheckList } from "@/components/check-list";

export default function AboutPage() {
  const t = useTranslations("AboutPage");

  const values = ["value1", "value2", "value3", "value4", "value5", "value6"].map((key) => t(key));
  const offers = ["offer1", "offer2", "offer3", "offer4", "offer5"].map((key) => t(key));

  return (
    <main className="container">
      <SplitRow
        className="relative overflow-hidden"
        decoration={
          <DecorativeShape style={{ width: "160px", height: "160px", top: "-30px", right: "-40px" }} />
        }
        content={
          <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">
            {t.rich("whoWeAre", { em: (chunks) => <em>{chunks}</em> })}
          </h1>
        }
        media={<ImageSlot label={t.markup("whoWeAre", { em: (chunks) => chunks })} />}
      />

      <SplitRow
        tinted
        reverse
        media={<ImageSlot label={t("ourMission")} />}
        content={
          <div>
            <h2>{t("ourMission")}</h2>
            <p className="max-w-[48ch] text-[clamp(1.05rem,1.5vw,1.25rem)] text-text-muted">
              {t("missionCopy")}
            </p>
          </div>
        }
      />

      <section>
        <Reveal>
          <h2>{t("ourValues")}</h2>
          <CheckList items={values} />
        </Reveal>
      </section>

      <section>
        <Reveal>
          <h2>{t("whatWeOffer")}</h2>
          <CheckList items={offers} />
        </Reveal>
      </section>
    </main>
  );
}
