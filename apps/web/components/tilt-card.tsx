"use client";

import { useRef, type ReactNode } from "react";
import { gsap, useGSAP } from "@/lib/gsap";

const MAX_TILT_DEG = 7;

// Tilts its child towards the pointer in 3D. Only on fine pointers with
// motion allowed, so touch and reduced-motion visitors get a flat card.
export function TiltCard({ children, className = "" }: { children: ReactNode; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(pointer: fine) and (prefers-reduced-motion: no-preference)", () => {
        const el = ref.current;
        if (!el) return;
        gsap.set(el, { transformPerspective: 900 });
        const toRotateX = gsap.quickTo(el, "rotationX", { duration: 0.4, ease: "power3" });
        const toRotateY = gsap.quickTo(el, "rotationY", { duration: 0.4, ease: "power3" });

        const handleMove = (event: PointerEvent) => {
          const rect = el.getBoundingClientRect();
          const x = (event.clientX - rect.left) / rect.width - 0.5;
          const y = (event.clientY - rect.top) / rect.height - 0.5;
          toRotateY(x * MAX_TILT_DEG * 2);
          toRotateX(-y * MAX_TILT_DEG * 2);
        };

        const handleLeave = () => {
          toRotateX(0);
          toRotateY(0);
        };

        el.addEventListener("pointermove", handleMove);
        el.addEventListener("pointerleave", handleLeave);
        return () => {
          el.removeEventListener("pointermove", handleMove);
          el.removeEventListener("pointerleave", handleLeave);
        };
      });
      return () => mm.revert();
    },
    { scope: ref },
  );

  return (
    <div ref={ref} className={`will-change-transform ${className}`.trim()}>
      {children}
    </div>
  );
}
