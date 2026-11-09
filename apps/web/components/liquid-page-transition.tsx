"use client";

import { useEffect, useRef } from "react";
import { useRouter } from "next/navigation";
import { gsap } from "@/lib/gsap";

// Give up waiting for a route that never commits (e.g. an error).
const MAX_WAIT_MS = 6000;
const SVG_NS = "http://www.w3.org/2000/svg";

type Bubble = {
  el: SVGCircleElement;
  x: number; // 0–1 across the screen
  radius: number; // px at full swell
  delay: number; // 0–0.4 of the cover phase
  rest: number; // 0–1 down the screen where it settles once covering
  wobble: number;
  phase: number;
};

type Wisp = { el: SVGCircleElement; x: number; radius: number; speed: number; offset: number; phase: number };

const ease = (t: number) => 1 - (1 - t) ** 3;
const easeIn = (t: number) => t * t;
const clamp01 = gsap.utils.clamp(0, 1);

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

function circle(parent: SVGElement, className: string): SVGCircleElement {
  const el = document.createElementNS(SVG_NS, "circle");
  el.setAttribute("class", className);
  parent.appendChild(el);
  return el;
}

/**
 * Navigating between pages boils up like a pot: bubbles rise from the
 * bottom, swell and merge (an SVG "goo" filter fuses them into one liquid)
 * until the screen is covered, with wisps of steam drifting up through
 * them. The route changes underneath — slow pages just keep it simmering —
 * then the bubbles carry on upwards and thin out like steam, uncovering
 * the new page from the bottom. Internal links are intercepted in the
 * capture phase, before next/link, which skips navigation once the event
 * is default-prevented. Links marked `data-no-page-transition` (the menu
 * filter) are left alone. Off with reduced motion.
 */
