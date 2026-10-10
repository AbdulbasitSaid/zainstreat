"use client";

import Image from "next/image";
import { useTranslations } from "next-intl";
import { ImageSlot } from "@/components/image-slot";
import { QuantityStepper } from "@/components/quantity-stepper";
import { formatPrice } from "@/lib/format";
import { useCart, type CartLine } from "@/lib/cart-context";

export function CartLineItem({ line }: { line: CartLine }) {
  const t = useTranslations("Cart");
  const { setQuantity, removeLine } = useCart();
  const lineTotal = (Number.parseFloat(line.unitPrice) * line.quantity).toFixed(2);

  return (
    <li className="flex items-center gap-4 border-b border-text/10 py-4 last:border-0">
      <div className="h-16 w-16 flex-shrink-0 overflow-hidden rounded-card">
        {line.imageUrl ? (
          <Image
            src={line.imageUrl}
            alt={line.name}
            width={64}
            height={64}
            sizes="64px"
            unoptimized={line.imageUrl.startsWith("http://")}
            className="h-full w-full object-cover"
          />
        ) : (
          <ImageSlot label={line.name} className="h-full min-h-0 p-0 text-[0.6rem]" />
        )}
      </div>
      <div className="flex flex-1 flex-col gap-0.5">
        <p className="m-0 font-semibold">{line.name}</p>
        {line.optionLabel && <p className="m-0 text-xs text-text-muted">{line.optionLabel}</p>}
        <p className="m-0 text-sm text-text-muted">{formatPrice(line.unitPrice)}</p>
      </div>
      <QuantityStepper
        quantity={line.quantity}
        onDecrement={() => setQuantity(line.key, line.quantity - 1)}
        onIncrement={() => setQuantity(line.key, line.quantity + 1)}
        decrementLabel={t("decreaseQuantity", { name: line.name })}
        incrementLabel={t("increaseQuantity", { name: line.name })}
      />
      <p className="m-0 w-20 text-right font-semibold text-primary">{formatPrice(lineTotal)}</p>
      <button
        type="button"
        onClick={() => removeLine(line.key)}
        className="text-xs text-text-muted underline hover:text-text"
      >
        {t("remove")}
      </button>
    </li>
  );
}
