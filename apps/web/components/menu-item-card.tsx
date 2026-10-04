import { useTranslations } from "next-intl";
import Image from "next/image";
import { ImageSlot } from "@/components/image-slot";
import { AddToCartControls } from "@/components/add-to-cart-controls";
import { formatPrice, formatPriceOrNull } from "@/lib/format";
import type { MenuItem } from "@/lib/api";

export function MenuItemCard({ item }: { item: MenuItem }) {
  const t = useTranslations("MenuPage");
  const flatPrice = formatPriceOrNull(item.price);

  return (
    <div
      className={`flex flex-col overflow-hidden rounded-card border border-text/10 bg-card shadow-sm ${
        item.is_available ? "" : "opacity-60 grayscale-[30%]"
      }`.trim()}
    >
      <div className="relative">
        {item.image_url ? (
          <Image
            src={item.image_url}
            alt={item.name}
            width={600}
            height={400}
            // Phase 14's uploaded photos are absolute URLs served by the API
            // (apps/api's media route); the backend already serves a fixed
            // pre-cropped square and never generates resized variants, so
            // next/image's own optimizer has nothing to gain here — and in
            // dev, that optimizer runs server-side inside the `web`
            // container, which can't reach the api container at the
            // browser-facing `localhost:8080` host. Relative seed-data
            // paths (same origin) are unaffected and keep optimizing.
            unoptimized={item.image_url.startsWith("http")}
            className="h-[180px] w-full object-cover"
          />
        ) : (
          <ImageSlot label={item.name} className="h-[180px] min-h-0 rounded-none" />
        )}
        {!item.is_available && (
          <span className="absolute top-3 right-3 rounded-full bg-primary-dark px-3 py-1 text-[0.7rem] font-semibold tracking-wide text-text-light uppercase">
            {t("unavailable")}
          </span>
        )}
      </div>
      <div className="flex flex-1 flex-col gap-1.5 p-5">
        <h3 className="m-0 text-[1.05rem]">{item.name}</h3>
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
