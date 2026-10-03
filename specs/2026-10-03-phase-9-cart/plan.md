# Phase 9 — Cart: Implementation Plan

All groups touch only `apps/web` — no `apps/api`, no infra files. Every new
file is TypeScript/TSX under strict mode + `@typescript-eslint/no-explicit-any:
error`; no `any` anywhere, `unknown` + narrowing where input isn't already
typed (localStorage read).

## Group 0 — Branch

Branched `2026-10-03-phase-9-cart` off `master` (Phase 8 already merged;
this phase touches no file unique to any unmerged branch).

## Group 1 — Cart state: `apps/web/lib/cart-context.tsx`

New file. Exports the line type, the `CartProvider`, and the `useCart` hook.

```tsx
"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useState,
  type ReactNode,
} from "react";
import { parsePrice } from "@/lib/format";

export interface CartLine {
  key: string;
  itemId: number;
  priceOptionId: number | null;
  name: string;
  optionLabel: string | null;
  unitPrice: string;
  imageUrl: string | null;
  quantity: number;
}

type NewCartLine = Omit<CartLine, "key" | "quantity">;

type CartAction =
  | { type: "hydrate"; lines: CartLine[] }
  | { type: "add"; line: NewCartLine; quantity: number }
  | { type: "setQuantity"; key: string; quantity: number }
  | { type: "remove"; key: string }
  | { type: "clear" };

const STORAGE_KEY = "zainstreat:cart:v1";

function lineKey(itemId: number, priceOptionId: number | null): string {
  return `${itemId}:${priceOptionId ?? "flat"}`;
}

function isCartLine(value: unknown): value is CartLine {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.key === "string" &&
    typeof v.itemId === "number" &&
    (v.priceOptionId === null || typeof v.priceOptionId === "number") &&
    typeof v.name === "string" &&
    (v.optionLabel === null || typeof v.optionLabel === "string") &&
    typeof v.unitPrice === "string" &&
    (v.imageUrl === null || typeof v.imageUrl === "string") &&
    typeof v.quantity === "number" &&
    v.quantity > 0
  );
}

function cartReducer(lines: CartLine[], action: CartAction): CartLine[] {
  switch (action.type) {
    case "hydrate":
      return action.lines;
    case "add": {
      const key = lineKey(action.line.itemId, action.line.priceOptionId);
      const existing = lines.find((line) => line.key === key);
      if (existing) {
        return lines.map((line) =>
          line.key === key ? { ...line, quantity: line.quantity + action.quantity } : line,
        );
      }
      return [...lines, { ...action.line, key, quantity: action.quantity }];
    }
    case "setQuantity": {
      if (action.quantity <= 0) {
        return lines.filter((line) => line.key !== action.key);
      }
      return lines.map((line) =>
        line.key === action.key ? { ...line, quantity: action.quantity } : line,
      );
    }
    case "remove":
      return lines.filter((line) => line.key !== action.key);
    case "clear":
      return [];
    default:
      return lines;
  }
}

interface CartContextValue {
  lines: CartLine[];
  itemCount: number;
  subtotal: number;
  addLine: (line: NewCartLine, quantity?: number) => void;
  setQuantity: (key: string, quantity: number) => void;
  removeLine: (key: string) => void;
  clearCart: () => void;
}

const CartContext = createContext<CartContextValue | null>(null);

export function CartProvider({ children }: { children: ReactNode }) {
  const [lines, dispatch] = useReducer(cartReducer, []);
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.every(isCartLine)) {
          dispatch({ type: "hydrate", lines: parsed });
        }
      }
    } catch {
      // localStorage unavailable or corrupt — start from an empty cart.
    } finally {
      setIsHydrated(true);
    }
  }, []);

  useEffect(() => {
    if (!isHydrated) return;
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(lines));
    } catch {
      // localStorage unavailable (quota, private browsing) — degrade to in-memory only.
    }
  }, [lines, isHydrated]);

  const addLine = useCallback((line: NewCartLine, quantity = 1) => {
    dispatch({ type: "add", line, quantity });
  }, []);
  const setQuantity = useCallback((key: string, quantity: number) => {
    dispatch({ type: "setQuantity", key, quantity });
  }, []);
  const removeLine = useCallback((key: string) => {
    dispatch({ type: "remove", key });
  }, []);
  const clearCart = useCallback(() => {
    dispatch({ type: "clear" });
  }, []);

  const itemCount = useMemo(() => lines.reduce((sum, line) => sum + line.quantity, 0), [lines]);
  const subtotal = useMemo(
    () => lines.reduce((sum, line) => sum + parsePrice(line.unitPrice) * line.quantity, 0),
    [lines],
  );

  const value = useMemo(
    () => ({ lines, itemCount, subtotal, addLine, setQuantity, removeLine, clearCart }),
    [lines, itemCount, subtotal, addLine, setQuantity, removeLine, clearCart],
  );

  return <CartContext.Provider value={value}>{children}</CartContext.Provider>;
}

export function useCart(): CartContextValue {
  const context = useContext(CartContext);
  if (!context) {
    throw new Error("useCart must be used within a CartProvider");
  }
  return context;
}
```

