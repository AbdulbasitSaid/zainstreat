"use client";

import { useEffect } from "react";

// The Badging API isn't in every browser (Android Chrome lacks it).
interface NavigatorBadge {
  setAppBadge?: (contents?: number) => Promise<void>;
  clearAppBadge?: () => Promise<void>;
}

// Registers public/sw.js so the admin can be installed and receive push
// (Phase 27 Decision 5). Only on the cms host: the public site never gets
// the admin's worker. Also re-syncs the app icon badge with the orders
// needing action on every admin page load (Decision 11).
export function AdminServiceWorker({ badgeCount }: { badgeCount: number | null }) {
  useEffect(() => {
    if (!("serviceWorker" in navigator) || !window.location.hostname.startsWith("cms.")) return;
    navigator.serviceWorker
      .register("/sw.js", { scope: "/", updateViaCache: "none" })
      .catch((error: unknown) => console.warn("Service worker registration failed", error));
  }, []);

  useEffect(() => {
    if (badgeCount === null) return;
    const nav: Navigator & NavigatorBadge = navigator;
    const update = badgeCount > 0 ? nav.setAppBadge?.(badgeCount) : nav.clearAppBadge?.();
    update?.catch(() => {});
  }, [badgeCount]);

  return null;
}
