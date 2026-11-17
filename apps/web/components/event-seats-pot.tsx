"use client";

import { useRef } from "react";
import { gsap, useGSAP } from "@/lib/gsap";

// Inside the pot, in viewBox units: the soup line runs from BOTTOM (empty)
// up to TOP (full).
const TOP = 52;
const BOTTOM = 150;

// A wave twice the pot's width, so sliding it one width left loops.
const WAVE = "M0 8 Q20 0 40 8 T80 8 T120 8 T160 8 T200 8 T240 8 T280 8 T320 8 V200 H0 Z";

/**
 * The seats meter as a cooking pot filling with soup (Phase 31 Decision
 * 16): the level is the share of seats taken. It fills up when scrolled
 * into view and the surface ripples; under reduced motion it's drawn at
 * its level and still. The label carries the numbers for screen readers.
 */
export function EventSeatsPot({ taken, capacity, label }: { taken: number; capacity: number; label: string }) {
  const ref = useRef<SVGSVGElement>(null);
  const share = capacity > 0 ? Math.min(1, taken / capacity) : 0;
  const level = BOTTOM - (BOTTOM - TOP) * share;

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.from("[data-soup]", {
          y: BOTTOM - level,
          duration: 1.6,
          ease: "outExpo",
          scrollTrigger: { trigger: ref.current, start: "top 85%", once: true },
        });
        gsap.to("[data-wave]", { x: -160, duration: 3.2, ease: "none", repeat: -1 });
        gsap.to("[data-bubble]", {
          y: -30,
          opacity: 0,
          duration: 2.4,
          ease: "sine.in",
          repeat: -1,
          stagger: { each: 0.8, repeat: -1 },
        });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [level] },
  );

  return (
    <figure className="m-0 flex flex-col items-center gap-2">
      <svg ref={ref} viewBox="0 0 200 170" role="img" aria-label={label} className="w-full max-w-[220px]">
        <defs>
          <clipPath id="pot-inside">
            <path d="M28 46 H172 V138 Q172 160 150 160 H50 Q28 160 28 138 Z" />
          </clipPath>
        </defs>
        {/* Handles */}
        <path d="M28 70 Q8 70 8 86 Q8 100 28 100" className="fill-none stroke-text" strokeWidth={6} strokeLinecap="round" />
        <path d="M172 70 Q192 70 192 86 Q192 100 172 100" className="fill-none stroke-text" strokeWidth={6} strokeLinecap="round" />
        <g clipPath="url(#pot-inside)">
          <rect x="0" y="0" width="200" height="200" className="fill-cream" />
          <g data-soup transform={`translate(0 ${level - 8})`}>
            <path data-wave d={WAVE} className="fill-primary" />
          </g>
          {share > 0 && (
            <g className="fill-accent-light/80">
              <circle data-bubble cx="70" cy={level + 22} r="4" />
              <circle data-bubble cx="112" cy={level + 30} r="3" />
              <circle data-bubble cx="140" cy={level + 18} r="5" />
            </g>
          )}
        </g>
        <path d="M28 46 H172 V138 Q172 160 150 160 H50 Q28 160 28 138 Z" className="fill-none stroke-text" strokeWidth={6} strokeLinejoin="round" />
        {/* Rim */}
        <rect x="18" y="38" width="164" height="12" rx="6" className="fill-text" />
      </svg>
      <figcaption aria-hidden="true" className="text-sm font-semibold text-text-muted">
        {label}
      </figcaption>
    </figure>
  );
}
