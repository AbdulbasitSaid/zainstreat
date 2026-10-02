# Phase 5 — Public Static Pages: Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed and
sanity-checked before moving to the next. All work is in `apps/web` — no
`apps/api`/deployment files are touched.

## Group 0 — Branch

0.1. Create branch `2026-10-02-phase-5-public-static-pages` off `master`
     (done).

No new npm dependency to install for brand/CSS work — `@picocss/pico@^2`
is already a dependency (Phase 1) and already imported in
`apps/web/app/globals.css`. This phase only adds a brand override layer
on top of it, plus `next/font/google` (built into `next`, no install
needed).

## Group 1 — Brand tokens (Pico CSS custom properties)

Depends on: Group 0.

1.1. `apps/web/app/globals.css` — keep the existing `@import`, then append
     a brand override block, reusing README §3's palette verbatim and
     remapping the subset of tokens Pico itself reads
     (`--pico-primary*`) so built-in Pico components (buttons, links,
     form focus rings) inherit the brand automatically:

```css
@import "@picocss/pico/css/pico.min.css";

:root {
  color-scheme: light; /* force light theme — food photography on a white
    background is the brand direction (README §3, §34); don't let a
    visitor's OS dark-mode setting invert it. Do not add a dark-mode
    toggle component in this phase. */

  /* Brand palette (README §3) */
  --color-primary: #A7041B;
  --color-primary-dark: #5E0407;
  --color-primary-light: #E2073E;
  --color-accent: #F93173;
  --color-accent-light: #F4B7C7;
  --color-background: #FFFFFF;
  --color-background-soft: #FAE8EA;
  --color-text: #2B1114;
  --color-text-muted: #6F4A4F;
  --color-text-light: #FFFFFF;
  --color-whatsapp: #1DA332;

  /* Remap onto Pico's own tokens so default Pico components inherit */
  --pico-primary: var(--color-primary);
  --pico-primary-background: var(--color-primary);
  --pico-primary-border: var(--color-primary);
  --pico-primary-underline: rgba(167, 4, 27, 0.5);
  --pico-primary-hover: var(--color-primary-dark);
  --pico-primary-hover-background: var(--color-primary-light);
  --pico-primary-hover-border: var(--color-primary-light);
  --pico-primary-hover-underline: var(--color-primary-dark);
  --pico-primary-focus: rgba(249, 49, 115, 0.375);
  --pico-primary-inverse: #fff;
}
```
     (Pico ships `prefers-color-scheme: dark` and `[data-theme=dark]`
     blocks; neither triggers now that `:root` pins `color-scheme: light`
     and no `data-theme` toggle is ever added.)

1.2. Typography — self-hosted `next/font/google` display + body pairing,
     per README §34's "strong display heading / clean sans-serif body"
     direction (exact typefaces are a low-stakes, reversible default):
```typescript
// apps/web/app/[locale]/layout.tsx
import { Fraunces, Work_Sans } from "next/font/google";

const displayFont = Fraunces({
  subsets: ["latin"],
  variable: "--font-display",
  weight: ["600", "700"],
});
const bodyFont = Work_Sans({
  subsets: ["latin"],
  variable: "--font-body",
});
```
     Apply `className={`${displayFont.variable} ${bodyFont.variable}`}`
     on `<html>`, then in `globals.css`:
```css
body { font-family: var(--font-body), sans-serif; color: var(--color-text); background: var(--color-background); }
h1, h2, h3, h4, h5, h6 { font-family: var(--font-display), serif; }
```

## Group 2 — Logo placeholder

Depends on: Group 1.

2.1. `apps/web/components/logo.tsx` — a server component rendering a
     styled text wordmark now (no real logo file exists in the repo —
     see `requirement.md` Decision 1), structured so swapping in a real
     `public/logo.svg`/`.png` later is a one-line change:
```typescript
import { Link } from "@/i18n/navigation";

export function Logo() {
  return (
    <Link href="/" className="logo" aria-label="Zain's Treat n More — Home">
      Zain&apos;s Treat n More
    </Link>
  );
}
```
2.2. `globals.css` addition:
```css
.logo { font-family: var(--font-display), serif; font-weight: 700; font-size: 1.25rem; color: var(--color-primary); text-decoration: none; }
```

