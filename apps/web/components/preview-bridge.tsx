"use client";

import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";
import { routing } from "@/i18n/routing";

const PREVIEW_PREFIX = "/preview";

/** Messages between the admin's device simulator and a preview frame. */
export type PreviewMessage = { type: "zt:refresh" } | { type: "zt:path"; path: string };

function isLocalePath(path: string): boolean {
  return routing.locales.some((locale) => path === `/${locale}` || path.startsWith(`/${locale}/`));
}

/**
 * Rendered only in draft mode (Phase 25 Decisions 22, 25), inside the
 * simulator's same-origin iframe on the cms host:
 * - `zt:refresh` from the admin re-renders the server components with the
 *   latest draft, keeping scroll position;
 * - site links stay under /preview/…, and the current path is reported to
 *   the parent so its page picker follows.
 */
export function PreviewBridge() {
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    function onMessage(event: MessageEvent<PreviewMessage>) {
      if (event.origin !== window.location.origin) return;
      if (event.data?.type === "zt:refresh") router.refresh();
    }

    function onClick(event: MouseEvent) {
      if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey) return;
      const anchor = (event.target as Element | null)?.closest("a");
      if (!anchor || !anchor.href) return;

      const url = new URL(anchor.href, window.location.href);
      if (url.origin !== window.location.origin) {
        event.preventDefault();
        window.open(url.href, "_blank", "noopener,noreferrer");
        return;
      }
      if (url.pathname.startsWith(PREVIEW_PREFIX) || !isLocalePath(url.pathname)) return;
      // Same page, different hash: let the browser scroll.
      if (url.hash && url.pathname === window.location.pathname.slice(PREVIEW_PREFIX.length)) return;

      event.preventDefault();
      router.push(`${PREVIEW_PREFIX}${url.pathname}${url.search}${url.hash}`);
    }

    window.addEventListener("message", onMessage);
    document.addEventListener("click", onClick, true);
    return () => {
      window.removeEventListener("message", onMessage);
      document.removeEventListener("click", onClick, true);
    };
  }, [router]);

  // `pathname` only triggers this: depending on the render pass it can be
  // the browser's /preview/… path or the rewritten one, so read the
  // browser's and report it without the prefix.
  useEffect(() => {
    if (window.parent === window) return;
    const path = window.location.pathname.replace(/^\/preview(?=\/)/, "");
    const message: PreviewMessage = { type: "zt:path", path: `${path}${window.location.search}` };
    window.parent.postMessage(message, window.location.origin);
  }, [pathname]);

  return (
    <div className="pointer-events-none fixed bottom-3 left-1/2 z-[100] -translate-x-1/2 rounded-full bg-primary-dark px-4 py-1.5 text-xs font-semibold tracking-wide text-text-light shadow-lg">
      Preview — ordering disabled
    </div>
  );
}
