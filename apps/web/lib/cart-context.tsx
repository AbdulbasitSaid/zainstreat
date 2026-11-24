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
import { useLocale } from "next-intl";
import { parsePrice } from "@/lib/format";
import type { LocalizedText } from "@/lib/api";
import type { AppLocale } from "@/i18n/routing";

// A menu item or a rental (Phase 37 Decision 2). For a rental, `itemId` is
// the rental's id.
export type CartLineKind = "menu" | "rental";

export interface CartLine {
  key: string;
  kind: CartLineKind;
  itemId: number;
  priceOptionId: number | null;
  name: string;
  optionLabel: string | null;
  unitPrice: string;
  imageUrl: string | null;
  quantity: number;
  // The item's minimum order when it was added (Phase 36 Decision 3).
  minQuantity: number;
  // Phase 38 Decision 8: every language, so the cart follows the site's.
  // Lines stored before Phase 38 have neither and show `name` as stored.
  names?: LocalizedText;
  optionLabels?: LocalizedText | null;
}

type NewCartLine = Omit<CartLine, "key" | "quantity">;

type CartAction =
  | { type: "hydrate"; lines: CartLine[] }
  | { type: "add"; line: NewCartLine; quantity: number }
  | { type: "setQuantity"; key: string; quantity: number }
  | { type: "remove"; key: string }
  | { type: "clear" };

const STORAGE_KEY = "zainstreat:cart:v1";
// The API takes at most 99 of one rental (Phase 37 Decision 2).
const RENTAL_MAX_QUANTITY = 99;

function capQuantity(kind: CartLineKind, quantity: number): number {
  return kind === "rental" ? Math.min(quantity, RENTAL_MAX_QUANTITY) : quantity;
}

function lineKey(kind: CartLineKind, itemId: number, priceOptionId: number | null): string {
  // Rentals get their own key space, so ids never collide with menu items.
  return kind === "rental" ? `rental:${itemId}` : `${itemId}:${priceOptionId ?? "flat"}`;
}

// Lines stored before Phase 36 have no minQuantity (read as 1), and before
// Phase 37 no kind (read as "menu").
type StoredCartLine = Omit<CartLine, "minQuantity" | "kind"> & { minQuantity?: number; kind?: CartLineKind };

function isCartLine(value: unknown): value is StoredCartLine {
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
    v.quantity > 0 &&
    (v.kind === undefined || v.kind === "menu" || v.kind === "rental") &&
    (v.minQuantity === undefined || (typeof v.minQuantity === "number" && v.minQuantity >= 1)) &&
    (v.names === undefined || isLocalizedText(v.names)) &&
    (v.optionLabels === undefined || v.optionLabels === null || isLocalizedText(v.optionLabels))
  );
}

function isLocalizedText(value: unknown): value is LocalizedText {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.en === "string" && typeof v.nl === "string" && typeof v.fr === "string";
}

// The line as the current language shows it (Phase 38 Decision 8).
function localizeLine(line: CartLine, locale: AppLocale): CartLine {
  return {
    ...line,
    name: line.names?.[locale] ?? line.name,
    optionLabel: line.optionLabels?.[locale] ?? line.optionLabel,
  };
}

function cartReducer(lines: CartLine[], action: CartAction): CartLine[] {
  switch (action.type) {
    case "hydrate":
      return action.lines;
    case "add": {
      const key = lineKey(action.line.kind, action.line.itemId, action.line.priceOptionId);
      const existing = lines.find((line) => line.key === key);
      if (existing) {
        // Refresh display fields too: an admin may have replaced the photo
        // (and deleted the old object) since this line was stored (Phase 19).
        return lines.map((line) =>
          line.key === key
            ? {
                ...line,
                name: action.line.name,
                names: action.line.names,
                optionLabels: action.line.optionLabels,
                imageUrl: action.line.imageUrl,
                minQuantity: action.line.minQuantity,
                quantity: capQuantity(line.kind, Math.max(line.quantity + action.quantity, action.line.minQuantity)),
              }
            : line,
        );
      }
      return [...lines, { ...action.line, key, quantity: Math.max(action.quantity, action.line.minQuantity) }];
    }
    case "setQuantity": {
      // Below the minimum (or 0) removes the line (Phase 36 Decision 3).
      const line = lines.find((candidate) => candidate.key === action.key);
      if (!line || action.quantity < Math.max(line.minQuantity, 1)) {
        return lines.filter((candidate) => candidate.key !== action.key);
      }
      return lines.map((line) =>
        line.key === action.key ? { ...line, quantity: capQuantity(line.kind, action.quantity) } : line,
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
  hasRentals: boolean;
  addLine: (line: NewCartLine, quantity?: number) => void;
  setQuantity: (key: string, quantity: number) => void;
  removeLine: (key: string) => void;
  clearCart: () => void;
}

const CartContext = createContext<CartContextValue | null>(null);

export function CartProvider({ children }: { children: ReactNode }) {
  const locale = useLocale() as AppLocale;
  const [storedLines, dispatch] = useReducer(cartReducer, []);
  const lines = useMemo(() => storedLines.map((line) => localizeLine(line, locale)), [storedLines, locale]);
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.every(isCartLine)) {
          dispatch({
            type: "hydrate",
            lines: parsed.map((line) => ({ ...line, minQuantity: line.minQuantity ?? 1, kind: line.kind ?? "menu" })),
          });
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
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(storedLines));
    } catch {
      // localStorage unavailable (quota, private browsing) — degrade to in-memory only.
    }
  }, [storedLines, isHydrated]);

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

  const hasRentals = useMemo(() => lines.some((line) => line.kind === "rental"), [lines]);

  const value = useMemo(
    () => ({ lines, itemCount, subtotal, hasRentals, addLine, setQuantity, removeLine, clearCart }),
    [lines, itemCount, subtotal, hasRentals, addLine, setQuantity, removeLine, clearCart],
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
