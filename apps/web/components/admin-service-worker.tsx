"use client";

import { useEffect } from "react";

// Registers public/sw.js so the admin can be installed (Phase 27 Decision 5).
// Only on the cms host: the public site never gets the admin's worker.
export function AdminServiceWorker() {
  useEffect(() => {
    if (!("serviceWorker" in navigator) || !window.location.hostname.startsWith("cms.")) return;
    navigator.serviceWorker
      .register("/sw.js", { scope: "/", updateViaCache: "none" })
      .catch((error: unknown) => console.warn("Service worker registration failed", error));
  }, []);

  return null;
}