Notes:
- `isHydrated` guards the persist effect so the initial empty-array render
  doesn't clobber real stored data before the hydrate effect's `dispatch`
  has taken effect (both effects fire once after first mount, in
  declaration order, within the same commit — the persist effect would
  otherwise write `[]` over whatever was in storage).
- `parsePrice` is `apps/web/lib/format.ts`'s existing
  `Number.parseFloat` wrapper — reused as-is, no new parsing logic.
- `quantity > 0` is part of the `isCartLine` guard so a hand-edited or
  corrupted stored value with `quantity: 0` doesn't resurrect a
  zero-quantity line.

## Group 2 — Shared cart UI primitives

**`apps/web/components/cart-icon.tsx`** (new) — a small inline SVG, no icon
library installed anywhere in the repo:

```tsx
export function CartIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <circle cx="9" cy="21" r="1" />
      <circle cx="20" cy="21" r="1" />
      <path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6" />
    </svg>
  );
}
```

**`apps/web/components/cart-badge.tsx`** (new), `"use client"`:

```tsx
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
```

**`apps/web/components/quantity-stepper.tsx`** (new), `"use client"`:

```tsx
"use client";

export function QuantityStepper({
  quantity,
  onDecrement,
  onIncrement,
  decrementLabel,
  incrementLabel,
}: {
  quantity: number;
  onDecrement: () => void;
  onIncrement: () => void;
  decrementLabel: string;
  incrementLabel: string;
}) {
  return (
    <div className="inline-flex items-center gap-3 rounded-full border border-text/15 px-1 py-1">
      <button
        type="button"
        aria-label={decrementLabel}
        onClick={onDecrement}
        className="flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        −
      </button>
      <span aria-live="polite" className="w-5 text-center text-sm font-semibold">
        {quantity}
      </span>
      <button
        type="button"
        aria-label={incrementLabel}
        onClick={onIncrement}
        className="flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        +
      </button>
    </div>
  );
}
```

Decrementing to 0 is handled by the caller passing `quantity - 1` to
`setQuantity`, which the reducer treats as a removal (Group 1). A separate
explicit "Remove" action also exists on each cart line (Group 5) regardless
of current quantity.

## Group 3 — Menu integration

**`apps/web/components/add-to-cart-controls.tsx`** (new), `"use client"`:

```tsx
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
            unitPrice: item.price as string,
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
```

`item.price as string` is safe inside the `item.price !== null` branch
(narrowing already proved it) — not an unchecked cast of untrusted input,
same pattern TypeScript would infer automatically outside a
destructured-prop edge case; kept explicit here only because `item.price`
is read again inside a closure passed to `onClick`, which widens it back
to `string | null` without a re-check. If `tsc` narrows it fine without the
cast during implementation, drop the cast — don't fight the compiler
either way.

**Edit `apps/web/components/menu-item-card.tsx`**: add the import and
render `<AddToCartControls item={item} />` as the last child inside the
existing `<div className="flex flex-1 flex-col gap-1.5 p-5">`, directly
after the existing flat-price/`price_options` display block. No other
changes to this file — the existing price/option *display* markup is
untouched, this only appends the interactive controls below it.

## Group 4 — Header integration

**Edit `apps/web/components/site-header.tsx`**: add one more `<li>` to the
existing `NAV_ITEMS`-rendered `<ul>`, placed right before the `LocaleToggle`
`<li>`:

