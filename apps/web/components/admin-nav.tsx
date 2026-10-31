"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

// `enabled: false` renders a muted, non-focusable span instead of a link —
// requirement.md Decision 2.
const NAV_ITEMS = [
  { label: "Dashboard", href: "/", enabled: true },
  { label: "Orders", href: "/orders", enabled: true },
  { label: "Menu", href: "/menu", enabled: true },
  { label: "Categories", href: "/categories", enabled: true },
  { label: "Enquiries", href: "/enquiries", enabled: true },
  { label: "Settings", href: "/settings", enabled: false },
] as const;

const ITEM_CLASSES =
  "flex items-center justify-between rounded-xl px-4 py-2.5 text-sm font-semibold";

// The admin is served from cms.<host>, rewritten onto app/admin/** by
// proxy.ts. Depending on render pass, usePathname() may report either the
// browser path (/orders) or the rewritten one (/admin/orders), so strip
// the prefix before comparing.
function isActive(pathname: string, href: string) {
  const path = pathname.replace(/^\/admin(?=\/|$)/, "") || "/";
  return href === "/" ? path === "/" : path.startsWith(href);
}

export function AdminNav() {
  const pathname = usePathname();

  return (
    <nav aria-label="Admin sections" className="flex flex-col gap-1">
      {NAV_ITEMS.map((item) =>
        item.enabled ? (
          <Link
            key={item.href}
            href={item.href}
            aria-current={isActive(pathname, item.href) ? "page" : undefined}
            className={`${ITEM_CLASSES} transition-colors duration-150 ease-out-expo focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              isActive(pathname, item.href)
                ? "bg-primary text-white"
                : "text-text hover:bg-background-soft"
            }`}
          >
            {item.label}
          </Link>
        ) : (
          <span key={item.href} aria-disabled="true" className={`${ITEM_CLASSES} text-text-muted opacity-50`}>
            {item.label}
            <span className="text-[0.65rem] font-semibold uppercase tracking-wider">Soon</span>
          </span>
        ),
      )}
    </nav>
  );
}
