import type { MenuItem } from "@/lib/api";

export type BentoSize = "regular" | "tall" | "wide" | "feature";

// Tuned against real menu content; see Phase 25 requirement.md Decision 7.
const THRESHOLDS = { feature: 4, wide: 2.5, tallOptions: 3 } as const;
const CHARS_PER_POINT = 60;

export function bentoScore(item: MenuItem): number {
  const descriptionLength = item.description?.trim().length ?? 0;
  return (
    descriptionLength / CHARS_PER_POINT +
    Math.max(0, item.price_options.length - 1) +
    (item.is_featured ? 2 : 0)
  );
}

export function bentoSize(item: MenuItem): BentoSize {
  const score = bentoScore(item);
  if (score >= THRESHOLDS.feature) return "feature";
  if (score >= THRESHOLDS.wide) return "wide";
  if (item.price_options.length >= THRESHOLDS.tallOptions) return "tall";
  return "regular";
}

// Written out in full so Tailwind's scanner finds every class.
export const BENTO_SPAN: Record<BentoSize, string> = {
  regular: "",
  tall: "sm:row-span-2",
  wide: "sm:col-span-2",
  feature: "sm:col-span-2 sm:row-span-2",
};

const SM_COLUMN_SPAN: Record<BentoSize, number> = { regular: 1, tall: 1, wide: 2, feature: 2 };

// A category whose cards add up to fewer columns than the widest grid gets
// fewer columns, so it never leaves a mostly empty row (Decision 8).
const COLUMNS_FOR_SPAN = {
  1: "grid-cols-1 sm:grid-cols-2 sm:max-w-[50%]",
  2: "grid-cols-1 sm:grid-cols-2",
  3: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
  many: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4",
} as const;

export function bentoColumns(sizes: readonly BentoSize[]): string {
  const totalSpan = sizes.reduce((sum, size) => sum + SM_COLUMN_SPAN[size], 0);
  if (totalSpan <= 1) return COLUMNS_FOR_SPAN[1];
  if (totalSpan === 2) return COLUMNS_FOR_SPAN[2];
  if (totalSpan === 3) return COLUMNS_FOR_SPAN[3];
  return COLUMNS_FOR_SPAN.many;
}
