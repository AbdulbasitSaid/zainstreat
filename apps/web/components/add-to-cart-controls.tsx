"use client";

import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { useCart } from "@/lib/cart-context";
import type { MenuItem } from "@/lib/api";

export function AddToCartControls({ item }: { item: MenuItem }) {
  const t = useTranslations("MenuPage");
  const { addLine } = useCart();

  if (!item.is_available) return null;

  if (item.price !== null) {
    const flatPrice = item.price;
    return (
      <Button
        variant="outline"
        className="mt-2 w-full justify-center"
        onClick={() =>
          addLine({
            itemId: item.id,
            priceOptionId: null,
            name: item.name,
            optionLabel: null,
            unitPrice: flatPrice,
            imageUrl: item.image_url,
          })
        }
      >
        {t("addToCart")}
      </Button>
    );
  }

  return (
    <div className="mt-2 flex flex-col gap-1.5">
      {item.price_options.map((option) => (
        <Button
          key={option.id}
          variant="outline"
          className="w-full justify-between text-xs"
          onClick={() =>
            addLine({
              itemId: item.id,
              priceOptionId: option.id,
              name: item.name,
              optionLabel: option.label,
              unitPrice: option.price,
              imageUrl: item.image_url,
            })
          }
        >
          {t("addToCartOption", { option: option.label })}
        </Button>
      ))}
    </div>
  );
}
