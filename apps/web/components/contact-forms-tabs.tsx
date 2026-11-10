"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { ContactForm } from "@/components/contact-form";
import { CateringEnquiryForm } from "@/components/catering-enquiry-form";
import { PillLiquid, pillClasses } from "@/components/button";

type Tab = "general" | "catering";

export function ContactFormsTabs({ initialTab }: { initialTab: Tab }) {
  const t = useTranslations("ContactPage");
  const [tab, setTab] = useState<Tab>(initialTab);

  return (
    <div>
      <div role="tablist" aria-label={t("formTabsLabel")} className="mb-6 flex gap-2">
        {(["general", "catering"] as const).map((value) => (
          <button
            key={value}
            type="button"
            role="tab"
            aria-selected={tab === value}
            onClick={() => setTab(value)}
            className={pillClasses(tab === value)}
          >
            {value === "general" ? t("tabGeneral") : t("tabCatering")}
            <PillLiquid isActive={tab === value} />
          </button>
        ))}
      </div>

      {tab === "general" ? <ContactForm /> : <CateringEnquiryForm />}
    </div>
  );
}