## Group 3 — Site header / navigation (desktop + mobile hamburger)

Depends on: Groups 1, 2.

3.1. `apps/web/components/site-header.tsx` — `"use client"` (needs
     `useState` for the hamburger toggle and `usePathname` to close the
     menu on navigation):
```typescript
"use client";

import { useEffect, useState } from "react";
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

  useEffect(() => {
    setIsMenuOpen(false);
  }, [pathname]);

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
```
     Notes on mechanics:
     - The mobile/desktop split is pure CSS, not conditional rendering:
       `<ul id="primary-navigation" hidden={!isMenuOpen}>` is driven by
       the `hidden` attribute (removes it from the accessibility tree too
       when collapsed, not just `display:none`). A
       `@media (min-width: 769px)` rule (3.2) forces it visible regardless
       of `hidden` on desktop and hides `.hamburger` entirely above that
       breakpoint — JS state only matters on narrow viewports.
     - `role="button"` on the `Order Now` `<Link>` is required for Pico
       CSS to style an anchor as a button (Pico only button-styles
       `<button>` and `[role=button]`, not bare `<a>`).
     - Closing the menu on route change (`useEffect` keyed on `pathname`)
       prevents a stale open menu after navigating.
     - `aria-expanded`/`aria-controls`/`aria-label` give a working
       disclosure control with no extra JS library.
     - `Order Now` links to `/contact` for now (no cart/order flow exists
       until Phases 6–8); same destination as the Services/Catering CTAs.

3.2. `apps/web/app/globals.css` — append responsive rules:
```css
.site-header { background: var(--color-background); border-bottom: 1px solid var(--color-accent-light); }
.site-header nav { display: flex; align-items: center; justify-content: space-between; }
.hamburger { display: inline-flex; background: none; border: none; font-size: 1.5rem; color: var(--color-primary); }
#primary-navigation { flex-direction: column; }
#primary-navigation[hidden] { display: none; }

@media (min-width: 769px) {
  .hamburger { display: none; }
  #primary-navigation,
  #primary-navigation[hidden] { display: flex !important; flex-direction: row; align-items: center; gap: 1rem; }
}
```

3.3. Relocate `apps/web/components/locale-toggle.tsx`'s usage: remove it
     from `app/[locale]/page.tsx` and render it only inside
     `<SiteHeader>` as shown above (closes Phase 3's forward-reference in
     `docs/local-development.md`). The component file itself is
     unchanged.

## Group 4 — Site footer

Depends on: Group 1.

