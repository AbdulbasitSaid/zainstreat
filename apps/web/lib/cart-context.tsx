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
