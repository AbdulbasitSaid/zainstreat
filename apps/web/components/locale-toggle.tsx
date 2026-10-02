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
    <label className="inline-flex items-center gap-2 text-sm font-medium text-text">
      {t("label")}
      <select
        value={locale}
        onChange={(e) => router.replace(pathname, { locale: e.target.value })}
        className="rounded-full border border-text/20 bg-transparent px-3 py-1.5 text-sm text-text focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
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
