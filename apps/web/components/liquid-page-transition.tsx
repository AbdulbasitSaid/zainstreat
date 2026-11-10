"use client";

import { useEffect, useRef, useState } from "react";
import { useTranslations } from "next-intl";
import { BubbleLoader } from "@/components/bubble-loader";
import { gsap } from "@/lib/gsap";

// Stop waiting for a route that never commits (e.g. an error).
const MAX_WAIT_S = 8;
const BUBBLES_PER_SECOND = 14;
// Bubbles burst somewhere in this band, as a fraction of the screen height
// from the top.
const BURST_BAND = [0.06, 0.2] as const;
const SVG_NS = "http://www.w3.org/2000/svg";

type Bubble = {
  el: SVGGElement;
  x: number;
  y: number;
  radius: number;
  speed: number; // px/s upwards, increasing as it rises
  wobble: number;
  phase: number;
  age: number;
  burstAt: number; // y (px) where it pops
};

type Pop = { el: SVGGElement; ring: SVGCircleElement; drops: SVGCircleElement[]; radius: number; age: number };

const POP_DURATION_S = 0.32;

function isInternalNavigation(event: MouseEvent): boolean {
  if (event.defaultPrevented || event.button !== 0) return false;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return false;
  const anchor = event.target instanceof Element ? event.target.closest("a") : null;
  if (!anchor || anchor.hasAttribute("download") || anchor.hasAttribute("data-no-page-transition")) return false;
  if (anchor.target && anchor.target !== "_self") return false;
  const url = new URL(anchor.href, location.href);
  if (url.origin !== location.origin) return false;
  // Same page (or only a #hash away): nothing is loading.
  return !(url.pathname === location.pathname && url.search === location.search);
}

function svgEl<K extends "g" | "circle">(name: K, className = ""): SVGElementTagNameMap[K] {
  const el = document.createElementNS(SVG_NS, name);
  if (className) el.setAttribute("class", className);
  return el;
}

// A see-through bubble: faint tinted body, a rim, and a glint.
function makeBubbleElement(parent: SVGElement, radius: number): SVGGElement {
  const group = svgEl("g");
  const body = svgEl("circle", "fill-accent-light/20 stroke-primary/40");
  body.setAttribute("r", radius.toFixed(1));
  body.setAttribute("stroke-width", "1.2");
  const glint = svgEl("circle", "fill-white/80");
  glint.setAttribute("r", (radius * 0.25).toFixed(1));
  glint.setAttribute("cx", (-radius * 0.38).toFixed(1));
  glint.setAttribute("cy", (-radius * 0.38).toFixed(1));
  group.append(body, glint);
  parent.appendChild(group);
  return group;
}

// A burst: the rim snaps outwards and a few droplets fly off.
function makePop(parent: SVGElement, x: number, y: number, radius: number): Pop {
  const el = svgEl("g");
  el.setAttribute("transform", `translate(${x.toFixed(1)} ${y.toFixed(1)})`);
  const ring = svgEl("circle", "fill-none stroke-primary/50");
  ring.setAttribute("stroke-width", "1.2");
  const drops = Array.from({ length: 5 }, () => {
    const drop = svgEl("circle", "fill-primary/50");
    el.appendChild(drop);
    return drop;
  });
  el.appendChild(ring);
  parent.appendChild(el);
  return { el, ring, drops, radius, age: 0 };
}

/**
 * Page loading: an internal link click swaps the old page for a pot
 * simmering under the header (a cream screen with the bubble loader), with
 * small see-through bubbles rising from the bottom of the screen and
 * bursting near the top. Navigation itself is untouched; once the URL
 * changes the new page has rendered underneath, the screen fades away and
 * the last bubbles float up and pop. Links marked
 * `data-no-page-transition` (the menu filter, which has its own loader) are
 * skipped. Off with reduced motion.
 */
