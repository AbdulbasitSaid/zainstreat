"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { ContactForm } from "@/components/contact-form";
import { CateringEnquiryForm } from "@/components/catering-enquiry-form";
import { LiquidSurface } from "@/components/liquid-surface";

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
            className={`group/liquid relative isolate rounded-full px-4 py-2 text-sm font-semibold transition-colors duration-150 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              tab === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value === "general" ? t("tabGeneral") : t("tabCatering")}
            <LiquidSurface
              pathClassName={`transition-[fill] duration-150 ${
                tab === value ? "fill-primary" : "fill-card group-hover/liquid:fill-background-soft"
              }`}
              splashClassName="fill-primary"
            />
          </button>
        ))}
      </div>

      {tab === "general" ? <ContactForm /> : <CateringEnquiryForm />}
    </div>
  );
}
