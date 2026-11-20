import type { ReactNode } from "react";
import { useTranslations } from "next-intl";
import Image from "next/image";
import { ImageSlot } from "@/components/image-slot";
import { AddRentalToCart, AddToCartControls } from "@/components/add-to-cart-controls";
import { LiquidCard } from "@/components/liquid-card";
import { formatPrice } from "@/lib/format";
import type { MenuItem, Rental } from "@/lib/api";
import type { TilePlacement, TileShape, TileSlot, TileTint } from "@/lib/menu-bento";

type ShapeClasses = { root: string; photo: string; body: string; description: string };

// Written out in full, per breakpoint, so Tailwind's scanner finds every
// class. Each entry sets every property the other shapes set, so an `lg:`
// shape fully overrides the `sm:` one. In stacked tiles the text takes its
// natural height and the photo fills the rest; line clamps are sized to
// the 8rem row unit (Phase 25 Decision 7).
const SM_SHAPE: Record<TileShape, ShapeClasses> = {
  small: {
    root: "sm:flex-col",
    photo: "sm:h-auto sm:w-full sm:flex-1",
    body: "sm:flex-none",
    description: "sm:hidden",
  },
  tall: {
    root: "sm:flex-col",
    photo: "sm:h-auto sm:w-full sm:flex-1",
    body: "sm:flex-none",
    description: "sm:line-clamp-3",
  },
  wide: {
    root: "sm:flex-row",
    photo: "sm:h-full sm:w-1/2 sm:flex-none",
    body: "sm:flex-1",
    description: "sm:line-clamp-4",
  },
};

const LG_SHAPE: Record<TileShape, ShapeClasses> = {
  small: {
    root: "lg:flex-col",
    photo: "lg:h-auto lg:w-full lg:flex-1",
    body: "lg:flex-none",
    description: "lg:hidden",
  },
  tall: {
    root: "lg:flex-col",
    photo: "lg:h-auto lg:w-full lg:flex-1",
    body: "lg:flex-none",
    description: "lg:line-clamp-3",
  },
  wide: {
    root: "lg:flex-row",
    photo: "lg:h-full lg:w-1/2 lg:flex-none",
    body: "lg:flex-1",
    description: "lg:line-clamp-4",
  },
};

// A 4-row tall tile has room for twice the description.
const SM_LONG_DESCRIPTION = "sm:line-clamp-6";
const LG_LONG_DESCRIPTION = "lg:line-clamp-6";

const TINT_CLASS: Record<TileTint, { root: string; muted: string; price: string }> = {
  brand: { root: "bg-primary text-text-light", muted: "text-on-dark-body", price: "text-accent-light" },
  cream: { root: "bg-cream", muted: "text-text-muted", price: "text-primary" },
  soft: { root: "bg-background-soft", muted: "text-text-muted", price: "text-primary" },
  card: { root: "bg-card", muted: "text-text-muted", price: "text-primary" },
  white: { root: "border border-text/10 bg-background", muted: "text-text-muted", price: "text-primary" },
};

function descriptionClass(slot: TileSlot, table: Record<TileShape, ShapeClasses>, long: string): string {
  return slot.shape === "tall" && slot.rows >= 4 ? long : table[slot.shape].description;
}

// The lowest option's price, for "from €X" on multi-price items.
function lowestPrice(item: MenuItem): string | null {
  if (item.price !== null) return item.price;
  const [first, ...rest] = item.price_options;
  if (!first) return null;
  return rest.reduce((low, option) => (Number(option.price) < Number(low) ? option.price : low), first.price);
}

export function MenuItemCard({ item, placement }: { item: MenuItem; placement: TilePlacement }) {
  const t = useTranslations("MenuPage");
  const price = lowestPrice(item);

  return (
    <BentoCard
      placement={placement}
      name={item.name}
      imageUrl={item.image_url}
      description={item.description}
      dimmed={!item.is_available}
      badge={item.is_available ? null : t("unavailable")}
      note={item.min_quantity > 1 ? t("minimumOrder", { count: item.min_quantity }) : null}
      price={price === null ? null : item.price === null ? t("fromPrice", { price: formatPrice(price) }) : formatPrice(price)}
      action={(onBrand) => <AddToCartControls item={item} onBrand={onBrand} />}
    />
  );
}

