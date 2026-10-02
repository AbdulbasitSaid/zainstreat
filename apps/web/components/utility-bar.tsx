import { useTranslations } from "next-intl";

const ITEM_KEYS = ["halal", "fresh", "catering", "whatsapp"] as const;

export function UtilityBar() {
  const t = useTranslations("UtilityBar");

  const renderItems = (copy: "a" | "b") =>
    ITEM_KEYS.map((key) => (
      <span
        key={`${copy}-${key}`}
        className="inline-flex items-center px-6 text-[0.78rem] font-semibold tracking-[0.08em] uppercase after:ml-6 after:opacity-50 after:content-['•']"
      >
        {t(key)}
      </span>
    ));

  return (
    <div className="overflow-hidden whitespace-nowrap bg-primary-dark text-text-light" aria-hidden="true">
      <div className="inline-flex w-max animate-marquee py-[0.55rem] motion-reduce:animate-none">
        {renderItems("a")}
        {renderItems("b")}
      </div>
    </div>
  );
}
