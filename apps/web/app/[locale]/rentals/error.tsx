"use client";

import { useTranslations } from "next-intl";
import { Notice } from "@/components/notice";

export default function RentalsError() {
  const t = useTranslations("RentalsPage");

  return (
    <main className="container">
      <Notice className="my-section">{t("loadError")}</Notice>
    </main>
  );
}
