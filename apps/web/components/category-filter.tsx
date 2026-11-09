"use client";

import type { MouseEvent } from "react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { PillLiquid, pillClasses } from "@/components/button";
import { useMenuFilter } from "@/components/menu-filter-transition";
import type { Category } from "@/lib/api";

// Room for the sticky header when scrolling the filter bar back into view.
const HEADER_OFFSET_PX = 96;

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
    if (categoryId === activeId) return;
    // Scrolled past the filter bar? Bring it back, or a shorter category
    // could land above the viewport and leave the screen blank.
    const nav = event.currentTarget.closest("nav");
    if (nav && nav.getBoundingClientRect().top < HEADER_OFFSET_PX) {
      window.scrollTo({ top: nav.getBoundingClientRect().top + window.scrollY - HEADER_OFFSET_PX, behavior: "smooth" });
    }
    select(categoryId);
  };

  return (
    <nav aria-label={t("categoryNavLabel")} className="mb-10 flex flex-wrap gap-3">
      <Link
        href="/menu"
        aria-current={activeId === undefined ? "page" : undefined}
        className={pillClasses(activeId === undefined)}
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
            className={pillClasses(isActive)}
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
