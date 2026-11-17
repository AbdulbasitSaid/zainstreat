"use client";

import { useEffect, useRef, useState } from "react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { gsap, useGSAP } from "@/lib/gsap";
import { useReducedMotion } from "@/lib/use-reduced-motion";

export type AddToastContent = {
  /** New per add, so adding the same dish again restarts the toast. */
  id: number;
  name: string;
  optionLabel: string | null;
  imageUrl: string | null;
};

const VISIBLE_MS = 3500;

/**
 * "Jollof Rice added to your order" (Phase 30 Decision 6). Bottom centre on
 * phones, top right under the header from 769px. Hovering or focusing it
 * pauses the auto-hide. A separate, always-present status region announces
 * each add to screen readers without moving focus.
 */
export function AddToast({
  content,
  open,
  onDismiss,
}: {
  content: AddToastContent | null;
  open: boolean;
  onDismiss: () => void;
}) {
  const t = useTranslations("CartFeedback");
  const reducedMotion = useReducedMotion();
  const cardRef = useRef<HTMLDivElement>(null);
  // Keyed to the toast id, so a new add never inherits an old pause.
  const [pausedId, setPausedId] = useState<number | null>(null);
  const paused = content !== null && pausedId === content.id;

  const message = content
    ? t.rich(content.optionLabel ? "addedOption" : "added", {
        name: content.name,
        option: content.optionLabel ?? "",
        strong: (chunks) => <strong className="font-semibold">{chunks}</strong>,
      })
    : null;

  useGSAP(
    () => {
      const card = cardRef.current;
      if (!card) return;
      gsap.to(card, {
        autoAlpha: open ? 1 : 0,
        y: open ? 0 : 16,
        duration: reducedMotion ? 0 : 0.25,
        ease: "outExpo",
      });
    },
    { dependencies: [open, content?.id, reducedMotion] },
  );

  useEffect(() => {
    if (!open || paused) return;
    const timer = window.setTimeout(onDismiss, VISIBLE_MS);
    return () => window.clearTimeout(timer);
  }, [open, paused, content?.id, onDismiss]);

  return (
    <>
      <div role="status" aria-live="polite" className="sr-only">
        {open && content && <span key={content.id}>{message}</span>}
      </div>
      <div
        ref={cardRef}
        inert={!open}
        aria-hidden={!open}
        style={{ opacity: 0, visibility: "hidden", transform: "translateY(16px)" }}
        onPointerEnter={() => setPausedId(content?.id ?? null)}
        onPointerLeave={() => setPausedId(null)}
        onFocus={() => setPausedId(content?.id ?? null)}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) setPausedId(null);
        }}
        className="fixed bottom-[calc(1.5rem+env(safe-area-inset-bottom))] left-1/2 z-[60] w-[min(22rem,calc(100vw-7rem))] -translate-x-1/2 min-[769px]:top-20 min-[769px]:right-6 min-[769px]:bottom-auto min-[769px]:left-auto min-[769px]:w-80 min-[769px]:translate-x-0"
      >
        <div className="flex items-center gap-3 rounded-2xl border border-text/10 bg-background p-3 text-text shadow-xl">
          {content?.imageUrl && (
            // A tiny thumbnail of an already-loaded photo; next/image adds nothing here.
            // eslint-disable-next-line @next/next/no-img-element
            <img src={content.imageUrl} alt="" className="h-10 w-10 shrink-0 rounded-full object-cover" />
          )}
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <p className="m-0 text-sm leading-snug">{message}</p>
            <Link href="/cart" className="text-sm font-semibold text-primary underline" onClick={onDismiss}>
              {t("viewCart")}
            </Link>
          </div>
          <button
            type="button"
            aria-label={t("dismiss")}
            onClick={onDismiss}
            className="inline-flex h-8 w-8 shrink-0 items-center justify-center self-start rounded-full text-lg text-text-muted hover:bg-text/10 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2"
          >
            <span aria-hidden="true">×</span>
          </button>
        </div>
      </div>
    </>
  );
}
