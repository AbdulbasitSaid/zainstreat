"use client";

import { useRef } from "react";
import { useTranslations } from "next-intl";
import { LiquidSurface } from "@/components/liquid-surface";
import { useCart } from "@/lib/cart-context";
import { formatPrice } from "@/lib/format";
import { PREVIEW_DISABLED_TITLE, usePreview } from "@/lib/preview-context";
import type { MenuItem, MenuItemPriceOption } from "@/lib/api";

const FOCUS_CLASSES =
  "focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2";

// A compact round "+" so the price and the button share one row even in a
// small bento tile (Phase 25 Decision 9). `onBrand` inverts it for the
// `bg-primary` tiles. Hover swaps fill and glyph colours (the border keeps
// the inverted button visible against the tile) and spins the "+" once.
function addButtonClasses(onBrand: boolean) {
  return (
    "group/add group/liquid relative isolate inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-full border-2 text-xl leading-none " +
    "transition-[transform,background-color,color] duration-200 ease-out-expo hover:scale-115 active:scale-95 " +
    "disabled:pointer-events-none disabled:opacity-50 " +
    FOCUS_CLASSES +
    (onBrand
      ? " border-white bg-white text-primary-dark hover:bg-primary-dark hover:text-white"
      : " border-primary bg-primary text-white hover:bg-white hover:text-primary")
  );
}

// The "+" plus the button's liquid surface (mirrors addButtonClasses'
// fill, border and hover inversion).
function AddGlyph({ onBrand }: { onBrand: boolean }) {
  return (
    <>
      <span
        aria-hidden="true"
        className="inline-block transition-transform duration-500 ease-out-expo group-hover/add:rotate-360"
      >
        +
      </span>
      <LiquidSurface
        strokeWidth={2}
        pathClassName={`transition-[fill] duration-200 ${
          onBrand
            ? "fill-white stroke-white group-hover/liquid:fill-primary-dark"
            : "fill-primary stroke-primary group-hover/liquid:fill-white"
        }`}
        splashClassName={onBrand ? "fill-white" : "fill-primary"}
      />
    </>
  );
}

export function AddToCartControls({ item, onBrand = false }: { item: MenuItem; onBrand?: boolean }) {
  const t = useTranslations("MenuPage");
  const { addLine } = useCart();
  const preview = usePreview();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const previewProps = preview ? { disabled: true, title: PREVIEW_DISABLED_TITLE } : {};

  if (!item.is_available) return null;

  const addOption = (option: MenuItemPriceOption | null, unitPrice: string) =>
    addLine({
      itemId: item.id,
      priceOptionId: option?.id ?? null,
      name: item.name,
      optionLabel: option?.label ?? null,
      unitPrice,
      imageUrl: item.image_url,
    });

  if (item.price !== null) {
    const flatPrice = item.price;
    return (
      <button
        type="button"
        aria-label={`${t("addToCart")}: ${item.name}`}
        className={addButtonClasses(onBrand)}
        {...previewProps}
        onClick={() => addOption(null, flatPrice)}
      >
        <AddGlyph onBrand={onBrand} />
      </button>
    );
  }

  // Multi-price items: one button opens a native <dialog> listing every
  // option (not window.confirm, which would block the page and any preview
  // frames). Adding an option closes it.
  return (
    <>
      <button
        type="button"
        aria-label={`${t("chooseOption")}: ${item.name}`}
        aria-haspopup="dialog"
        className={addButtonClasses(onBrand)}
        {...previewProps}
        onClick={() => dialogRef.current?.showModal()}
      >
        <AddGlyph onBrand={onBrand} />
      </button>
      <dialog
        ref={dialogRef}
        aria-labelledby={`options-${item.id}`}
        className="m-auto w-[min(26rem,calc(100vw-2rem))] rounded-card border border-text/10 bg-background p-6 text-text shadow-xl backdrop:bg-text/40"
      >
        <div className="mb-4 flex items-start justify-between gap-4">
          <div>
            <p className="m-0 text-sm text-text-muted">{t("chooseOption")}</p>
            <h2 id={`options-${item.id}`} className="m-0 text-xl">
              {item.name}
            </h2>
          </div>
          <button
            type="button"
            aria-label={t("closeDialog")}
            className={`-mt-1 -mr-2 inline-flex h-9 w-9 items-center justify-center rounded-full text-xl text-text-muted hover:bg-text/10 ${FOCUS_CLASSES}`}
            onClick={() => dialogRef.current?.close()}
          >
            <span aria-hidden="true">×</span>
          </button>
        </div>
        <ul className="m-0 flex list-none flex-col gap-2 p-0">
          {item.price_options.map((option) => (
            <li
              key={option.id}
              className="flex items-center justify-between gap-3 rounded-xl bg-cream px-4 py-2.5"
            >
              <span>{option.label}</span>
              <span className="ml-auto font-semibold text-primary">{formatPrice(option.price)}</span>
              <button
                type="button"
                aria-label={t("addToCartOption", { option: option.label })}
                className={addButtonClasses(false)}
                {...previewProps}
                onClick={() => {
                  addOption(option, option.price);
                  dialogRef.current?.close();
                }}
              >
                <AddGlyph onBrand={false} />
              </button>
            </li>
          ))}
        </ul>
      </dialog>
    </>
  );
}
