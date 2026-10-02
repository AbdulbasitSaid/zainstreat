import { useTranslations } from "next-intl";

const ITEM_KEYS = ["halal", "fresh", "catering", "whatsapp"] as const;

export function UtilityBar() {
  const t = useTranslations("UtilityBar");

  const renderItems = (copy: "a" | "b") =>
    ITEM_KEYS.map((key) => (
      <span key={`${copy}-${key}`} className="marquee-item">
        {t(key)}
      </span>
    ));

  return (
    <div className="marquee" aria-hidden="true">
      <div className="marquee-track">
        {renderItems("a")}
        {renderItems("b")}
      </div>
    </div>
  );
}
