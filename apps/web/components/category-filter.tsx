"use client";

import type { MouseEvent } from "react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { LiquidSurface } from "@/components/liquid-surface";
import { useMenuFilter } from "@/components/menu-filter-transition";
import type { Category } from "@/lib/api";

const PILL_BASE_CLASS =
  "group/liquid relative isolate inline-flex items-center rounded-full px-5 py-2 text-sm font-semibold tracking-wide transition-colors";

function pillClass(isActive: boolean) {
  return `${PILL_BASE_CLASS} ${
    isActive ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
  }`;
}

function PillLiquid({ isActive }: { isActive: boolean }) {
  return (
    <LiquidSurface
      pathClassName={`transition-[fill] duration-200 ${
        isActive ? "fill-primary" : "fill-card group-hover/liquid:fill-background-soft"
      }`}
      splashClassName="fill-primary"
    />
  );
}

// Plain clicks filter in place (MenuFilterProvider); modified clicks keep
// the link's normal behaviour (new tab etc.).
function isPlainClick(event: MouseEvent) {
  return event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
}

export function CategoryFilter({
  categories,
  selectedCategoryId,
}: {
  categories: Category[];
  selectedCategoryId: number | undefined;
}) {
  const t = useTranslations("MenuPage");
  const { optimisticCategoryId, select } = useMenuFilter();
  const activeId = optimisticCategoryId === null ? selectedCategoryId : optimisticCategoryId;

  const handleClick = (categoryId: number | undefined) => (event: MouseEvent) => {
    if (!isPlainClick(event)) return;
    event.preventDefault();
    if (categoryId !== activeId) select(categoryId);
  };

  return (
    <nav aria-label={t("categoryNavLabel")} className="mb-10 flex flex-wrap gap-3">
      <Link
        href="/menu"
        aria-current={activeId === undefined ? "page" : undefined}
        className={pillClass(activeId === undefined)}
        data-no-page-transition
        onClick={handleClick(undefined)}
      >
        {t("allCategories")}
        <PillLiquid isActive={activeId === undefined} />
      </Link>
      {categories.map((category) => {
        const isActive = category.id === activeId;
        return (
          <Link
            key={category.id}
            href={{ pathname: "/menu", query: { category: category.id } }}
            aria-current={isActive ? "page" : undefined}
            className={pillClass(isActive)}
            data-no-page-transition
            onClick={handleClick(category.id)}
          >
            {category.name}
            <PillLiquid isActive={isActive} />
          </Link>
        );
      })}
    </nav>
  );
}
