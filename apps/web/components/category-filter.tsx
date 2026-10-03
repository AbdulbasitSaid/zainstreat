import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import type { Category } from "@/lib/api";

const PILL_BASE_CLASS =
  "inline-flex items-center rounded-full px-5 py-2 text-sm font-semibold tracking-wide transition-colors";

function pillClass(isActive: boolean) {
  return `${PILL_BASE_CLASS} ${
    isActive ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
  }`;
}

export function CategoryFilter({
  categories,
  selectedCategoryId,
}: {
  categories: Category[];
  selectedCategoryId: number | undefined;
}) {
  const t = useTranslations("MenuPage");

  return (
    <nav aria-label={t("categoryNavLabel")} className="mb-10 flex flex-wrap gap-3">
      <Link
        href="/menu"
        aria-current={selectedCategoryId === undefined ? "page" : undefined}
        className={pillClass(selectedCategoryId === undefined)}
      >
        {t("allCategories")}
      </Link>
      {categories.map((category) => {
        const isActive = category.id === selectedCategoryId;
        return (
          <Link
            key={category.id}
            href={{ pathname: "/menu", query: { category: category.id } }}
            aria-current={isActive ? "page" : undefined}
            className={pillClass(isActive)}
          >
            {category.name}
          </Link>
        );
      })}
    </nav>
  );
}
