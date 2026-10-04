"use client";

import { useRef, type ReactNode } from "react";
import { gsap, useGSAP } from "@/lib/gsap";

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
        gsap.from(targets, {
          opacity: 0,
          y: 16,
          duration: 0.5,
          ease: "outExpo",
          stagger: stagger ? 0.08 : 0,
          scrollTrigger: { trigger: ref.current, start: "top 80%", once: true },
        });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [stagger] },
  );

  return <div ref={ref}>{children}</div>;
}
