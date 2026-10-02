# Phase 3 — Internationalization (Dutch/English): Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed and
sanity-checked before moving to the next.

## Group 0 — Branch & dependency

0.1. Create branch `2026-10-02-phase-3-i18n-dutch-english` off `master`
     (done).

0.2. `apps/web/package.json` — add `"next-intl": "^4.14"` to
     `dependencies`. Run `pnpm install` inside `apps/web` to update
     `apps/web/pnpm-lock.yaml`.

## Group 1 — Message files & routing config

Depends on: Group 0.

1.1. `apps/web/messages/en.json`:
```json
{
  "HomePage": {
    "title": "Zain's Treat n More",
    "subtitle": "Great food, coming soon to the web."
  },
  "LocaleToggle": {
    "label": "Language"
  }
}
```

1.2. `apps/web/messages/nl.json`:
```json
{
  "HomePage": {
    "title": "Zain's Treat n More",
    "subtitle": "Heerlijk eten, binnenkort online."
  },
  "LocaleToggle": {
    "label": "Taal"
  }
}
```
(Flagged in `requirement.md` as best-effort Dutch pending stakeholder
review.)

1.3. `apps/web/i18n/routing.ts`:
```typescript
import { defineRouting } from "next-intl/routing";

export const routing = defineRouting({
  locales: ["en", "nl"],
  defaultLocale: "en",
});
```

1.4. `apps/web/i18n/navigation.ts`:
```typescript
import { createNavigation } from "next-intl/navigation";
import { routing } from "./routing";

export const { Link, redirect, usePathname, useRouter, getPathname } =
  createNavigation(routing);
```

1.5. `apps/web/i18n/request.ts`:
```typescript
import { hasLocale } from "next-intl";
import { getRequestConfig } from "next-intl/server";
import { routing } from "./routing";

export default getRequestConfig(async ({ requestLocale }) => {
  const requested = await requestLocale;
  const locale = hasLocale(routing.locales, requested)
    ? requested
    : routing.defaultLocale;

  return {
    locale,
    messages: (await import(`../messages/${locale}.json`)).default,
  };
});
```

## Group 2 — Proxy (middleware) & Next.js config

Depends on: Group 1.

2.1. `apps/web/proxy.ts` (Next.js 16 convention — **not** `middleware.ts`;
     shipping both is a hard build error):
```typescript
import createMiddleware from "next-intl/middleware";
import { routing } from "./i18n/routing";

export default createMiddleware(routing);

export const config = {
  matcher: ["/((?!api|_next|_vercel|.*\\..*).*)"],
};
```
     `createMiddleware(routing)` applies `localePrefix: "always"` and
     `localeDetection: true` (the `next-intl` defaults), giving the
     Accept-Language auto-redirect and the `NEXT_LOCALE` cookie override
     for free.

2.2. `apps/web/next.config.ts` — wrap the existing config:
```typescript
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");

const nextConfig: NextConfig = {
  output: "standalone",
};

export default withNextIntl(nextConfig);
```

## Group 3 — Locale-aware app directory

Depends on: Groups 1, 2. This moves the existing hello page.

3.1. Create `apps/web/app/[locale]/layout.tsx`, replacing the current
     root `apps/web/app/layout.tsx`:
```typescript
import type { Metadata } from "next";
import { NextIntlClientProvider, hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { routing } from "@/i18n/routing";
import "../globals.css";

export const metadata: Metadata = {
  title: "Zain's Treat n More",
};

export function generateStaticParams() {
  return routing.locales.map((locale) => ({ locale }));
}

export default async function LocaleLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  if (!hasLocale(routing.locales, locale)) {
    notFound();
  }

  return (
    <html lang={locale}>
      <body>
        <NextIntlClientProvider>{children}</NextIntlClientProvider>
      </body>
    </html>
  );
}
```
     This is how `<html lang="en">` becomes locale-aware — `lang` switches
     to `locale` (`"en"` or `"nl"`), resolved per-request.

3.2. Move `apps/web/app/page.tsx` to `apps/web/app/[locale]/page.tsx`,
     migrating the hardcoded strings into the translation keys from Group
     1 and adding the placeholder toggle (Group 4):
```typescript
import { useTranslations } from "next-intl";
import { LocaleToggle } from "@/components/locale-toggle";

export default function Home() {
  const t = useTranslations("HomePage");

  return (
    <main className="container">
      <article>
        <h1>{t("title")}</h1>
        <p>{t("subtitle")}</p>
        <LocaleToggle />
      </article>
    </main>
  );
}
```

3.3. Delete the old root `apps/web/app/layout.tsx` and
     `apps/web/app/page.tsx` (superseded by the `[locale]` versions).
     Root `apps/web/app/` retains only `globals.css` and the new
     `[locale]/` segment — no root-level `page.tsx` (a bare `/` request
     is caught by `proxy.ts` and redirected to `/en` or `/nl` before it
     ever reaches App Router).

## Group 4 — Placeholder language-toggle component

Depends on: Group 3 (needs `i18n/navigation.ts`'s `usePathname`/
`useRouter`).

4.1. `apps/web/components/locale-toggle.tsx` — a client component,
     documented as the convention later phases (Phase 5 renumbered Static
     Pages) will relocate into the real header/nav:
```typescript
"use client";

import { useLocale, useTranslations } from "next-intl";
import { usePathname, useRouter } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";

export function LocaleToggle() {
  const t = useTranslations("LocaleToggle");
  const locale = useLocale();
  const pathname = usePathname();
  const router = useRouter();

  return (
    <label>
      {t("label")}:{" "}
      <select
        value={locale}
        onChange={(e) => router.replace(pathname, { locale: e.target.value })}
      >
        {routing.locales.map((l) => (
          <option key={l} value={l}>
            {l === "en" ? "English" : "Nederlands"}
          </option>
        ))}
      </select>
    </label>
  );
}
```
     `router.replace(pathname, { locale })` is `next-intl`'s navigation
     wrapper — it re-navigates to the same path under the new locale
     prefix and sets the `NEXT_LOCALE` cookie as a side effect, satisfying
     the persistence requirement with no manual `document.cookie` code.

4.2. Add a short note to `docs/local-development.md` documenting this
     convention for Phase 5 onward: where translation keys live
     (`apps/web/messages/{locale}.json`), the naming convention
     (`PageName.keyName`), and that the toggle component will move into
     the header/nav once one exists.

## Group 5 — tsconfig / path alias check

Depends on: Group 3.

5.1. Confirm `apps/web/tsconfig.json`'s existing `@/*` path alias
     (already present from Phase 1) resolves `@/i18n/routing`,
     `@/i18n/navigation`, `@/components/locale-toggle` correctly — no
     tsconfig changes expected, but verify after moving files into
     `app/[locale]/`.

## Group 6 — Roadmap & tech-stack doc updates

Independent of Groups 0–5 (pure doc edits) but logically last since it
documents what this phase actually did.

6.1. `specs/roadmap.md` — renumber Phase 3 (Data Model) through Phase 14
     (Production Hardening) to Phase 4 through Phase 15. Insert the new
     Phase 3 — Internationalization (Dutch/English) section.

6.2. Add the locale-prefix addendum note to the (renumbered) Phase 14 —
     Polish & SEO section's clean-URL list.

6.3. `specs/tech-stack.md` — add the `## Internationalization (i18n)`
     section and Summary Table row.

## Group 7 — Verification

See `validation.md` for the full pass/fail checklist.
