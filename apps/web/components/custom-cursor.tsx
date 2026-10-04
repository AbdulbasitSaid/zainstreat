"use client";

import { useEffect, useRef, useSyncExternalStore } from "react";
import { gsap } from "@/lib/gsap";
import { useReducedMotion } from "@/lib/use-reduced-motion";

const FINE_POINTER_QUERY = "(pointer: fine)";

function subscribeToPointerQuery(callback: () => void) {
  const query = window.matchMedia(FINE_POINTER_QUERY);
  query.addEventListener("change", callback);
  return () => query.removeEventListener("change", callback);
}

function getHasFinePointerSnapshot() {
  return window.matchMedia(FINE_POINTER_QUERY).matches;
}

const INTERACTIVE_SELECTOR = "a, button, [role='button'], input, textarea, select, label";

export function CustomCursor() {
  const shouldReduceMotion = useReducedMotion();
  const hasFinePointer = useSyncExternalStore(subscribeToPointerQuery, getHasFinePointerSnapshot, () => false);
  const dotRef = useRef<HTMLDivElement>(null);
  const ringRef = useRef<HTMLDivElement>(null);
  const isActive = hasFinePointer && !shouldReduceMotion;

  useEffect(() => {
    if (!isActive) return;

    document.body.classList.add("custom-cursor-active");

    const setDotX = gsap.quickTo(dotRef.current, "x", { duration: 0.1, ease: "power3" });
    const setDotY = gsap.quickTo(dotRef.current, "y", { duration: 0.1, ease: "power3" });
    const setRingX = gsap.quickTo(ringRef.current, "x", { duration: 0.35, ease: "power3" });
    const setRingY = gsap.quickTo(ringRef.current, "y", { duration: 0.35, ease: "power3" });

    function handleMouseMove(event: MouseEvent) {
      setDotX(event.clientX);
      setDotY(event.clientY);
      setRingX(event.clientX);
      setRingY(event.clientY);
    }

    function handleMouseOver(event: MouseEvent) {
      if (event.target instanceof Element && event.target.closest(INTERACTIVE_SELECTOR)) {
        gsap.to(ringRef.current, { scale: 1.6, duration: 0.2 });
      }
    }

    function handleMouseOut(event: MouseEvent) {
      if (event.target instanceof Element && event.target.closest(INTERACTIVE_SELECTOR)) {
        gsap.to(ringRef.current, { scale: 1, duration: 0.2 });
      }
    }

    window.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseover", handleMouseOver);
    document.addEventListener("mouseout", handleMouseOut);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseover", handleMouseOver);
      document.removeEventListener("mouseout", handleMouseOut);
      document.body.classList.remove("custom-cursor-active");
    };
  }, [isActive]);

  if (!isActive) return null;

  return (
    <>
      <div
        ref={dotRef}
        aria-hidden="true"
        className="pointer-events-none fixed top-0 left-0 z-[100] h-2 w-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-primary"
      />
      <div
        ref={ringRef}
        aria-hidden="true"
        className="pointer-events-none fixed top-0 left-0 z-[100] h-8 w-8 -translate-x-1/2 -translate-y-1/2 rounded-full border border-primary"
      />
    </>
  );
}
