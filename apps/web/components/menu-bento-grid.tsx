import type { CSSProperties } from "react";
import { MenuItemCard } from "@/components/menu-item-card";
import { bentoLayout } from "@/lib/menu-bento";
import type { MenuItem } from "@/lib/api";

type AreaVars = CSSProperties & { "--sm-area": string; "--lg-area": string };

// Phase 25 Decisions 7–10: one grid for /menu and the homepage's featured
// items. Below `sm` it's a plain list; from `sm` each tile is placed
// explicitly from the pattern, so visual order is DOM (`display_order`)
// order.
export function MenuBentoGrid({ items }: { items: MenuItem[] }) {
  const layout = bentoLayout(items.length);

  return (
    <div className="grid grid-cols-1 gap-6 sm:auto-rows-[8rem] sm:grid-cols-2 lg:grid-cols-4">
      {items.map((item, i) => {
        const style: AreaVars = { "--sm-area": layout[i].sm.area, "--lg-area": layout[i].lg.area };
        return (
          <div
            key={item.id}
            style={style}
            className="sm:[grid-area:var(--sm-area)] lg:[grid-area:var(--lg-area)]"
          >
            <MenuItemCard item={item} placement={layout[i]} />
          </div>
        );
      })}
    </div>
  );
}
