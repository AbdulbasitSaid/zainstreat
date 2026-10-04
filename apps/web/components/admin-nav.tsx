"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

// `enabled: false` renders a muted, non-focusable span instead of a link —
// requirement.md Decision 2.
const NAV_ITEMS = [
  { label: "Dashboard", href: "/admin", enabled: true },
  { label: "Orders", href: "/admin/orders", enabled: true },
  { label: "Menu", href: "/admin/menu", enabled: true },
  { label: "Categories", href: "/admin/categories", enabled: true },
  { label: "Enquiries", href: "/admin/enquiries", enabled: true },
  { label: "Settings", href: "/admin/settings", enabled: false },
] as const;

const ITEM_CLASSES =
  "flex items-center justify-between rounded-xl px-4 py-2.5 text-sm font-semibold";

function isActive(pathname: string, href: string) {
  return href === "/admin" ? pathname === "/admin" : pathname.startsWith(href);
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
