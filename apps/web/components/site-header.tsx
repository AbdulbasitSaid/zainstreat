"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { Link, usePathname } from "@/i18n/navigation";
import { Logo } from "@/components/logo";
import { LocaleToggle } from "@/components/locale-toggle";

const NAV_ITEMS = [
  { href: "/", key: "home" },
  { href: "/about", key: "about" },
  { href: "/services", key: "services" },
  { href: "/contact", key: "contact" },
] as const;

export function SiteHeader() {
  const t = useTranslations("SiteHeader");
  const pathname = usePathname();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
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
        <ul id="primary-navigation" hidden={!isMenuOpen} className="primary-nav">
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
        </ul>
      </nav>
    </header>
  );
}
