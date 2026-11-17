"use client";

import { useCallback, useEffect, useRef, useState, type TouchEvent } from "react";
import Image from "next/image";
import { useTranslations } from "next-intl";
import type { EventPhoto } from "@/lib/events";

const SWIPE_PX = 40;

// A past event's photos (Phase 31 Decision 13): a grid, opening a native
// <dialog> lightbox with arrow keys, swipe and Esc.
export function EventGallery({ photos, title }: { photos: EventPhoto[]; title: string }) {
  const t = useTranslations("EventPage");
  const dialogRef = useRef<HTMLDialogElement>(null);
  const touchStart = useRef<number | null>(null);
  const [open, setOpen] = useState<number | null>(null);

  const show = (index: number) => {
    setOpen(index);
    dialogRef.current?.showModal();
  };
  const close = () => dialogRef.current?.close();
  const step = useCallback(
    (delta: number) => setOpen((current) => (current === null ? null : (current + delta + photos.length) % photos.length)),
    [photos.length],
  );

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "ArrowRight") step(1);
      if (event.key === "ArrowLeft") step(-1);
    };
    const onClose = () => setOpen(null);
    dialog.addEventListener("keydown", onKey);
    dialog.addEventListener("close", onClose);
    return () => {
      dialog.removeEventListener("keydown", onKey);
      dialog.removeEventListener("close", onClose);
    };
  }, [step]);

  const onTouchStart = (event: TouchEvent) => {
    touchStart.current = event.touches[0]?.clientX ?? null;
  };
  const onTouchEnd = (event: TouchEvent) => {
    const start = touchStart.current;
    const end = event.changedTouches[0]?.clientX;
    touchStart.current = null;
    if (start === null || end === undefined || Math.abs(end - start) < SWIPE_PX) return;
    step(end < start ? 1 : -1);
  };

  const current = open === null ? null : photos[open];

  return (
    <>
      <ul className="m-0 grid list-none grid-cols-2 gap-3 p-0 md:grid-cols-3">
        {photos.map((photo, index) => (
          <li key={`${photo.image_url}-${index}`} className={index % 5 === 0 ? "col-span-2 row-span-2" : ""}>
            <button
              type="button"
              onClick={() => show(index)}
              className="group/photo relative block aspect-square w-full overflow-hidden rounded-card focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-offset-2 focus-visible:outline-accent"
              aria-label={t("openPhoto", { n: index + 1, total: photos.length })}
            >
              <Image
                src={photo.image_url}
                alt={photo.caption ?? ""}
                fill
                sizes={index % 5 === 0 ? "(max-width: 768px) 100vw, 66vw" : "(max-width: 768px) 50vw, 33vw"}
                unoptimized={photo.image_url.startsWith("http://")}
                className="object-cover transition-transform duration-700 ease-out-expo group-hover/photo:scale-105"
              />
            </button>
          </li>
        ))}
      </ul>

      <dialog
        ref={dialogRef}
        aria-label={title}
        onClick={(event) => event.target === dialogRef.current && close()}
        onTouchStart={onTouchStart}
        onTouchEnd={onTouchEnd}
        className="m-auto max-h-[95vh] w-[min(1100px,95vw)] max-w-none overflow-hidden rounded-card bg-text p-0 text-white backdrop:bg-text/80"
      >
        {current && (
          <figure className="m-0 flex flex-col">
            <div className="relative h-[min(75vh,800px)] w-full">
              <Image
                src={current.image_url}
                alt={current.caption ?? ""}
                fill
                sizes="95vw"
                unoptimized={current.image_url.startsWith("http://")}
                className="object-contain"
              />
            </div>
            <figcaption className="flex items-center justify-between gap-3 px-4 py-3 text-sm">
              <button type="button" onClick={() => step(-1)} className="rounded-full px-3 py-2 hover:bg-white/10" aria-label={t("previousPhoto")}>
                ←
              </button>
              <span className="flex-1 text-center">
                {current.caption}
                <span className="ml-2 text-on-dark-faint">
                  {(open ?? 0) + 1} / {photos.length}
                </span>
              </span>
              <button type="button" onClick={() => step(1)} className="rounded-full px-3 py-2 hover:bg-white/10" aria-label={t("nextPhoto")}>
                →
              </button>
              <button type="button" onClick={close} className="rounded-full px-3 py-2 hover:bg-white/10" aria-label={t("closePhoto")}>
                ×
              </button>
            </figcaption>
          </figure>
        )}
      </dialog>
    </>
  );
}
