// Phase 25 Decisions 7–9 (revised 2026-11-09): tile shape, placement and
// tint come from an item's position in a hand-designed pattern, never from
// its content. Pure functions only, so `scripts/check-bento.ts` can test
// every count.

export type TileShape = "small" | "tall" | "wide";
export type TileTint = "brand" | "cream" | "soft" | "card" | "white";

export type TileSlot = { area: string; shape: TileShape; rows: number };
export type TilePlacement = { sm: TileSlot; lg: TileSlot; tint: TileTint };

type Pattern = { rows: string[]; tints?: TileTint[] };

// One letter per item in item order (a = the group's 1st item, …), `.` for
// an empty cell. Letters follow reading order (each tile's top-left cell),
// so DOM, visual and tab order agree. 8 is the reference layout; 1–7 are
// tails for a category's last, partial group, made to fill every column.
// Tints are written by hand so no two touching tiles match, across group
// boundaries too (`scripts/check-bento.ts` asserts it).
const LG_PATTERNS: Record<number, Pattern> = {
  8: {
    rows: ["a b b c", "a b b c", "a d e c", "f d e c", "f g h h", "f g h h"],
    tints: ["cream", "brand", "soft", "soft", "card", "card", "white", "cream"],
  },
  7: {
    rows: ["a b b c", "a b b c", "a d d c", "a d d c", "e f f g", "e f f g"],
    tints: ["soft", "brand", "card", "cream", "cream", "brand", "soft"],
  },
  6: {
    rows: ["a b b c", "a b b c", "d d e f", "d d e f"],
    tints: ["soft", "brand", "card", "cream", "soft", "cream"],
  },
  5: {
    rows: ["a b b c", "a b b c", "a d e c", "a d e c"],
    tints: ["cream", "brand", "soft", "soft", "card"],
  },
  4: {
    rows: ["a b b c", "a b b c", "a d d c", "a d d c"],
    tints: ["soft", "brand", "card", "cream"],
  },
  3: { rows: ["a b b c", "a b b c"], tints: ["cream", "brand", "soft"] },
  2: { rows: ["a a b b", "a a b b"], tints: ["cream", "brand"] },
  1: { rows: ["a . . .", "a . . ."], tints: ["cream"] },
};

const SM_PATTERNS: Record<number, Pattern> = {
  4: { rows: ["a b", "a b", "a c", "d c", "d c"] },
  3: { rows: ["a b", "a b", "c c", "c c"] },
  2: { rows: ["a b", "a b"] },
  1: { rows: ["a .", "a ."] },
};

type Box = { rowStart: number; colStart: number; rowSpan: number; colSpan: number };

export function parsePattern(rows: readonly string[]): Box[] {
  const cells = rows.map((row) => row.split(" "));
  const boxes = new Map<string, Box>();
  cells.forEach((row, r) =>
    row.forEach((letter, c) => {
      if (letter === ".") return;
      const box = boxes.get(letter);
      if (!box) {
        boxes.set(letter, { rowStart: r + 1, colStart: c + 1, rowSpan: 1, colSpan: 1 });
        return;
      }
      box.rowSpan = Math.max(box.rowSpan, r + 2 - box.rowStart);
      box.colSpan = Math.max(box.colSpan, c + 2 - box.colStart);
    }),
  );
  for (const [letter, box] of boxes) {
    for (let r = box.rowStart; r < box.rowStart + box.rowSpan; r++) {
      for (let c = box.colStart; c < box.colStart + box.colSpan; c++) {
        if (cells[r - 1][c - 1] !== letter) {
          throw new Error(`bento pattern: "${letter}" is not a rectangle in ${JSON.stringify(rows)}`);
        }
      }
    }
  }
  return [...boxes.keys()].sort().map((letter) => boxes.get(letter)!);
}

function shapeOf(box: Box): TileShape {
  if (box.colSpan === 2) return "wide";
  return box.rowSpan >= 3 ? "tall" : "small";
}

function layoutFor(
  count: number,
  patterns: Record<number, Pattern>,
  groupSize: number,
): { slot: TileSlot; tint: TileTint | undefined }[] {
  const out: { slot: TileSlot; tint: TileTint | undefined }[] = [];
  let rowOffset = 0;
  for (let start = 0; start < count; start += groupSize) {
    const pattern = patterns[Math.min(groupSize, count - start)];
    parsePattern(pattern.rows).forEach((box, i) => {
      out.push({
        slot: {
          area: `${box.rowStart + rowOffset} / ${box.colStart} / span ${box.rowSpan} / span ${box.colSpan}`,
          shape: shapeOf(box),
          rows: box.rowSpan,
        },
        tint: pattern.tints?.[i],
      });
    });
    rowOffset += pattern.rows.length;
  }
  return out;
}

export function bentoLayout(count: number): TilePlacement[] {
  const lg = layoutFor(count, LG_PATTERNS, 8);
  const sm = layoutFor(count, SM_PATTERNS, 4);
  return lg.map(({ slot, tint }, i) => ({ sm: sm[i].slot, lg: slot, tint: tint ?? "cream" }));
}
