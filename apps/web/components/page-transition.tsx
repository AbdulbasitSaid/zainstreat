"use client";

import { useRef, type ReactNode } from "react";
import { usePathname } from "@/i18n/navigation";
import { gsap, useGSAP, ScrollTrigger } from "@/lib/gsap";

function Panel({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.from(ref.current, { opacity: 0, y: 8, duration: 0.3, ease: "outExpo" });
      });
      ScrollTrigger.refresh();
      return () => mm.revert();
    },
    { scope: ref },
  );

  return <div ref={ref}>{children}</div>;
}

export function PageTransition({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  return <Panel key={pathname}>{children}</Panel>;
}
