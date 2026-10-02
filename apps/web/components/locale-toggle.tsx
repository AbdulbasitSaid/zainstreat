"use client";

import { useLocale, useTranslations } from "next-intl";
import { usePathname, useRouter } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";

export function LocaleToggle() {
  const t = useTranslations("LocaleToggle");
  const locale = useLocale();
  const pathname = usePathname();
  const router = useRouter();

  return (
    <label>
      {t("label")}:{" "}
      <select
        value={locale}
        onChange={(e) => router.replace(pathname, { locale: e.target.value })}
      >
        {routing.locales.map((l) => (
          <option key={l} value={l}>
            {l === "en" ? "English" : "Nederlands"}
          </option>
        ))}
      </select>
    </label>
  );
}
