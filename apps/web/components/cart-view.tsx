"use client";

import { useTranslations } from "next-intl";
import { Notice } from "@/components/notice";
import { Button, ButtonLink } from "@/components/button";
import { CartLineItem } from "@/components/cart-line-item";
import { useCart } from "@/lib/cart-context";
import { formatPrice } from "@/lib/format";

export function CartView() {
  const t = useTranslations("Cart");
  const { lines, subtotal, clearCart } = useCart();

  if (lines.length === 0) {
    return (
      <div className="flex flex-col items-center gap-6 py-16 text-center">
        <Notice>{t("empty")}</Notice>
        <ButtonLink href="/menu">{t("continueShopping")}</ButtonLink>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8 py-8">
      <ul className="m-0 list-none p-0">
        {lines.map((line) => (
          <CartLineItem key={line.key} line={line} />
        ))}
      </ul>
      <div className="flex flex-col items-end gap-4 border-t border-text/10 pt-6">
        <p className="m-0 text-lg font-semibold">
          {t("subtotal")}: <span className="text-primary">{formatPrice(subtotal.toFixed(2))}</span>
        </p>
        <div className="flex flex-wrap justify-end gap-3">
          <ButtonLink href="/menu" variant="secondary">
            {t("continueShopping")}
          </ButtonLink>
          <Button variant="secondary" onClick={clearCart}>
            {t("clearCart")}
          </Button>
          <ButtonLink href="/order">{t("proceedToOrder")}</ButtonLink>
        </div>
      </div>
    </div>
  );
}