export function LiquidPageTransition() {
  const t = useTranslations("SiteHeader");
  const svgRef = useRef<SVGSVGElement>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    let bubbles: Bubble[] = [];
    let pops: Pop[] = [];
    let bubbling = false;
    let running = false;
    let waitedFor = 0;
    let spawnDebt = 0;
    let startHref = "";

    const spawn = (width: number, height: number) => {
      const radius = 2.5 + Math.random() ** 2 * 8;
      bubbles.push({
        el: makeBubbleElement(svg, radius),
        x: Math.random() * width,
        y: height + radius,
        radius,
        speed: 140 + Math.random() * 120 + (10 - radius) * 12,
        wobble: 4 + Math.random() * 10,
        phase: Math.random() * Math.PI * 2,
        age: 0,
        burstAt: height * (BURST_BAND[0] + Math.random() * (BURST_BAND[1] - BURST_BAND[0])),
      });
    };

    const tick = (_time: number, deltaMs: number) => {
      const dt = Math.min(deltaMs / 1000, 0.05);
      const width = window.innerWidth;
      const height = window.innerHeight;
      svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

      if (bubbling) {
        waitedFor += dt;
        if (location.href !== startHref || waitedFor >= MAX_WAIT_S) {
          bubbling = false;
          // Let the new page paint under the screen before fading it.
          requestAnimationFrame(() => setLoading(false));
        }
        spawnDebt += dt * BUBBLES_PER_SECOND;
        for (; spawnDebt >= 1; spawnDebt -= 1) spawn(width, height);
      }

      bubbles = bubbles.filter((b) => {
        b.age += dt;
        b.speed *= 1 + dt * 0.5;
        b.y -= b.speed * dt;
        const x = b.x + Math.sin(b.age * 3 + b.phase) * b.wobble;
        if (b.y <= b.burstAt) {
          b.el.remove();
          pops.push(makePop(svg, x, b.y, b.radius));
          return false;
        }
        b.el.setAttribute("transform", `translate(${x.toFixed(1)} ${b.y.toFixed(1)})`);
        b.el.setAttribute("opacity", Math.min(b.age / 0.2, 1).toFixed(2));
        return true;
      });

      pops = pops.filter((pop) => {
        pop.age += dt;
        const k = pop.age / POP_DURATION_S;
        if (k >= 1) {
          pop.el.remove();
          return false;
        }
        const out = 1 - (1 - k) ** 2;
        pop.ring.setAttribute("r", (pop.radius * (1 + out * 1.4)).toFixed(1));
        pop.ring.setAttribute("opacity", (1 - k).toFixed(2));
        pop.drops.forEach((drop, i) => {
          const angle = (i / pop.drops.length) * Math.PI * 2 - Math.PI / 2;
          const reach = pop.radius * (1.2 + out * 2.2);
          drop.setAttribute("cx", (Math.cos(angle) * reach).toFixed(1));
          drop.setAttribute("cy", (Math.sin(angle) * reach + k * k * 6).toFixed(1));
          drop.setAttribute("r", (Math.max(pop.radius * 0.18, 1) * (1 - k * 0.6)).toFixed(2));
          drop.setAttribute("opacity", (1 - k).toFixed(2));
        });
        return true;
      });

      if (!bubbling && bubbles.length === 0 && pops.length === 0) {
        gsap.ticker.remove(tick);
        running = false;
      }
    };

    const handleClick = (event: MouseEvent) => {
      if (!isInternalNavigation(event)) return;
      startHref = location.href;
      bubbling = true;
      waitedFor = 0;
      setLoading(true);
      if (!running) {
        running = true;
        gsap.ticker.add(tick);
      }
    };

    document.addEventListener("click", handleClick, true);
    return () => {
      document.removeEventListener("click", handleClick, true);
      gsap.ticker.remove(tick);
      bubbles.forEach((b) => b.el.remove());
      pops.forEach((pop) => pop.el.remove());
    };
  }, []);

  return (
    <>
      {/* Below the sticky header (z-50), so the nav stays put. Showing
          waits a beat so fast navigations don't flash it. */}
      <div
        aria-hidden={!loading}
        className={`fixed inset-0 z-40 flex items-center justify-center bg-cream transition-opacity duration-300 ease-out-expo ${
          loading ? "opacity-100 delay-150" : "pointer-events-none opacity-0"
        }`}
      >
        <BubbleLoader label={t("loadingPage")} className="scale-125" />
      </div>
      <svg ref={svgRef} aria-hidden="true" className="pointer-events-none fixed inset-0 z-[90] h-full w-full" />
    </>
  );
}
