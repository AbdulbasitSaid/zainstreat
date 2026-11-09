"use client";

import { useEffect, useRef, type ReactNode } from "react";
import { LiquidOutline, cornerRadius, frameLoop, prefersLiquidMotion, subscribePointer, toLocal } from "@/lib/liquid";

// Cards are the jelly counterpart of the buttons' liquid: rather than
// reaching out to the pointer, the edge near it is drawn *in* (a wide,
// gentle dent), and crossing an edge on the way in or out sends a ripple
// round the whole card.
const DENT_PX = 9;
const FALLOFF_PX = 42;
const CROSSING_KICK = -5;

/**
 * A card whose outline behaves like jelly. `className` styles the card
 * itself (it's clipped to the liquid outline, so photos and backgrounds
 * follow the dents); `wrapperClassName` sizes the outer box, which carries
 * the shadow — a box-shadow would be clipped away, so while the effect runs
 * `data-liquid="clip"` swaps the card's own border/shadow for a drop-shadow
 * on the wrapper and an optional `strokeClassName` outline (globals.css).
 * Touch and reduced-motion visitors get the plain card.
 */
export function LiquidCard({
  children,
  className = "",
  wrapperClassName = "",
  strokeClassName,
}: {
  children: ReactNode;
  className?: string;
  wrapperClassName?: string;
  strokeClassName?: string;
}) {
  const cardRef = useRef<HTMLDivElement>(null);
  const strokeRef = useRef<SVGPathElement>(null);

  useEffect(() => {
    const card = cardRef.current;
    if (!card || !prefersLiquidMotion()) return;

    const outline = new LiquidOutline();
    let pointer: { x: number; y: number } | null = null;

    const render = () => {
      const d = outline.path();
      card.style.clipPath = `path("${d}")`;
      strokeRef.current?.setAttribute("d", d);
    };

    const loop = frameLoop((time) => {
      const moving = outline.step((pt, i) => {
        if (!pointer) return 0;
        const d2 = (pointer.x - pt.x) ** 2 + (pointer.y - pt.y) ** 2;
        return -DENT_PX * Math.exp(-d2 / (2 * FALLOFF_PX * FALLOFF_PX)) + Math.sin(time * 1.8 + i * 0.25) * 0.4;
      });
      render();
      return moving;
    });

    const measure = () => {
      const width = card.offsetWidth;
      const height = card.offsetHeight;
      outline.setShape(width, height, cornerRadius(card, width, height), { spacing: 7 });
      strokeRef.current?.ownerSVGElement?.setAttribute("viewBox", `0 0 ${width} ${height}`);
      render();
    };

    const handlePointer = (clientX: number, clientY: number) => {
      const local = Number.isNaN(clientX) ? null : toLocal(card, clientX, clientY);
      const inside =
        local !== null && local.x >= 0 && local.y >= 0 && local.x <= card.offsetWidth && local.y <= card.offsetHeight;
      if (!inside && !pointer) return;
      // Entering or leaving: the edge it crossed dips and a ripple runs off.
      if (inside !== (pointer !== null)) {
        const at = local ?? pointer;
        if (at) outline.kick(at.x, at.y, CROSSING_KICK, 18);
      }
      pointer = inside ? local : null;
      loop.wake();
    };

    measure();
    card.dataset.liquid = "clip";
    const resizeObserver = new ResizeObserver(measure);
    resizeObserver.observe(card);
    const unsubscribe = subscribePointer(handlePointer);

    return () => {
      loop.stop();
      resizeObserver.disconnect();
      unsubscribe();
      delete card.dataset.liquid;
      card.style.clipPath = "";
    };
  }, []);

  return (
    <div className={`liquid-card ${wrapperClassName}`.trim()}>
      <div ref={cardRef} className={`relative ${className}`.trim()}>
        {children}
        {strokeClassName && (
          <svg aria-hidden="true" className="pointer-events-none absolute inset-0 hidden h-full w-full overflow-visible in-data-[liquid=clip]:block">
            <path ref={strokeRef} className={`fill-none ${strokeClassName}`} strokeWidth={2} />
          </svg>
        )}
      </div>
    </div>
  );
}
