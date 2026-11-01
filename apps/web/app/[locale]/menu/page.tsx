import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { Notice } from "@/components/notice";
import { CategoryFilter } from "@/components/category-filter";
import { MenuBentoGrid } from "@/components/menu-bento-grid";
import { getCategories, getMenuItems } from "@/lib/api";
import { localeAlternates } from "@/lib/seo";

export const dynamic = "force-dynamic";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "MenuPage" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
    alternates: localeAlternates(locale, "/menu"),
  };
}

function parseCategoryId(value: string | string[] | undefined): number | undefined {
  const raw = Array.isArray(value) ? value[0] : value;
  const parsed = raw ? Number.parseInt(raw, 10) : NaN;
  return Number.isNaN(parsed) ? undefined : parsed;
}

export default async function MenuPage({
  searchParams,
}: {
  searchParams: Promise<{ category?: string | string[] }>;
}) {
  const t = await getTranslations("MenuPage");
  const resolvedSearchParams = await searchParams;
  const selectedCategoryId = parseCategoryId(resolvedSearchParams.category);

  const [categories, items] = await Promise.all([
    getCategories(),
    getMenuItems(selectedCategoryId),
  ]);

  const visibleCategories =
    selectedCategoryId === undefined
      ? categories
      : categories.filter((category) => category.id === selectedCategoryId);

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
      </PageHero>

      <CategoryFilter categories={categories} selectedCategoryId={selectedCategoryId} />

      {visibleCategories.map((category) => {
        const categoryItems = items.filter((item) => item.category_id === category.id);

        return (
          <section key={category.id}>
            <h2>{category.name}</h2>
            {categoryItems.length > 0 ? (
              <MenuBentoGrid items={categoryItems} />
            ) : (
              <Notice>{t("emptyCategory")}</Notice>
            )}
          </section>
        );
      })}
    </main>
  );
}
