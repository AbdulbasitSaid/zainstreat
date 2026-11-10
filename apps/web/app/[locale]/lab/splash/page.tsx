import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { SplashLab, type SplashLabSlide } from "@/components/splash-lab";
import { getMenuItems } from "@/lib/api";

export const metadata: Metadata = { title: "Splash lab", robots: { index: false, follow: false } };

// Dev-only workbench for the hero's liquid splash, fed by the same
// featured dishes as the homepage.
export default async function SplashLabPage({
  searchParams,
}: {
  searchParams: Promise<{ t?: string; seed?: string }>;
}) {
  if (process.env.NODE_ENV === "production") notFound();
  const { t, seed } = await searchParams;

  const items = await getMenuItems();
  const seen = new Set<string>();
  const slides: SplashLabSlide[] = items.flatMap((item) => {
    if (!item.is_featured || !item.is_available || !item.image_url || seen.has(item.image_url)) return [];
    seen.add(item.image_url);
    return [{ src: item.image_url, name: item.name }];
  });

  const initialTime = t !== undefined && Number.isFinite(Number(t)) ? Number(t) : null;
  return (
    <main className="mx-auto max-w-6xl px-4 py-10">
      <h1 className="mb-6 text-2xl">Splash lab</h1>
      <SplashLab slides={slides} initialTime={initialTime} initialSeed={Number(seed) || 1} />
    </main>
  );
}
