"use client";

import { useEffect } from "react";
import { gsap, ScrollTrigger } from "@/lib/gsap";

// Degrees of lean per px/s of scroll velocity, and the caps.
const SKEW_PER_VELOCITY = 1 / 1100;
const MAX_SKEW_DEG = 2.2;
const MAX_STRETCH = 0.035;
// How long after the last scroll update the content wobbles back.
const SETTLE_DELAY_S = 0.08;

/**
 * Makes scrolling feel liquid: while the page moves, every
 * `[data-liquid-scroll]` block (each Reveal) leans against the motion and
 * stretches slightly, then wobbles back to rest once scrolling stops. It
 * drives two CSS variables on <html>, which globals.css applies, so one
 * tween serves every block. Off with reduced motion.
 */
export function LiquidScroll() {
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const root = document.documentElement;
    let settle: gsap.core.Tween | null = null;

    const trigger = ScrollTrigger.create({
      start: 0,
      end: "max",
      onUpdate(self) {
        const velocity = self.getVelocity();
        const skew = gsap.utils.clamp(-MAX_SKEW_DEG, MAX_SKEW_DEG, -velocity * SKEW_PER_VELOCITY);
        const stretch = 1 + Math.min(Math.abs(velocity) / 40000, MAX_STRETCH);
        gsap.to(root, {
          "--liquid-skew": `${skew}deg`,
          "--liquid-stretch": stretch,
          duration: 0.25,
          ease: "power2.out",
          overwrite: true,
        });
        settle?.kill();
        settle = gsap.delayedCall(SETTLE_DELAY_S, () => {
          gsap.to(root, {
            "--liquid-skew": "0deg",
            "--liquid-stretch": 1,
            duration: 1.1,
            ease: "elastic.out(1, 0.35)",
            overwrite: true,
          });
        });
      },
    });

    return () => {
      trigger.kill();
      settle?.kill();
      gsap.killTweensOf(root);
      root.style.removeProperty("--liquid-skew");
      root.style.removeProperty("--liquid-stretch");
    };
  }, []);

  return null;
}
