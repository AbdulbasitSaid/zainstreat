"use client";

import { useEffect, useRef } from "react";
import { useRouter } from "next/navigation";
import { gsap } from "@/lib/gsap";

// Wave shape of the liquid's leading edges.
const WAVE_STEP_PX = 24;
const WAVE_AMPLITUDE_PX = 26;
const PAD_PX = WAVE_AMPLITUDE_PX * 2;
// Give up waiting for a route that never commits (e.g. an error).
const MAX_WAIT_MS = 6000;

type Edges = { top: number; bottom: number };

// A full-width band of liquid between two wavy edges, with the waves at
// their strongest mid-motion.
function bandPath(width: number, edges: Edges, time: number, swell: number): string {
  const wave = (x: number, phase: number) =>
    WAVE_AMPLITUDE_PX *
    swell *
    (0.6 * Math.sin(x / 140 + time * 3.1 + phase) + 0.4 * Math.sin(x / 63 - time * 4.3 + phase * 2));
  let d = "";
  for (let x = 0; x <= width + WAVE_STEP_PX; x += WAVE_STEP_PX) {
    d += `${d ? "L" : "M"}${x},${(edges.top + wave(x, 0)).toFixed(1)}`;
  }
  for (let x = width + WAVE_STEP_PX; x >= 0; x -= WAVE_STEP_PX) {
    d += `L${x},${(edges.bottom + wave(x, 1.7)).toFixed(1)}`;
  }
  return `${d}Z`;
}

function isInternalNavigation(event: MouseEvent): URL | null {
  if (event.defaultPrevented || event.button !== 0) return null;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return null;
  const anchor = event.target instanceof Element ? event.target.closest("a") : null;
  if (!anchor || anchor.hasAttribute("download") || anchor.hasAttribute("data-no-page-transition")) return null;
  if (anchor.target && anchor.target !== "_self") return null;
  const url = new URL(anchor.href, location.href);
  if (url.origin !== location.origin) return null;
  // Same page (or only a #hash away): let the browser handle it.
  if (url.pathname === location.pathname && url.search === location.search) return null;
  return url;
}

/**
 * Navigating between pages pours liquid over the screen: a wavy wall of
 * brand colour rises to cover the page, the route changes underneath (as
 * long as it takes — slow pages keep the liquid sloshing instead of looking
 * frozen), then it drains away upwards to reveal the new page. Internal
 * links are intercepted in the capture phase, before next/link, which skips
 * navigation once the event is default-prevented. Links marked
 * `data-no-page-transition` (the menu filter) are left alone. Off with
 * reduced motion.
 */
export function LiquidPageTransition() {
  const router = useRouter();
  const svgRef = useRef<SVGSVGElement>(null);
  const frontRef = useRef<SVGPathElement>(null);
  const backRef = useRef<SVGPathElement>(null);

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    // progress: 0 → 1 covers, 1 → 2 drains. The back layer leads slightly.
    const state = { progress: 0 };
    let busy = false;
    let startedAt = 0;

    const draw = () => {
      const width = window.innerWidth;
      const height = window.innerHeight;
      const time = (performance.now() - startedAt) / 1000;
      svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
      const edgesAt = (p: number): Edges => {
        const cover = gsap.utils.clamp(0, 1, p);
        const drain = gsap.utils.clamp(0, 1, p - 1);
        const span = height + 2 * PAD_PX;
        return { top: height + PAD_PX - cover * span, bottom: height + PAD_PX - drain * span };
      };
      // Waves calm down once the screen is fully covered or uncovered.
      const swell = (p: number) => 0.35 + 0.65 * Math.abs(Math.sin(Math.PI * gsap.utils.clamp(0, 2, p)));
      // The accent layer leads the cover and trails the drain.
      const p = state.progress;
      const backP = p <= 1 ? Math.min(p + 0.1, 1) : Math.max(p - 0.1, 1);
      backRef.current?.setAttribute("d", bandPath(width, edgesAt(backP), time, swell(backP)));
      frontRef.current?.setAttribute("d", bandPath(width, edgesAt(p), time, swell(p)));
    };

    const urlCommitted = (from: string) =>
      new Promise<void>((resolve) => {
        const giveUpAt = performance.now() + MAX_WAIT_MS;
        const check = () => {
          if (location.href !== from || performance.now() > giveUpAt) {
            // One more frame for the new page to paint under the liquid.
            requestAnimationFrame(() => resolve());
          } else {
            requestAnimationFrame(check);
          }
        };
        check();
      });

    const run = async (url: URL) => {
      busy = true;
      startedAt = performance.now();
      state.progress = 0;
      svg.style.visibility = "visible";
      gsap.ticker.add(draw);

      await gsap.to(state, { progress: 1, duration: 0.55, ease: "power2.inOut" });
      const from = location.href;
      router.push(url.pathname + url.search + url.hash);
      await urlCommitted(from);
      await gsap.to(state, { progress: 2, duration: 0.7, ease: "power2.inOut" });

      gsap.ticker.remove(draw);
      svg.style.visibility = "hidden";
      busy = false;
    };

    const handleClick = (event: MouseEvent) => {
      const url = isInternalNavigation(event);
      if (!url) return;
      event.preventDefault();
      if (!busy) void run(url);
    };

    document.addEventListener("click", handleClick, true);
    return () => {
      document.removeEventListener("click", handleClick, true);
      gsap.ticker.remove(draw);
      gsap.killTweensOf(state);
    };
  }, [router]);

  return (
    <svg
      ref={svgRef}
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-[90] h-full w-full"
      style={{ visibility: "hidden" }}
      preserveAspectRatio="none"
    >
      <path ref={backRef} className="fill-accent" />
      <path ref={frontRef} className="fill-primary" />
    </svg>
  );
}
