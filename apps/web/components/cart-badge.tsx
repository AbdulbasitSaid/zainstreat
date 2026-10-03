"use client";

import { useCart } from "@/lib/cart-context";

export function CartBadge() {
  const { itemCount } = useCart();
  if (itemCount === 0) return null;

  return (
    <span className="absolute -top-1 -right-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-primary px-1 text-[0.65rem] font-bold text-white">
      {itemCount}
    </span>
  );
}
