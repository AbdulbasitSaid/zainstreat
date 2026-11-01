import { useTranslations } from "next-intl";
import Image from "next/image";
import { ImageSlot } from "@/components/image-slot";
import { AddToCartControls } from "@/components/add-to-cart-controls";
import { formatPrice, formatPriceOrNull } from "@/lib/format";
import type { MenuItem } from "@/lib/api";
import type { BentoSize } from "@/lib/menu-bento";

const PHOTO_CLASS: Record<BentoSize, string> = {
  regular: "h-[180px]",
  tall: "h-[260px]",
  wide: "h-[180px] sm:h-full sm:min-h-[180px]",
  feature: "h-[220px] sm:h-[320px]",
};

const PHOTO_SIZES: Record<BentoSize, string> = {
  regular: "(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 400px",
  tall: "(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 400px",
  wide: "(max-width: 640px) 100vw, 50vw",
  feature: "(max-width: 640px) 100vw, 50vw",
};

export function MenuItemCard({ item, size = "regular" }: { item: MenuItem; size?: BentoSize }) {
  const t = useTranslations("MenuPage");
  const flatPrice = formatPriceOrNull(item.price);

  return (
    <div
      className={`flex h-full flex-col overflow-hidden rounded-card border border-text/10 bg-card shadow-sm ${
        size === "wide" ? "sm:flex-row" : ""
      } ${item.is_available ? "" : "opacity-60 grayscale-[30%]"}`.trim()}
    >
      <div className={`relative ${size === "wide" ? "sm:w-1/2 sm:shrink-0" : ""}`.trim()}>
        {item.image_url ? (
          <Image
            src={item.image_url}
            alt={item.name}
            width={600}
            height={400}
            sizes={PHOTO_SIZES[size]}
            // Phase 14's uploaded photos are absolute URLs served by the API
            // (apps/api's media route). In dev, next/image's server-side
            // optimizer runs inside the `web` container, which can't reach
            // the api container at the browser-facing `localhost:8080` host
            // — so only that dev-only scheme stays unoptimized. Production's
            // `https://api.$DOMAIN` URLs now get real format conversion
            // (Phase 17 Decision 9). Relative seed-data paths (same origin)
            // are unaffected and keep optimizing.
            unoptimized={item.image_url.startsWith("http://")}
            className={`${PHOTO_CLASS[size]} w-full object-cover`}
          />
        ) : (
          <ImageSlot label={item.name} className={`${PHOTO_CLASS[size]} min-h-0 rounded-none`} />
        )}
        {!item.is_available && (
          <span className="absolute top-3 right-3 rounded-full bg-primary-dark px-3 py-1 text-[0.7rem] font-semibold tracking-wide text-text-light uppercase">
            {t("unavailable")}
          </span>
        )}
      </div>
      <div className="flex flex-1 flex-col gap-1.5 p-5">
        <h3 className={`m-0 ${size === "feature" ? "text-[1.35rem]" : "text-[1.05rem]"}`}>{item.name}</h3>
        {item.description && (
          <p className="m-0 text-[0.9rem] text-text-muted">{item.description}</p>
        )}
        {flatPrice !== null ? (
          <p className="m-0 mt-auto pt-2 text-[1.05rem] font-semibold text-primary">
            {flatPrice}
          </p>
        ) : (
          <ul className="m-0 mt-auto flex flex-col gap-0.5 pt-2">
            {item.price_options.map((option) => (
              <li
                key={option.id}
                className="flex items-baseline justify-between gap-3 text-[0.95rem]"
              >
                <span className="text-text-muted">{option.label}</span>
                <span className="font-semibold text-primary">{formatPrice(option.price)}</span>
              </li>
            ))}
          </ul>
        )}
        <AddToCartControls item={item} />
      </div>
    </div>
  );
}
