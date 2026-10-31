"use client";

import { createContext, useContext, useEffect, useMemo, useSyncExternalStore, type ReactNode } from "react";
import { useRouter } from "next/navigation";

const INTERVAL_MS = 30_000;
const BASELINE_KEY = "zainstreat:admin-badge-baseline";
const TITLE_BADGE = /^\(\d+\) /;

interface Baseline {
  latestOrderId: number;
  claimedIds: number[];
}

// What the owner had seen when they last looked at this tab. Kept in
// sessionStorage so a reload doesn't lose the count, with an in-memory copy
// for when storage is blocked (Phase 22 Decision 9). Read through
// useSyncExternalStore, like order-view.tsx's last order.
let memoryBaseline: string | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getBaselineSnapshot(): string | null {
  try {
    return window.sessionStorage.getItem(BASELINE_KEY) ?? memoryBaseline;
  } catch {
    return memoryBaseline;
  }
}

function getServerBaselineSnapshot(): string | null {
  return null;
}

function saveBaseline(baseline: Baseline) {
  memoryBaseline = JSON.stringify(baseline);
  try {
    window.sessionStorage.setItem(BASELINE_KEY, memoryBaseline);
  } catch {
    // Storage unavailable: the in-memory copy still works until a reload.
  }
  listeners.forEach((listener) => listener());
}

const NewOrdersContext = createContext<ReadonlySet<number>>(new Set());

// Keeps the dashboard and orders list current while admin is open, and
// counts what arrived since the owner last looked in the tab title
// (Phase 22 Decision 9). Polls even while hidden: a background tab is when
// the badge matters. Real push is Phase 27.
export function AdminAutoRefresh({
  recentOrderIds,
  claimedIds,
  children,
}: {
  // Newest first, from the orders summary.
  recentOrderIds: number[];
  claimedIds: number[];
  children: ReactNode;
}) {
  const router = useRouter();
  const latestOrderId = recentOrderIds[0] ?? 0;

  const baselineRaw = useSyncExternalStore(subscribe, getBaselineSnapshot, getServerBaselineSnapshot);
  const baseline = useMemo<Baseline | null>(() => {
    if (!baselineRaw) return null;
    try {
      return JSON.parse(baselineRaw) as Baseline;
    } catch {
      return null;
    }
  }, [baselineRaw]);

  // First visit in this tab: what's on screen now counts as seen.
  useEffect(() => {
    if (getBaselineSnapshot() === null) saveBaseline({ latestOrderId, claimedIds });
  }, [latestOrderId, claimedIds]);

  // Looking at the tab means everything on it has been seen.
  useEffect(() => {
    const handleFocus = () => {
      saveBaseline({ latestOrderId, claimedIds });
      router.refresh();
    };
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [router, latestOrderId, claimedIds]);

  useEffect(() => {
    const id = window.setInterval(() => router.refresh(), INTERVAL_MS);
    return () => window.clearInterval(id);
  }, [router]);

  const newOrderIds = baseline ? recentOrderIds.filter((id) => id > baseline.latestOrderId) : [];
  const newClaimIds = baseline ? claimedIds.filter((id) => !baseline.claimedIds.includes(id)) : [];
  const highlighted = new Set([...newOrderIds, ...newClaimIds]);
  const count = newOrderIds.length + newClaimIds.length;

  // Re-applied after every render: a refresh can reset the title from the
  // page's metadata.
  useEffect(() => {
    const title = document.title.replace(TITLE_BADGE, "");
    document.title = count > 0 ? `(${count}) ${title}` : title;
  });

  return <NewOrdersContext.Provider value={highlighted}>{children}</NewOrdersContext.Provider>;
}

const HIGHLIGHT_CLASSES = "bg-accent-light/60 ring-2 ring-inset ring-primary";

// A row or list item that lights up when its order is new since the owner
// last looked (Phase 22 Decision 9).
export function NewOrderHighlight({
  as: Tag,
  orderId,
  className,
  children,
}: {
  as: "tr" | "li";
  orderId: number;
  className?: string;
  children: ReactNode;
}) {
  const isNew = useContext(NewOrdersContext).has(orderId);
  return <Tag className={`${className ?? ""} ${isNew ? HIGHLIGHT_CLASSES : ""}`.trim()}>{children}</Tag>;
}