```tsx
<li className="px-4 py-2 min-[769px]:px-0 min-[769px]:py-0">
  <Link
    href="/cart"
    aria-label={t("cart")}
    className="relative inline-flex h-10 w-10 items-center justify-center rounded-full text-primary hover:bg-background-soft"
  >
    <CartIcon className="h-5 w-5" />
    <CartBadge />
  </Link>
</li>
```

Add `import { CartIcon } from "@/components/cart-icon";` and
`import { CartBadge } from "@/components/cart-badge";` to this file's
existing import block. `SiteHeader` is already `"use client"`, so no new
client boundary is introduced.

**Edit `apps/web/app/[locale]/layout.tsx`**: wrap `CartProvider` around the
existing body content, inside `NextIntlClientProvider`:

```tsx
<NextIntlClientProvider>
  <CartProvider>
    <MotionProvider />
    <UtilityBar />
    <SiteHeader />
    <PageTransition>{children}</PageTransition>
    <SiteFooter />
  </CartProvider>
</NextIntlClientProvider>
```

Add `import { CartProvider } from "@/lib/cart-context";` to this file's
existing import block.

## Group 5 — Cart page

**`apps/web/app/[locale]/cart/page.tsx`** (new), server component (mirrors
`menu/page.tsx`'s `generateMetadata` pattern):

```tsx
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { PageHero } from "@/components/page-hero";
import { CartView } from "@/components/cart-view";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "Cart" });

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}

export default async function CartPage() {
  const t = await getTranslations("Cart");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading")}</h1>
      </PageHero>
      <CartView />
    </main>
  );
}
```

**`apps/web/components/cart-view.tsx`** (new), `"use client"`:

```tsx
"use client";

import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
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
          <Button disabled title={t("proceedToOrderComingSoon")}>
            {t("proceedToOrder")}
          </Button>
        </div>
      </div>
    </div>
  );
}
```

Note: imports `Link` from `@/i18n/navigation` for parity with the rest of
the codebase even though this file doesn't end up using it directly once
`ButtonLink` is used everywhere — drop the unused import during
implementation if `eslint`/`tsc` flags it (`ButtonLink` already wraps
`Link` internally, so a bare `Link` import here is likely unnecessary;
left as a reminder to double check rather than asserted as needed).

**`apps/web/components/cart-line-item.tsx`** (new), `"use client"`:

```tsx
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
```

## Group 6 — Translations

Add to **both** `apps/web/messages/en.json` and
`apps/web/messages/nl.json`:

- A new top-level `Cart` namespace:

  English:
  ```json
  "Cart": {
    "metaTitle": "Cart | Zain's Treat n More",
    "metaDescription": "Review your order before checking out.",
    "heading": "Your Cart",
    "empty": "Your cart is empty.",
    "continueShopping": "Continue Shopping",
    "subtotal": "Subtotal",
    "clearCart": "Clear Cart",
    "proceedToOrder": "Proceed to Order",
    "proceedToOrderComingSoon": "Order submission is coming soon.",
    "remove": "Remove",
    "increaseQuantity": "Increase quantity of {name}",
    "decreaseQuantity": "Decrease quantity of {name}"
  }
  ```

  Dutch:
  ```json
  "Cart": {
    "metaTitle": "Winkelwagen | Zain's Treat n More",
    "metaDescription": "Bekijk je bestelling voordat je afrekent.",
    "heading": "Jouw Winkelwagen",
    "empty": "Je winkelwagen is leeg.",
    "continueShopping": "Verder Winkelen",
    "subtotal": "Subtotaal",
    "clearCart": "Winkelwagen Legen",
    "proceedToOrder": "Doorgaan naar Bestellen",
    "proceedToOrderComingSoon": "Bestellen komt binnenkort beschikbaar.",
    "remove": "Verwijderen",
    "increaseQuantity": "Aantal {name} verhogen",
    "decreaseQuantity": "Aantal {name} verlagen"
  }
  ```

- `MenuPage` namespace gains two keys (English shown; Dutch equivalents
  follow the same key names):
  ```json
  "addToCart": "Add to Cart",
  "addToCartOption": "Add {option}"
  ```
  Dutch: `"addToCart": "In Winkelwagen"`, `"addToCartOption": "{option} Toevoegen"`.

- `SiteHeader` namespace gains one key:
  ```json
  "cart": "Cart"
  ```
  Dutch: `"cart": "Winkelwagen"`.

Both files must stay in sync (same key set) — matches the existing
convention confirmed across every other namespace in both files today.

## Verification

See `validation.md`.
