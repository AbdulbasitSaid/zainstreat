"use client";

import { createContext, useContext, useRef, useState, useTransition, type ReactNode } from "react";
import { useRouter } from "@/i18n/navigation";
import { gsap, ScrollTrigger, useGSAP } from "@/lib/gsap";

type FilterHref = "/menu" | { pathname: "/menu"; query: { category: number } };

type MenuFilterState = {
  isPending: boolean;
  // The category the visitor just picked, shown as active before the
  // server responds; `undefined` means "All".
  optimisticCategoryId: number | undefined | null;
  select: (categoryId: number | undefined) => void;
};

const MenuFilterContext = createContext<MenuFilterState | null>(null);

export function useMenuFilter(): MenuFilterState {
  const state = useContext(MenuFilterContext);
  if (!state) throw new Error("useMenuFilter must be used inside <MenuFilterProvider>");
  return state;
}

/**
 * Filtering the menu without a full page transition: the pills route
 * through here so the results can melt while the new category loads and
 * drip back in once it lands (MenuFilterResults).
 */
export function MenuFilterProvider({ children }: { children: ReactNode }) {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();
  const [optimisticCategoryId, setOptimisticCategoryId] = useState<number | undefined | null>(null);

  const select = (categoryId: number | undefined) => {
    const href: FilterHref = categoryId === undefined ? "/menu" : { pathname: "/menu", query: { category: categoryId } };
    setOptimisticCategoryId(categoryId);
    startTransition(() => {
      router.push(href, { scroll: false });
    });
  };

  return (
    <MenuFilterContext.Provider
      value={{ isPending, optimisticCategoryId: isPending ? optimisticCategoryId : null, select }}
    >
      {children}
    </MenuFilterContext.Provider>
  );
}

/**
 * The filtered results. While a new category loads they melt (sag, blur,
 * fade); when `filterKey` changes the new cards drip in with a squash and
 * stretch, staggered like drops landing.
 */
export function MenuFilterResults({
  filterKey,
  loadingLabel,
  children,
}: {
  filterKey: string;
  loadingLabel: string;
  children: ReactNode;
}) {
  const { isPending } = useMenuFilter();
  const ref = useRef<HTMLDivElement>(null);
  const firstRender = useRef(true);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.to(ref.current, {
          opacity: isPending ? 0.35 : 1,
          filter: isPending ? "blur(6px)" : "blur(0px)",
          scaleY: isPending ? 0.97 : 1,
          y: isPending ? 14 : 0,
          transformOrigin: "50% 0%",
          duration: isPending ? 0.35 : 0.5,
          ease: isPending ? "power2.in" : "outExpo",
          overwrite: "auto",
        });
      });
      return () => mm.revert();
    },
    { dependencies: [isPending] },
  );

  useGSAP(
    () => {
      // The first paint is the page transition's job.
      if (firstRender.current) {
        firstRender.current = false;
        return;
      }
      // The new sections' Reveal triggers were created without a page
      // change, so nothing else refreshes their positions.
      ScrollTrigger.refresh();
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        const cards = gsap.utils.toArray<HTMLElement>(".liquid-card", ref.current);
        gsap.from(cards, {
          opacity: 0,
          y: 36,
          duration: 0.6,
          ease: "outExpo",
          stagger: 0.035,
        });
        gsap.from(cards, {
          scaleY: 1.18,
          scaleX: 0.88,
          transformOrigin: "50% 0%",
          duration: 1.1,
          ease: "elastic.out(1, 0.45)",
          stagger: 0.035,
          clearProps: "transform,opacity",
        });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [filterKey] },
  );

  return (
    <div aria-busy={isPending}>
      {/* Pinned under the header while the results load, so it's in view
          however far down the menu the visitor is. */}
      <div aria-live="polite" className="pointer-events-none sticky top-24 z-20 flex h-0 justify-center">
        {isPending && <BubbleLoader label={loadingLabel} />}
      </div>
      <div ref={ref}>{children}</div>
    </div>
  );
}

// A small pot of rising, popping bubbles (globals.css `.bubble-loader`).
// Fades in after a beat so quick loads don't flash it.
function BubbleLoader({ label }: { label: string }) {
  return (
    <div
      role="status"
      className="loader-in mt-4 flex items-center gap-3 rounded-full bg-background/95 py-2.5 pr-5 pl-4 shadow-lg"
    >
      <span aria-hidden="true" className="bubble-loader">
        <span />
        <span />
        <span />
        <span />
        <span />
      </span>
      <span className="text-sm font-semibold text-primary">{label}</span>
    </div>
  );
}
