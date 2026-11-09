"use client";

import { useEffect, useRef } from "react";
import {
  LiquidOutline,
  cornerRadius,
  frameLoop,
  prefersLiquidMotion,
  reducedMotion,
  subscribePointer,
  toLocal,
} from "@/lib/liquid";

// How far beyond the button the pointer starts to attract its outline, and
// the SVG's bleed around the button (must exceed the largest pull).
const ATTRACT_ZONE_PX = 50;
const BLEED_PX = 40;
// Spread of the pull along the outline: tight, so only the stretch of edge
// right under/near the pointer bulges.
const FALLOFF_PX = 16;
const MAX_PULL_PX = 16;

const SVG_NS = "http://www.w3.org/2000/svg";

type Drop = { el: SVGCircleElement; x: number; y: number; vx: number; vy: number; r: number; life: number; age: number };

/**
 * Draws a button's surface (fill and/or border) as a liquid outline. Render
 * it inside a `relative isolate group/liquid` button: once running it sets
 * `data-liquid="surface"` on the button, which hides the real background,
 * border and shadow (globals.css), so until hydration — and on touch or
 * with reduced motion — the plain CSS button shows instead.
 *
 * Near the pointer, the bit of edge closest to it stretches towards it and
 * sloshes back. A click splashes: the edge bounces and droplets fly out
 * (touch gets the droplets too).
 */
export function LiquidSurface({
  pathClassName,
  splashClassName,
  strokeWidth = 0,
  className = "",
}: {
  pathClassName: string;
  splashClassName: string;
  strokeWidth?: number;
  className?: string;
}) {
  const svgRef = useRef<SVGSVGElement>(null);
  const pathRef = useRef<SVGPathElement>(null);

  useEffect(() => {
    const svg = svgRef.current;
    const path = pathRef.current;
    const host = svg?.parentElement;
    if (!svg || !path || !host || reducedMotion()) return;
    const liquid = prefersLiquidMotion();

    const outline = new LiquidOutline();
    let pointer: { x: number; y: number } | null = null;
    let drops: Drop[] = [];

    const loop = frameLoop((time) => {
      let moving = drops.length > 0;
      if (liquid) {
        moving =
          outline.step((pt, i) => {
            if (!pointer) return 0;
            const dx = pointer.x - pt.x;
            const dy = pointer.y - pt.y;
            // Positive when the pointer is outside the button on this
            // point's side, so the edge reaches out to meet it.
            const along = dx * pt.nx + dy * pt.ny;
            const pull = Math.min(Math.max(along + 6, 3), MAX_PULL_PX);
            const target = pull * Math.exp(-(dx * dx + dy * dy) / (2 * FALLOFF_PX * FALLOFF_PX));
            // A faint ripple while active so it never sits perfectly still.
            return target + Math.sin(time * 2.4 + i * 0.6) * 0.5;
          }) || moving;
        path.setAttribute("d", outline.path(BLEED_PX));
      }
      drops = drops.filter((drop) => {
        drop.age += 1;
        drop.vy += 0.32;
        drop.x += drop.vx;
        drop.y += drop.vy;
        const left = 1 - drop.age / drop.life;
        if (left <= 0) {
          drop.el.remove();
          return false;
        }
        drop.el.setAttribute("cx", (BLEED_PX + drop.x).toFixed(1));
        drop.el.setAttribute("cy", (BLEED_PX + drop.y).toFixed(1));
        drop.el.setAttribute("r", (drop.r * (0.4 + 0.6 * left)).toFixed(2));
        return true;
      });
      return moving;
    });

    function measure() {
      if (!svg || !host || !path) return;
      // Layout size, unaffected by the button's own hover scale. A stroke
      // sits inside the box, like a CSS border.
      const width = host.offsetWidth;
      const height = host.offsetHeight;
      outline.setShape(width, height, cornerRadius(host, width, height), { inset: strokeWidth / 2 });
      svg.setAttribute("viewBox", `0 0 ${width + 2 * BLEED_PX} ${height + 2 * BLEED_PX}`);
      svg.style.width = `${width + 2 * BLEED_PX}px`;
      svg.style.height = `${height + 2 * BLEED_PX}px`;
      if (liquid) path.setAttribute("d", outline.path(BLEED_PX));
    }

    function splash(event: MouseEvent) {
      if (!svg || !host) return;
      // Keyboard activation has no position: splash from the centre.
      const origin =
        event.detail === 0
          ? { x: host.offsetWidth / 2, y: host.offsetHeight / 2 }
          : toLocal(host, event.clientX, event.clientY);
      if (liquid) outline.kick(origin.x, origin.y, 7, 14);
      for (let i = 0; i < 12; i++) {
        // Mostly upwards and outwards, like a drop hitting a surface.
        const angle = -Math.PI / 2 + (Math.random() - 0.5) * Math.PI * 1.5;
        const speed = 2.5 + Math.random() * 4;
        const el = document.createElementNS(SVG_NS, "circle");
        el.setAttribute("class", splashClassName);
        svg.appendChild(el);
        drops.push({
          el,
          x: origin.x,
          y: origin.y,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          r: 2.4 + Math.random() * 3.2,
          life: 26 + Math.random() * 16,
          age: 0,
        });
      }
      loop.wake();
    }

    function handlePointer(clientX: number, clientY: number) {
      if (!host) return;
      const local = Number.isNaN(clientX) ? null : toLocal(host, clientX, clientY);
      const near =
        local !== null &&
        !host.matches(":disabled") &&
        local.x > -ATTRACT_ZONE_PX &&
        local.y > -ATTRACT_ZONE_PX &&
        local.x < host.offsetWidth + ATTRACT_ZONE_PX &&
        local.y < host.offsetHeight + ATTRACT_ZONE_PX;
      if (!near && !pointer) return;
      pointer = near ? local : null;
      loop.wake();
    }

    measure();
    if (liquid) host.dataset.liquid = "surface";
    const resizeObserver = new ResizeObserver(measure);
    resizeObserver.observe(host);
    const unsubscribe = liquid ? subscribePointer(handlePointer) : () => {};
    host.addEventListener("click", splash);

    return () => {
      loop.stop();
      resizeObserver.disconnect();
      unsubscribe();
      host.removeEventListener("click", splash);
      delete host.dataset.liquid;
      drops.forEach((drop) => drop.el.remove());
    };
  }, [strokeWidth, splashClassName]);

  return (
    <svg
      ref={svgRef}
      aria-hidden="true"
      className={`pointer-events-none absolute -z-10 overflow-visible ${className}`.trim()}
      style={{ left: -BLEED_PX, top: -BLEED_PX }}
    >
      <path ref={pathRef} className={pathClassName} strokeWidth={strokeWidth} />
    </svg>
  );
}
