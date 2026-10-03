import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { CartView } from "@/components/cart-view";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "Cart" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}

export default async function CartPage() {
  const t = await getTranslations("Cart");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
      </PageHero>
      <CartView />
    </main>
  );
}