4.1. `apps/web/components/site-footer.tsx` — server component:
```typescript
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export function SiteFooter() {
  const t = useTranslations("SiteFooter");
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container">
        <p>{t("tagline")}</p>
        <nav aria-label={t("footerNavLabel")}>
          <ul>
            <li><Link href="/about">{t("about")}</Link></li>
            <li><Link href="/services">{t("services")}</Link></li>
            <li><Link href="/contact">{t("contact")}</Link></li>
            <li><Link href="/terms">{t("terms")}</Link></li>
            <li><Link href="/food-regulations">{t("foodRegulations")}</Link></li>
          </ul>
        </nav>
        <p>{t("copyright", { year })}</p>
      </div>
    </footer>
  );
}
```
4.2. `globals.css` addition:
```css
.site-footer { background: var(--color-primary-dark); color: var(--color-text-light); padding: 2rem 0; }
.site-footer a { color: inherit; }
```
     (README §3 pins `primary-dark` for "dark sections, footer,
     headings" — this is the one place the dark burgundy becomes a
     background, not the whole site, consistent with the "don't make the
     entire website red/pink" rule.)

## Group 5 — Shared locale layout wiring

Depends on: Groups 3, 4.

5.1. `apps/web/app/[locale]/layout.tsx` — wrap `{children}` with the new
     header/footer and apply the font variables from Group 1.2:
```typescript
import type { Metadata } from "next";
import { NextIntlClientProvider, hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { Fraunces, Work_Sans } from "next/font/google";
import { routing } from "@/i18n/routing";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import "../globals.css";

const displayFont = Fraunces({ subsets: ["latin"], variable: "--font-display", weight: ["600", "700"] });
const bodyFont = Work_Sans({ subsets: ["latin"], variable: "--font-body" });

export const metadata: Metadata = {
  title: "Zain's Treat n More",
};

export function generateStaticParams() {
  return routing.locales.map((locale) => ({ locale }));
}

export default async function LocaleLayout({ children, params }: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  if (!hasLocale(routing.locales, locale)) notFound();

  return (
    <html lang={locale} className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body>
        <NextIntlClientProvider>
          <SiteHeader />
          {children}
          <SiteFooter />
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
```

## Group 6 — Page routes

Depends on: Group 5.

6.1. `apps/web/app/[locale]/page.tsx` — rebuild as the real homepage (no
     dynamic menu data — Phase 6's job). Sections, per README §10/§35,
     limited to what needs no DB:
     - Hero (`HomePage.heroHeadline`/`heroSupporting`, `Order Now` +
       `Book Catering` + optional `WhatsApp Us` CTAs, all linking to
       `/contact` for now).
     - Trust indicators row (README §10: 100% Halal / Fresh Ingredients /
       Hygienically Prepared / Reliable Service).
     - Services overview (static cards: Meals/Snacks/Catering/Event
       Rentals, each linking to `/services`).
     - "Why Choose Us" (short static copy).
     - Catering/Events CTA banner linking to `/contact`.
     - About preview (short teaser + link to `/about`).
     - *Omit* "Featured Menu" entirely — Phase 6 inserts it once menu data
       exists; do not stub it with fake items.
6.2. `apps/web/app/[locale]/about/page.tsx` — Who We Are / Our Mission /
     Our Values / What We Offer (README §11), static copy, each in its
     own `<section>`.
6.3. `apps/web/app/[locale]/services/page.tsx` — Meals / Snacks /
     Catering / Event Rentals detail sections (README §12), each with a
     description and, for Catering and Event Rentals, a "Request a
     Quote"/"Request Catering Quote" CTA linking to `/contact` (no
     dedicated catering-quote route exists until Phase 12).
6.4. `apps/web/app/[locale]/contact/page.tsx` — structural shell:
     - Contact info block (phone/email/WhatsApp/address/hours/social)
       rendered from message keys that are explicit placeholders until
       real business details are supplied (requirement.md Decision 3) —
       e.g. `{t("phoneLabel")}: {t("phonePlaceholder")}`.
     - Contact form markup (Name/Email/Phone/Subject dropdown/Message —
       README §22) with **no submit handler wired to the API** (Phase
       12's job) — submit button `disabled`, with a short note:
       `{t("formComingSoon")}` ("This form isn't connected yet — please
       reach us on WhatsApp or by email in the meantime.").
     - No map embed in this phase (no real address yet to plot) — a
       placeholder `<div>` with `aria-label` reserving the layout slot.
6.5. `apps/web/app/[locale]/terms/page.tsx` — one `<section>` per README
     §23 topic (Ordering, Payment, Cancellation, Refunds, Delivery/
     pickup, Catering orders, Event bookings, Customer responsibilities,
     Website usage, Changes to services/prices), each with templated
     generic copy and a visible banner: `{t("legalReviewNotice")}`
     ("Draft terms — pending review by a legal professional before
     go-live.").
6.6. `apps/web/app/[locale]/food-regulations/page.tsx` — Halal / Food
     Hygiene / Ingredients / Allergens sections (README §24), including
     the sample allergen list and the "please inform us of allergies"
     callout, adapted from README (generic copy — no real
     certification/hygiene-practice specifics exist yet, per
     requirement.md Decision 3).

## Group 7 — i18n message keys

Depends on: Group 6. Every new key added to **both**
`apps/web/messages/en.json` and `apps/web/messages/nl.json` in the same
commit (established convention — never ship an English-only key). Dutch
copy is best-effort/placeholder per requirement.md Decision 4, not a
professionally reviewed translation.

7.1. New top-level namespaces, `PageName.keyName` convention:
     - `SiteHeader`: `home`, `about`, `services`, `contact`, `orderNow`,
       `navLabel`, `openMenu`, `closeMenu`.
     - `SiteFooter`: `tagline`, `about`, `services`, `contact`, `terms`,
       `foodRegulations`, `copyright`, `footerNavLabel`.
     - `HomePage` (extend existing): `heroHeadline`, `heroSupporting`,
       `orderNow`, `bookCatering`, `whatsappUs`, `trustHalal`,
       `trustFresh`, `trustHygienic`, `trustReliable`, `servicesMeals`,
       `servicesSnacks`, `servicesCatering`, `servicesEventRentals`,
       `whyChooseUs`, `cateringCtaHeadline`, `requestCateringQuote`,
       `aboutPreview`, `learnMore`.
     - `AboutPage`: `whoWeAre`, `ourMission`, `missionCopy`, `ourValues`,
       `value1`–`value6`, `whatWeOffer`, `offer1`–`offer5`.
     - `ServicesPage`: `heading`, `mealsHeading`, `mealsCopy`,
       `snacksHeading`, `snacksCopy`, `cateringHeading`, `cateringCopy`,
       `requestCateringQuote`, `eventRentalsHeading`, `eventRentalsCopy`,
       `requestQuote`.
     - `ContactPage`: `phoneLabel`, `phonePlaceholder`, `emailLabel`,
       `whatsappLabel`, `addressLabel`, `addressPlaceholder`,
       `hoursLabel`, `hoursPlaceholder`, `formName`, `formEmail`,
       `formPhone`, `formSubject`, `formMessage`,
       `formSubjectGeneral`/`formSubjectCatering`/`formSubjectEventRental`/
       `formSubjectMenu`/`formSubjectOrder`/`formSubjectOther`,
       `sendMessage`, `formComingSoon`.
     - `TermsPage`: `legalReviewNotice` + one `heading`/`copy` pair per
       topic section.
     - `FoodRegulationsPage`: `halalHeading`, `halalCopy`,
       `hygieneHeading`, `hygieneCopy`, `ingredientsHeading`,
       `ingredientsCopy`, `allergensHeading`, `allergensIntro`,
       `allergensList`, `allergenMilk`...`allergenSoy`.
7.2. English copy can lift near-verbatim from README §§2, 11, 12, 22–24
     (the repo already treats README as the approved content source).

## Group 8 — Docs

Depends on: Group 7 (so the convention note reflects what actually
shipped).

8.1. `docs/local-development.md` — update the existing "Internationalization
     (Dutch/English)" section: note that `locale-toggle.tsx` has now
     moved into `site-header.tsx` (closing Phase 3's forward-reference),
     and that `SiteHeader`/`SiteFooter`/`PageName` namespaces are the
     established message-key groups going forward.

## Group 9 — Verification

See `validation.md` for the full pass/fail checklist.

## Group 10 — Motion/animation system (addendum)

Added after the phase's original static shell shipped — see
`requirement.md`'s "Addendum — motion/animation system". Depends on
Groups 1–7 (brand tokens, header/footer, layout, and all page routes
already exist).

10.1. `apps/web/package.json` — add `motion` and `lenis` as dependencies;
      `pnpm add motion lenis` inside `apps/web`.

10.2. `apps/web/app/globals.css` — add motion tokens
      (`--transition-fast/base/slow`, `--ease-out-expo`) to `:root`; add
      hover/active/focus-visible CSS for `a[role="button"]`/`button`/links;
      add `.decorative-shape`/`.decorative-shape--circle`/
      `@keyframes float-rotate`; add `.image-slot`; add the
      `prefers-reduced-motion: reduce` blanket safety net; update
      `#primary-navigation`'s base/desktop rules for the `inert`-based
      hamburger (see 10.4).

10.3. New components in `apps/web/components/`:
      - `motion-provider.tsx` (`"use client"`) — mounts/tears down Lenis,
        no-ops under reduced motion.
      - `reveal.tsx` (`"use client"`) — `<Reveal stagger?>` scroll-triggered
        fade-up wrapper.
      - `decorative-shape.tsx` (server component) — ambient CSS-animated
        brand circle.
      - `image-slot.tsx` (server component) — image-placeholder pattern.
      - `page-transition.tsx` (`"use client"`) — `AnimatePresence`-based
        route fade, keyed on the locale-aware pathname.

10.4. `apps/web/components/site-header.tsx` — replace the `hidden`
      attribute on `#primary-navigation` with `inert`, and animate
      `height`/`opacity` via `motion.ul`'s `animate` prop instead of a
      mount/unmount, so desktop's always-visible nav isn't broken by
      conditional mounting.

10.5. `apps/web/app/[locale]/layout.tsx` — mount `<MotionProvider />` and
      wrap `{children}` in `<PageTransition>`.

10.6. Per-page wiring — wrap existing `<section>`/list/group content in
      `<Reveal>` (richer treatment + `<DecorativeShape>`/`<ImageSlot>` on
      Home/About/Services/Contact; calmer bare `<Reveal>` only on Terms/
      Food Regulations, with a one-line comment marking that as deliberate).

10.7. `specs/tech-stack.md` and this phase's `requirement.md`/`validation.md`
      updated to document the decision and new checklist items (done as
      part of this addendum).

## Group 11 — Tailwind CSS v4 migration (addendum)

Added after Group 10 shipped — see `requirement.md`'s "Addendum —
Tailwind CSS v4 migration". Depends on Groups 1–10 (every page/component
this migration touches already exists).

11.1. `apps/web/package.json` — remove `@picocss/pico`; add `tailwindcss`
      and `@tailwindcss/postcss` as devDependencies
      (`pnpm remove @picocss/pico && pnpm add -D tailwindcss
      @tailwindcss/postcss`).

11.2. New `apps/web/postcss.config.mjs` registering `@tailwindcss/postcss`.
      No `tailwind.config.ts` (v4's automatic content detection covers this
      codebase — no dynamic/interpolated class-name strings anywhere).

11.3. `apps/web/app/globals.css` rewritten: `@import "tailwindcss"` replaces
      the Pico import; `@theme` carries the full brand palette, `--radius-
      card`, `--container-brand`, `--spacing-section`, `--ease-out-expo`,
      and `--animate-marquee`/`--animate-float-rotate` (paired with the
      existing top-level `@keyframes`); plain `:root` keeps `color-scheme`
      and the `--transition-*` duration tokens (no matching Tailwind
      namespace); `@layer base` keeps the font-family wiring, add a `main
      h1`/`main p` base sizing rule (Tailwind's Preflight reset removes the
      browser defaults Pico used to supply), and the reduced-motion safety
      net; `@layer components` now only holds `.container` (gained explicit
      `margin-inline: auto`/`padding-inline`, previously supplied silently
      by Pico's own `.container` utility underneath the brand override) and
      `.field` (shared form-control treatment).

11.4. New components in `apps/web/components/`: `button.tsx`
      (`<Button>`/`<ButtonLink>`, variants `primary`/`secondary`/`outline`/
      `invert`), `split-row.tsx` (`<SplitRow media content reverse? tinted?
      decoration?>`), `check-list.tsx`, `notice.tsx`, `page-hero.tsx`,
      `eyebrow.tsx` — replacing the corresponding global CSS classes.

11.5. Rewritten in place (Tailwind utilities, no behavior change unless
      noted): `logo.tsx`, `image-slot.tsx`, `decorative-shape.tsx`,
      `utility-bar.tsx`, `locale-toggle.tsx` (needed new explicit styling
      Tailwind's Preflight doesn't supply for free the way Pico did),
      `site-footer.tsx` (newsletter input gains a real focus-visible ring —
      see requirement.md's accessibility-fix note), and `site-header.tsx`
      (animated hamburger glyph, elevated mobile panel, active-route
      highlight via `aria-current`; `aria-expanded`/`aria-controls`/
      `inert`/the desktop-query hook/pathname-close effect all unchanged).

11.6. All 6 page files (`app/[locale]/{page,about,services,contact,terms,
      food-regulations}.tsx`) rewritten to use the new shared components
      and inline Tailwind utilities in place of the removed global classes.
      `app/[locale]/layout.tsx` untouched (font wiring unaffected).

11.7. `specs/tech-stack.md` and this phase's `requirement.md`/`validation.md`
      updated to document the decision and new checklist items (done as
      part of this addendum).
