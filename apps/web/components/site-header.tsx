"use client";

import { useState, useSyncExternalStore } from "react";
import { useTranslations } from "next-intl";
import { motion, useReducedMotion } from "motion/react";
import { Link, usePathname } from "@/i18n/navigation";
import { Logo } from "@/components/logo";
import { LocaleToggle } from "@/components/locale-toggle";

const NAV_ITEMS = [
  { href: "/", key: "home" },
  { href: "/about", key: "about" },
  { href: "/services", key: "services" },
  { href: "/contact", key: "contact" },
] as const;

const DESKTOP_QUERY = "(min-width: 769px)";

function subscribeToDesktopQuery(callback: () => void) {
  const query = window.matchMedia(DESKTOP_QUERY);
  query.addEventListener("change", callback);
  return () => query.removeEventListener("change", callback);
}

function getIsDesktopSnapshot() {
  return window.matchMedia(DESKTOP_QUERY).matches;
}

export function SiteHeader() {
  const t = useTranslations("SiteHeader");
  const pathname = usePathname();
  const shouldReduceMotion = useReducedMotion();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const isDesktop = useSyncExternalStore(subscribeToDesktopQuery, getIsDesktopSnapshot, () => false);
  const [lastPathname, setLastPathname] = useState(pathname);

  if (pathname !== lastPathname) {
    setLastPathname(pathname);
    setIsMenuOpen(false);
  }

  return (
    <header className="site-header">
      <nav className="container" aria-label={t("navLabel")}>
        <Logo />
        <button
          className="hamburger"
          aria-expanded={isMenuOpen}
          aria-controls="primary-navigation"
          aria-label={t(isMenuOpen ? "closeMenu" : "openMenu")}
          onClick={() => setIsMenuOpen((open) => !open)}
        >
          <span aria-hidden="true">☰</span>
        </button>
        <motion.ul
          id="primary-navigation"
          inert={!isDesktop && !isMenuOpen}
          animate={{ height: isMenuOpen ? "auto" : 0, opacity: isMenuOpen ? 1 : 0 }}
          transition={{ duration: shouldReduceMotion ? 0 : 0.25, ease: [0.22, 1, 0.36, 1] }}
          className="primary-nav"
          style={{ overflow: "hidden" }}
        >
          {NAV_ITEMS.map((item) => (
            <li key={item.href}>
              <Link href={item.href}>{t(item.key)}</Link>
            </li>
          ))}
          <li><LocaleToggle /></li>
          <li>
            <Link href="/contact" role="button">
              {t("orderNow")}
            </Link>
          </li>
        </motion.ul>
      </nav>
    </header>
  );
}
