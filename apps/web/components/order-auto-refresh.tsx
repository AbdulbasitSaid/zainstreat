"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

const INTERVAL_MS = 60_000;

// Keeps the tracking page current while it's open (Phase 22 Decision 10).
// Only polls while the tab is visible, and catches up when it becomes visible
// again. The page stops rendering this once the order is finished.
export function OrderAutoRefresh() {
  const router = useRouter();

  useEffect(() => {
    const refreshIfVisible = () => {
      if (document.visibilityState === "visible") router.refresh();
    };
    const id = window.setInterval(refreshIfVisible, INTERVAL_MS);
    document.addEventListener("visibilitychange", refreshIfVisible);
    return () => {
      window.clearInterval(id);
      document.removeEventListener("visibilitychange", refreshIfVisible);
    };
  }, [router]);

  return null;
}