export function LiquidPageTransition() {
  const router = useRouter();
  const svgRef = useRef<SVGSVGElement>(null);
  const backRef = useRef<SVGGElement>(null);
  const frontRef = useRef<SVGGElement>(null);
  const steamRef = useRef<SVGGElement>(null);
  const backFillRef = useRef<SVGRectElement>(null);
  const frontFillRef = useRef<SVGRectElement>(null);

  useEffect(() => {
    const svg = svgRef.current;
    const back = backRef.current;
    const front = frontRef.current;
    const steam = steamRef.current;
    const backFill = backFillRef.current;
    const frontFill = frontFillRef.current;
    if (!svg || !back || !front || !steam || !backFill || !frontFill) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    // progress: 0 → 1 boils up to cover, 1 → 2 steams away.
    const state = { progress: 0 };
    let busy = false;
    let startedAt = 0;
    let bubbles: { back: Bubble[]; front: Bubble[] } = { back: [], front: [] };
    let wisps: Wisp[] = [];

    const makeBubbles = (group: SVGGElement, count: number): Bubble[] =>
      Array.from({ length: count }, (_, i) => ({
        el: circle(group, ""),
        // Spread evenly with jitter so the cover has no gaps.
        x: (i + 0.5) / count + (Math.random() - 0.5) / count,
        radius: 70 + Math.random() * 110,
        delay: Math.random() * 0.4,
        rest: Math.random() * 0.9,
        wobble: 6 + Math.random() * 14,
        phase: Math.random() * Math.PI * 2,
      }));

    const setUp = () => {
      back.querySelectorAll("circle").forEach((el) => el.remove());
      front.querySelectorAll("circle").forEach((el) => el.remove());
      steam.replaceChildren();
      const count = Math.max(10, Math.round(window.innerWidth / 70));
      bubbles = { back: makeBubbles(back, count), front: makeBubbles(front, count) };
      wisps = Array.from({ length: Math.round(count * 1.6) }, () => ({
        el: circle(steam, "fill-cream"),
        x: Math.random(),
        radius: 3 + Math.random() * 12,
        speed: 90 + Math.random() * 160,
        offset: Math.random(),
        phase: Math.random() * Math.PI * 2,
      }));
    };

    const drawLayer = (list: Bubble[], fill: SVGRectElement, p: number, time: number) => {
      const width = window.innerWidth;
      const height = window.innerHeight;
      const cover = clamp01(p);
      const drain = clamp01(p - 1);

      // A liquid body boils up behind the bubbles so the cover is solid by
      // the end, and on the way out drops away from the bottom up.
      const fillTop = height - easeIn(clamp01((cover - 0.35) / 0.65)) * (height + 60);
      const fillBottom = height + 60 - ease(drain) * (height + 200);
      fill.setAttribute("y", fillTop.toFixed(1));
      fill.setAttribute("height", Math.max(fillBottom - fillTop, 0).toFixed(1));

      for (const b of list) {
        const rise = ease(clamp01((cover - b.delay) / 0.6));
        // Steaming away: each bubble lifts off in turn and shrinks.
        const lift = easeIn(clamp01((drain - b.delay * 0.5) / 0.7));
        const restY = b.rest * height;
        const y = height + b.radius - rise * (height + b.radius - restY) - lift * (restY + height * 0.6 + b.radius);
        const r = b.radius * (0.35 + 0.65 * rise) * (1 - 0.75 * lift);
        const x = b.x * width + Math.sin(time * 1.6 + b.phase) * b.wobble;
        b.el.setAttribute("cx", x.toFixed(1));
        b.el.setAttribute("cy", y.toFixed(1));
        b.el.setAttribute("r", Math.max(r, 0).toFixed(1));
      }
    };

    const drawSteam = (p: number, time: number) => {
      const width = window.innerWidth;
      const height = window.innerHeight;
      // Strongest while simmering, gone at either end.
      const strength = Math.sin(Math.PI * clamp01(p / 2)) ** 0.6;
      steam.setAttribute("opacity", (0.55 * strength).toFixed(2));
      for (const w of wisps) {
        const travel = (time * w.speed + w.offset * (height + 80)) % (height + 80);
        const y = height + 40 - travel;
        const x = w.x * width + Math.sin(time * 2.2 + w.phase) * 18;
        // Wisps grow and fade as they rise.
        const along = travel / (height + 80);
        w.el.setAttribute("cx", x.toFixed(1));
        w.el.setAttribute("cy", y.toFixed(1));
        w.el.setAttribute("r", (w.radius * (0.6 + along)).toFixed(1));
        w.el.setAttribute("opacity", (1 - along).toFixed(2));
      }
    };

    const draw = () => {
      const time = (performance.now() - startedAt) / 1000;
      const p = state.progress;
      svg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
      // Overhang the sides so the goo blur doesn't round them off.
      backFill.setAttribute("width", String(window.innerWidth + 80));
      frontFill.setAttribute("width", String(window.innerWidth + 80));
      // The accent layer leads the boil and trails the steam.
      drawLayer(bubbles.back, backFill, p <= 1 ? Math.min(p + 0.12, 1) : Math.max(p - 0.12, 1), time);
      drawLayer(bubbles.front, frontFill, p, time);
      drawSteam(p, time);
    };

    const urlCommitted = (from: string) =>
      new Promise<void>((resolve) => {
        const giveUpAt = performance.now() + MAX_WAIT_MS;
        const check = () => {
          if (location.href !== from || performance.now() > giveUpAt) {
            // One more frame for the new page to paint under the cover.
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
      setUp();
      svg.style.visibility = "visible";
      gsap.ticker.add(draw);

      await gsap.to(state, { progress: 1, duration: 0.75, ease: "power1.inOut" });
      const from = location.href;
      router.push(url.pathname + url.search + url.hash);
      await urlCommitted(from);
      await gsap.to(state, { progress: 2, duration: 0.9, ease: "power1.inOut" });

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
    >
      <defs>
        {/* Blur then re-threshold the alpha: overlapping circles fuse into
            one blobby liquid. */}
        <filter id="liquid-goo" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur in="SourceGraphic" stdDeviation="14" />
          <feColorMatrix values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 24 -11" />
        </filter>
      </defs>
      <g ref={backRef} filter="url(#liquid-goo)" className="fill-accent">
        <rect ref={backFillRef} x="-40" />
      </g>
      <g ref={frontRef} filter="url(#liquid-goo)" className="fill-primary">
        <rect ref={frontFillRef} x="-40" />
      </g>
      <g ref={steamRef} />
    </svg>
  );
}