// Phase 37: rentals in the same bento tiles. Admin crops their photos
// square, so `contain` on white shows the whole photo in any tile shape.
export function RentalCard({ rental, placement }: { rental: Rental; placement: TilePlacement }) {
  return (
    <BentoCard
      placement={placement}
      name={rental.name}
      imageUrl={rental.image_url}
      imageFit="contain"
      description={rental.description}
      price={formatPrice(rental.price)}
      action={(onBrand) => <AddRentalToCart rental={rental} onBrand={onBrand} />}
    />
  );
}

// One bento tile: photo, name, description, price and an add button.
function BentoCard({
  placement,
  name,
  imageUrl,
  imageFit = "cover",
  description,
  dimmed = false,
  badge = null,
  note = null,
  price,
  action,
}: {
  placement: TilePlacement;
  name: string;
  imageUrl: string | null;
  imageFit?: "cover" | "contain";
  description: string | null;
  dimmed?: boolean;
  badge?: string | null;
  note?: string | null;
  price: string | null;
  action: (onBrand: boolean) => ReactNode;
}) {
  const { sm, lg, tint } = placement;
  const tintClass = TINT_CLASS[tint];

  return (
    <LiquidCard
      wrapperClassName="h-full"
      strokeClassName={tint === "white" ? "stroke-text/10" : undefined}
      className={`flex h-full flex-col overflow-hidden rounded-card shadow-sm transition-shadow duration-300 ease-out-expo hover:shadow-[0_18px_36px_rgba(43,17,20,0.16)] ${SM_SHAPE[sm.shape].root} ${
        LG_SHAPE[lg.shape].root
      } ${tintClass.root} ${dimmed ? "opacity-60 grayscale-30" : ""}`.trim()}
    >
      <div
        className={`relative h-50 w-full shrink-0 sm:min-h-0 ${SM_SHAPE[sm.shape].photo} ${LG_SHAPE[lg.shape].photo} ${
          imageFit === "contain" ? "bg-white" : ""
        }`.trim()}
      >
        {imageUrl ? (
          <Image
            src={imageUrl}
            alt={name}
            fill
            sizes="(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 600px"
            // Phase 14's uploaded photos are absolute URLs served by the API
            // (apps/api's media route). In dev, next/image's server-side
            // optimizer runs inside the `web` container, which can't reach
            // the api container at the browser-facing `localhost:8080` host
            // — so only that dev-only scheme stays unoptimized. Production's
            // `https://api.$DOMAIN` URLs now get real format conversion
            // (Phase 17 Decision 9). Relative seed-data paths (same origin)
            // are unaffected and keep optimizing.
            unoptimized={imageUrl.startsWith("http://")}
            className={imageFit === "contain" ? "object-contain p-3" : "object-cover"}
          />
        ) : (
          <ImageSlot label={name} className="h-full min-h-0 rounded-none border-0" />
        )}
        {badge && (
          <span className="absolute top-3 right-3 rounded-full bg-primary-dark px-3 py-1 text-[0.7rem] font-semibold tracking-wide text-text-light uppercase">
            {badge}
          </span>
        )}
      </div>
      <div
        className={`flex min-h-0 min-w-0 flex-1 flex-col gap-1.5 p-4 ${SM_SHAPE[sm.shape].body} ${
          LG_SHAPE[lg.shape].body
        }`}
      >
        <h3 className="m-0 line-clamp-2 text-[1.05rem] text-inherit">{name}</h3>
        {description && (
          <p
            className={`m-0 line-clamp-3 text-[0.9rem] ${tintClass.muted} ${descriptionClass(
              sm,
              SM_SHAPE,
              SM_LONG_DESCRIPTION,
            )} ${descriptionClass(lg, LG_SHAPE, LG_LONG_DESCRIPTION)}`}
          >
            {description}
          </p>
        )}
        {note && <p className={`m-0 text-[0.8rem] font-semibold ${tintClass.price}`}>{note}</p>}
        <div className="mt-auto flex items-center justify-between gap-3 pt-2">
          {price !== null && <p className={`m-0 text-[1.05rem] font-semibold ${tintClass.price}`}>{price}</p>}
          {action(tint === "brand")}
        </div>
      </div>
    </LiquidCard>
  );
}
