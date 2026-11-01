import { MenuItemCard } from "@/components/menu-item-card";
import { BENTO_SPAN, bentoColumns, bentoSize } from "@/lib/menu-bento";
import type { MenuItem } from "@/lib/api";

// Phase 25 Decisions 7–10: one grid for /menu and the homepage's featured
// items. Dense packing fills the holes big cards leave; DOM order (and so
// reading and tab order) stays `display_order`.
export function MenuBentoGrid({ items }: { items: MenuItem[] }) {
  const sized = items.map((item) => ({ item, size: bentoSize(item) }));

  return (
    <div
      className={`grid grid-flow-row-dense auto-rows-[minmax(11rem,auto)] gap-6 ${bentoColumns(
        sized.map(({ size }) => size),
      )}`}
    >
      {sized.map(({ item, size }) => (
        <div key={item.id} className={BENTO_SPAN[size]}>
          <MenuItemCard item={item} size={size} />
        </div>
      ))}
    </div>
  );
}
