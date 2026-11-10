"use client";

import { useEffect, useState, type MouseEvent } from "react";

// How far the page scrolls before the cue fades away.
const HIDE_AFTER_PX = 80;

/**
 * A "scroll for more" hint pinned to the bottom of a full-screen hero: a
 * mouse outline whose wheel drips downward. Clicking it glides to the
 * section `targetId` names; it fades out once the visitor starts scrolling.
 */
export function ScrollCue({ targetId, label, ariaLabel }: { targetId: string; label: string; ariaLabel: string }) {
  const [hidden, setHidden] = useState(false);

  useEffect(() => {
    function onScroll() {
      setHidden(window.scrollY > HIDE_AFTER_PX);
    }
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  function onClick(event: MouseEvent<HTMLAnchorElement>) {
    const target = document.getElementById(targetId);
    if (!target) return;
    event.preventDefault();
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    target.scrollIntoView({ behavior: reduceMotion ? "auto" : "smooth" });
  }

  return (
    <a
      href={`#${targetId}`}
      onClick={onClick}
      aria-label={ariaLabel}
      tabIndex={hidden ? -1 : undefined}
      className={`absolute bottom-4 left-1/2 flex min-h-11 min-w-11 -translate-x-1/2 flex-col items-center justify-center gap-2 p-2 text-white/85 transition-opacity duration-500 ease-out-expo hover:text-white md:bottom-6 ${
        hidden ? "pointer-events-none opacity-0" : "opacity-100"
      }`}
    >
      <span
        aria-hidden="true"
        className="relative h-8 w-5 rounded-full border-2 border-current md:h-10 md:w-6"
      >
        <span className="absolute left-1/2 top-1.5 h-2 w-1 -translate-x-1/2 animate-scroll-drip rounded-full bg-current md:top-2" />
      </span>
      <span aria-hidden="true" className="hidden text-[0.7rem] font-semibold uppercase tracking-[0.2em] md:block">
        {label}
      </span>
    </a>
  );
}
