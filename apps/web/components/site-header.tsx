"use client";

import { useRef, useState, useSyncExternalStore } from "react";
import { useTranslations } from "next-intl";
import { Link, usePathname } from "@/i18n/navigation";
import { gsap, useGSAP } from "@/lib/gsap";
import { useReducedMotion } from "@/lib/use-reduced-motion";
import { Logo } from "@/components/logo";
import { LocaleToggle } from "@/components/locale-toggle";
import { ButtonLink } from "@/components/button";
import { CartIcon } from "@/components/cart-icon";
import { CartBadge } from "@/components/cart-badge";
import { buildWhatsAppLink } from "@/lib/whatsapp";

const NAV_ITEMS = [
  { href: "/", key: "home" },
  { href: "/about", key: "about" },
  { href: "/services", key: "services" },
  { href: "/menu", key: "menu" },
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

function WhatsAppIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden="true"
      className="h-5 w-5"
    >
      <path d="M7 16.5 4.5 19l.9-3.3A8 8 0 1 1 7 16.5Z" />
      <path d="M9 10.3c0 2.6 2.1 4.7 4.7 4.7" strokeLinecap="round" />
    </svg>
  );
}

export function SiteHeader() {
  const t = useTranslations("SiteHeader");
  const tw = useTranslations("Whatsapp");
  const pathname = usePathname();
  const shouldReduceMotion = useReducedMotion();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const isDesktop = useSyncExternalStore(subscribeToDesktopQuery, getIsDesktopSnapshot, () => false);
  const [lastPathname, setLastPathname] = useState(pathname);

  if (pathname !== lastPathname) {
    setLastPathname(pathname);
    setIsMenuOpen(false);
  }

  const bar1Ref = useRef<HTMLSpanElement>(null);
  const bar2Ref = useRef<HTMLSpanElement>(null);
  const bar3Ref = useRef<HTMLSpanElement>(null);
  const navListRef = useRef<HTMLUListElement>(null);
  const indicatorRef = useRef<HTMLSpanElement>(null);
  const navItemRefs = useRef(new Map<string, HTMLLIElement>());

  useGSAP(
    () => {
      const duration = shouldReduceMotion ? 0 : 0.2;
      const ease = "outExpo";
      gsap.to(bar1Ref.current, { rotate: isMenuOpen ? 45 : 0, y: isMenuOpen ? 6 : 0, duration, ease });
      gsap.to(bar2Ref.current, { opacity: isMenuOpen ? 0 : 1, duration, ease });
      gsap.to(bar3Ref.current, { rotate: isMenuOpen ? -45 : 0, y: isMenuOpen ? -6 : 0, duration, ease });
      gsap.to(navListRef.current, {
        height: isMenuOpen ? "auto" : 0,
        opacity: isMenuOpen ? 1 : 0,
        duration: shouldReduceMotion ? 0 : 0.25,
        ease,
      });
    },
    { dependencies: [isMenuOpen, shouldReduceMotion] },
  );

  function moveIndicatorTo(href: string) {
    const indicator = indicatorRef.current;
    const item = navItemRefs.current.get(href);
    const list = navListRef.current;
    if (!indicator || !item || !list) return;

    const itemRect = item.getBoundingClientRect();
    const listRect = list.getBoundingClientRect();
    gsap.to(indicator, {
      x: itemRect.left - listRect.left,
      width: itemRect.width,
      duration: shouldReduceMotion ? 0 : 0.3,
      ease: "outExpo",
    });
  }

  useGSAP(
    () => {
      if (!isDesktop) return;
      moveIndicatorTo(pathname);
    },
    { dependencies: [isDesktop, pathname, shouldReduceMotion] },
  );

  return (
    <header className="sticky top-0 z-50 bg-cream/90 shadow-[0_1px_0_rgba(43,17,20,0.08)] backdrop-blur-md">
      <nav className="container flex items-center justify-between py-3.5" aria-label={t("navLabel")}>
        <Logo />
        <button
          className="relative z-10 inline-flex h-10 w-10 flex-col items-center justify-center gap-1.5 rounded-full text-primary min-[769px]:hidden"
          aria-expanded={isMenuOpen}
          aria-controls="primary-navigation"
          aria-label={t(isMenuOpen ? "closeMenu" : "openMenu")}
          onClick={() => setIsMenuOpen((open) => !open)}
        >
          <span ref={bar1Ref} aria-hidden="true" className="block h-0.5 w-6 rounded-full bg-current" />
          <span ref={bar2Ref} aria-hidden="true" className="block h-0.5 w-6 rounded-full bg-current" />
          <span ref={bar3Ref} aria-hidden="true" className="block h-0.5 w-6 rounded-full bg-current" />
        </button>
        <ul
          ref={navListRef}
          id="primary-navigation"
          inert={!isDesktop && !isMenuOpen}
          style={{ overflow: "hidden", height: 0, opacity: 0 }}
          onMouseLeave={() => isDesktop && moveIndicatorTo(pathname)}
          className="absolute inset-x-0 top-full flex flex-col gap-1 rounded-b-3xl bg-cream px-4 pb-4 shadow-lg min-[769px]:!relative min-[769px]:!h-auto min-[769px]:!opacity-100 min-[769px]:flex-row min-[769px]:items-center min-[769px]:gap-7 min-[769px]:rounded-none min-[769px]:bg-transparent min-[769px]:p-0 min-[769px]:shadow-none"
        >
          {isDesktop && (
            <span
              ref={indicatorRef}
              aria-hidden="true"
              className="pointer-events-none absolute bottom-0 left-0 h-0.5 bg-primary"
            />
          )}
          {NAV_ITEMS.map((item) => {
            const isActive = pathname === item.href;
            return (
              <li
                key={item.href}
                ref={(node) => {
                  if (node) navItemRefs.current.set(item.href, node);
                  else navItemRefs.current.delete(item.href);
                }}
                onMouseEnter={() => isDesktop && moveIndicatorTo(item.href)}
              >
                <Link
                  href={item.href}
                  aria-current={isActive ? "page" : undefined}
                  className={`block rounded-xl px-4 py-3 text-base font-semibold tracking-wide transition-colors min-[769px]:bg-transparent min-[769px]:px-0 min-[769px]:py-1 min-[769px]:text-sm min-[769px]:hover:text-primary ${
                    isActive ? "text-primary" : "text-text hover:bg-background-soft min-[769px]:hover:bg-transparent"
                  }`}
                >
                  {t(item.key)}
                </Link>
              </li>
            );
          })}
          <li className="px-4 py-2 min-[769px]:px-0 min-[769px]:py-0">
            <Link
              href="/cart"
              aria-label={t("cart")}
              className="relative inline-flex h-10 w-10 items-center justify-center rounded-full text-primary hover:bg-background-soft"
            >
              <CartIcon className="h-5 w-5" />
              <CartBadge />
            </Link>
          </li>
          <li className="px-4 py-2 min-[769px]:px-0 min-[769px]:py-0">
            <LocaleToggle />
          </li>
          <li className="px-4 py-2 min-[769px]:px-0 min-[769px]:py-0">
            <a
              href={buildWhatsAppLink(tw("prefilledMessage"))}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={t("whatsappLabel")}
              className="inline-flex h-10 w-10 items-center justify-center rounded-full text-whatsapp-dark hover:bg-background-soft"
            >
              <WhatsAppIcon />
            </a>
          </li>
          <li className="px-4 pt-2 min-[769px]:px-0 min-[769px]:pt-0">
            <ButtonLink href="/contact" className="w-full justify-center min-[769px]:w-auto">
              {t("orderNow")}
            </ButtonLink>
          </li>
        </ul>
      </nav>
    </header>
  );
}
