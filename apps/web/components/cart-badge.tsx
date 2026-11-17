"use client";

import { useEffect, useRef } from "react";
import { useCart } from "@/lib/cart-context";
import { CART_ADDED_EVENT, CART_LANDED_EVENT, type CartAddedDetail } from "@/lib/cart-feedback";
import { gsap } from "@/lib/gsap";
import { usePreview } from "@/lib/preview-context";
import { useReducedMotion } from "@/lib/use-reduced-motion";

// Longest we wait for the flying photo before bouncing anyway.
const LANDING_TIMEOUT_MS = 900;

export function CartBadge() {
  const { itemCount } = useCart();
  const preview = usePreview();
  const reducedMotion = useReducedMotion();
  const badgeRef = useRef<HTMLSpanElement>(null);

  // Bounce only for a deliberate add (Phase 30 Decision 5): as the photo
  // lands, or straight away when nothing flies. Loading a saved cart or
  // editing it on /cart never fires the event.
  useEffect(() => {
    if (reducedMotion) return;
    const timers = new Set<number>();

    function bounce() {
      // The badge may only have mounted with this add (0 → 1).
      const badge = badgeRef.current;
      if (!badge) return;
      gsap.fromTo(badge, { scale: 1 }, { scale: 1.35, duration: 0.2, ease: "back.out(3)", yoyo: true, repeat: 1 });
    }

    function onAdded(event: Event) {
      const { flying } = (event as CustomEvent<CartAddedDetail>).detail;
      if (!flying) {
        requestAnimationFrame(() => requestAnimationFrame(bounce));
        return;
      }
      const timer = window.setTimeout(() => {
        timers.delete(timer);
        window.removeEventListener(CART_LANDED_EVENT, onLanded);
        bounce();
      }, LANDING_TIMEOUT_MS);
      timers.add(timer);
      function onLanded() {
        window.clearTimeout(timer);
        timers.delete(timer);
        bounce();
      }
      window.addEventListener(CART_LANDED_EVENT, onLanded, { once: true });
    }

    window.addEventListener(CART_ADDED_EVENT, onAdded);
    return () => {
      window.removeEventListener(CART_ADDED_EVENT, onAdded);
      timers.forEach((timer) => window.clearTimeout(timer));
    };
  }, [reducedMotion]);

  // The preview shares the admin's browser storage; its cart is never shown.
  if (preview || itemCount === 0) return null;

  return (
    <span
      ref={badgeRef}
      className="absolute -top-1 -right-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-primary px-1 text-[0.65rem] font-bold text-white"
    >
      {itemCount}
    </span>
  );
}
