// Checks lib/menu-bento.ts's layouts for every item count (Phase 25 plan
// 2.1): no overlapping cells, no holes except beside a lone tail item, and
// no two touching `lg` tiles with the same tint. Plain Node (its built-in
// type stripping loads the .ts module); run with `pnpm check:bento`.
import { bentoLayout } from "../lib/menu-bento.ts";

const MAX_COUNT = 17;
const COLUMNS = { sm: 2, lg: 4 };
const failures = [];

// "3 / 1 / span 2 / span 1" -> every [row, col] cell it covers.
function cellsOf(area) {
  const [row, col, rowSpan, colSpan] = area.split(" / ").map((part) => Number(part.replace("span ", "")));
  const cells = [];
  for (let r = row; r < row + rowSpan; r++) {
    for (let c = col; c < col + colSpan; c++) cells.push([r, c]);
  }
  return cells;
}

for (let count = 1; count <= MAX_COUNT; count++) {
  const layout = bentoLayout(count);
  for (const bp of ["sm", "lg"]) {
    const owner = new Map();
    layout.forEach((tile, i) => {
      for (const [r, c] of cellsOf(tile[bp].area)) {
        const key = `${r},${c}`;
        if (owner.has(key)) failures.push(`${count} items, ${bp}: items ${owner.get(key)} and ${i} overlap at ${key}`);
        owner.set(key, i);
      }
    });

    const rows = Math.max(...[...owner.keys()].map((key) => Number(key.split(",")[0])));
    const loneTail = count % (bp === "lg" ? 8 : 4) === 1;
    for (let r = 1; r <= rows; r++) {
      // A lone tail item sits in the last group's rows with empty space beside it.
      if (loneTail && r > rows - 2) continue;
      for (let c = 1; c <= COLUMNS[bp]; c++) {
        if (!owner.has(`${r},${c}`)) failures.push(`${count} items, ${bp}: hole at ${r},${c}`);
      }
    }

    if (bp === "lg") {
      for (const [key, i] of owner) {
        const [r, c] = key.split(",").map(Number);
        for (const next of [`${r + 1},${c}`, `${r},${c + 1}`]) {
          const j = owner.get(next);
          if (j !== undefined && j !== i && layout[i].tint === layout[j].tint) {
            failures.push(`${count} items, lg: touching items ${i} and ${j} are both "${layout[i].tint}"`);
          }
        }
      }
    }
  }
}

const unique = [...new Set(failures)];
if (unique.length > 0) {
  console.error(unique.join("\n"));
  process.exit(1);
}
console.log(`bento OK: counts 1–${MAX_COUNT}`);
