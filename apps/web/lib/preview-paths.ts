import { routing, type AppLocale } from "@/i18n/routing";

// The public pages the device simulator can show (Phase 25 Decision 24).
// Cart, order and quote routes are deliberately absent.
export const PREVIEW_PAGES = [
  { path: "", label: "Home" },
  { path: "/menu", label: "Menu" },
  { path: "/about", label: "About" },
  { path: "/services", label: "Services" },
  { path: "/contact", label: "Contact" },
] as const;

const PREVIEW_PATHS: readonly string[] = PREVIEW_PAGES.map((page) => page.path);

export function isPreviewLocale(value: string | null): value is AppLocale {
  return value !== null && (routing.locales as readonly string[]).includes(value);
}

/**
 * Normalizes `path` (e.g. `/menu?category=3`) to an allowlisted public path,
 * or `null`. The allowlist also rules out `//host` open redirects.
 */
export function normalizePreviewPath(path: string | null): string | null {
  const raw = path ?? "";
  if (raw !== "" && !raw.startsWith("/")) return null;

  const [pathname, query = ""] = raw.split("?", 2);
  const normalized = pathname === "/" ? "" : pathname;
  if (!PREVIEW_PATHS.includes(normalized)) return null;
  if (query === "") return normalized;

  const category = /^category=(\d{1,9})$/.exec(query);
  return normalized === "/menu" && category ? `/menu?category=${category[1]}` : null;
}

export function previewEnableUrl(locale: AppLocale, path: string): string {
  return `/api/preview/enable?${new URLSearchParams({ locale, path }).toString()}`;
}
