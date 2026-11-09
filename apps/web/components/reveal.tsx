"use client";

import { useRef, type ReactNode } from "react";
import { gsap, useGSAP } from "@/lib/gsap";

/**
 * Scroll-in reveal with a liquid feel: content rises while squashed and
 * wobbles to rest (elastic scale) like a settling drop. The outer wrapper
 * carries `data-liquid-scroll`, which LiquidScroll leans/stretches with the
 * scroll velocity — kept on a separate element so the two transforms don't
 * fight.
 */
export function Reveal({
  children,
  stagger = false,
}: {
  children: ReactNode;
  stagger?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        const targets = stagger ? gsap.utils.toArray<HTMLElement>(":scope > *", ref.current) : ref.current;
        const scrollTrigger = { trigger: ref.current, start: "top 85%", once: true };
        const each = stagger ? 0.08 : 0;
        gsap.from(targets, {
          opacity: 0,
          y: 48,
          duration: 0.7,
          ease: "outExpo",
          stagger: each,
          scrollTrigger,
        });
        gsap.from(targets, {
          scaleY: 1.08,
          scaleX: 0.96,
          transformOrigin: "50% 100%",
          duration: 1.2,
          ease: "elastic.out(1, 0.5)",
          stagger: each,
          scrollTrigger,
        });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [stagger] },
  );

  return (
    <div data-liquid-scroll>
      <div ref={ref}>{children}</div>
    </div>
  );
}
