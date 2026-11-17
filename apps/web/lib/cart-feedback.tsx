"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { AddToast, type AddToastContent } from "@/components/add-toast";
import { gsap } from "@/lib/gsap";
import { useReducedMotion } from "@/lib/use-reduced-motion";

// Add-to-cart feedback (Phase 30 Decisions 2, 4, 6): a toast for every add,
// the dish photo flying into the visible cart icon, and events the cart
// badge bounces on. Only a deliberate add (AddToCartControls) calls it, so
// loading a saved cart or editing quantities on /cart never celebrates.

export type AddAnnouncement = {
  name: string;
  optionLabel: string | null;
  imageUrl: string | null;
  /** The button that was clicked, where the fly starts. */
  origin: DOMRect;
};

/** Fired as soon as an add is announced; `detail.flying` says whether to wait for a landing. */
export const CART_ADDED_EVENT = "zt:cart-added";
/** Fired when the flying photo reaches the cart icon. */
export const CART_LANDED_EVENT = "zt:cart-landed";

export type CartAddedDetail = { flying: boolean };

type CartFeedbackValue = { announceAdd: (announcement: AddAnnouncement) => void };

const CartFeedbackContext = createContext<CartFeedbackValue>({ announceAdd: () => {} });

const FLY_SIZE_PX = 48;
const FLY_DURATION_S = 0.65;

/** The cart icon showing at the current width (one per breakpoint). */
function visibleCartTarget(): HTMLElement | null {
  const targets = document.querySelectorAll<HTMLElement>("[data-cart-target]");
  return Array.from(targets).find((el) => el.getClientRects().length > 0) ?? null;
}

function flyToCart(layer: HTMLElement, origin: DOMRect, imageUrl: string | null): boolean {
  const target = visibleCartTarget();
  if (!target) return false;
  const to = target.getBoundingClientRect();

  const el = document.createElement(imageUrl ? "img" : "div");
  if (el instanceof HTMLImageElement && imageUrl) {
    el.src = imageUrl;
    el.alt = "";
  }
  el.className = `absolute rounded-full object-cover shadow-lg ring-2 ring-white ${imageUrl ? "" : "bg-primary"}`;
  Object.assign(el.style, {
    width: `${FLY_SIZE_PX}px`,
    height: `${FLY_SIZE_PX}px`,
    left: `${origin.left + origin.width / 2 - FLY_SIZE_PX / 2}px`,
    top: `${origin.top + origin.height / 2 - FLY_SIZE_PX / 2}px`,
  });
  layer.appendChild(el);

  const dx = to.left + to.width / 2 - (origin.left + origin.width / 2);
  const dy = to.top + to.height / 2 - (origin.top + origin.height / 2);
  // Separate x and y eases bend the path into an arc.
  gsap
    .timeline({
      onComplete: () => {
        el.remove();
        window.dispatchEvent(new Event(CART_LANDED_EVENT));
      },
    })
    .to(el, { x: dx, duration: FLY_DURATION_S, ease: "power1.out" }, 0)
    .to(el, { y: dy, duration: FLY_DURATION_S, ease: "power2.in" }, 0)
    .to(el, { scale: 1 / 3, opacity: 0, duration: 0.2 }, FLY_DURATION_S - 0.2);
  return true;
}

export function CartFeedbackProvider({ children }: { children: ReactNode }) {
  const reducedMotion = useReducedMotion();
  const layerRef = useRef<HTMLDivElement>(null);
  const nextId = useRef(0);
  const [toast, setToast] = useState<AddToastContent | null>(null);
  const [open, setOpen] = useState(false);

  const announceAdd = useCallback(
    ({ name, optionLabel, imageUrl, origin }: AddAnnouncement) => {
      nextId.current += 1;
      setToast({ id: nextId.current, name, optionLabel, imageUrl });
      setOpen(true);
      const flying = !reducedMotion && layerRef.current !== null && flyToCart(layerRef.current, origin, imageUrl);
      window.dispatchEvent(new CustomEvent<CartAddedDetail>(CART_ADDED_EVENT, { detail: { flying } }));
    },
    [reducedMotion],
  );

  // Don't leave a photo mid-flight behind.
  useEffect(() => {
    const layer = layerRef.current;
    return () => {
      if (!layer) return;
      gsap.killTweensOf(layer.children);
      layer.replaceChildren();
    };
  }, []);

  const value = useMemo(() => ({ announceAdd }), [announceAdd]);
  const dismiss = useCallback(() => setOpen(false), []);

  return (
    <CartFeedbackContext.Provider value={value}>
      {children}
      <AddToast content={toast} open={open} onDismiss={dismiss} />
      <div ref={layerRef} aria-hidden="true" className="pointer-events-none fixed inset-0 z-[60] overflow-hidden" />
    </CartFeedbackContext.Provider>
  );
}

export function useCartFeedback(): CartFeedbackValue {
  return useContext(CartFeedbackContext);
}
