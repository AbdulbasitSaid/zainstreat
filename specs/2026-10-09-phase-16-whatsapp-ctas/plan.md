# Phase 16 — WhatsApp CTAs: Implementation Plan

Entirely `apps/web`. No `apps/api`, no migration, no `docker-compose*.yml`,
no new env var, no Caddyfile change.

## Group 0 — Branch

Branched `2026-10-09-phase-16-whatsapp-ctas` off `master` (Phase 15
already merged).

## Group 1 — `apps/web/lib/whatsapp.ts` (new)

Depends on: nothing.

```ts
export const WHATSAPP_NUMBER = "31630545277";

export function buildWhatsAppLink(message: string): string {
  return `https://wa.me/${WHATSAPP_NUMBER}?text=${encodeURIComponent(message)}`;
}
```

## Group 2 — i18n: new `Whatsapp` namespace + new `SiteHeader` key

Depends on: nothing. Both `apps/web/messages/en.json` and
`apps/web/messages/nl.json`.

New top-level namespace (add after `UtilityBar`, matching the existing
alphabetical-ish-by-introduction ordering in both files):

```json
"Whatsapp": {
  "prefilledMessage": "Hi Zain's Treat n More! I'd like to know more about your menu and services."
}
```

```json
"Whatsapp": {
  "prefilledMessage": "Hallo Zain's Treat n More! Ik wil graag meer weten over jullie menu en diensten."
}
```

New key inside the existing `SiteHeader` namespace in both files:

```json
"whatsappLabel": "WhatsApp"
```

```json
"whatsappLabel": "WhatsApp"
```

(`SiteHeader.navLabel`, `.openMenu`, `.closeMenu`, `.cart`, `.orderNow`,
and the `NAV_ITEMS` keys — `home`/`about`/`services`/`menu`/`contact` —
are all untouched.)

## Group 3 (revised) — `apps/web/components/site-header.tsx`: revert header
CTA; new `apps/web/components/whatsapp-float-button.tsx`

Depends on: Groups 1–2. Supersedes the original Group 3 (the header-icon
implementation below was built, then reverted per requirement.md's revised
Decision 4).

**3a. Revert `site-header.tsx`:** remove the `buildWhatsAppLink` import,
the `tw` translation hook, the local `WhatsAppIcon` function, and the
`<li>` WhatsApp list item added between `LocaleToggle` and "Order Now" —
back to its pre-Phase-16 shape plus nothing.

**3b. New `apps/web/components/whatsapp-float-button.tsx`** (server
component — no state/interactivity beyond a link, so no `"use client"`
needed, matching `contact/page.tsx`'s pattern of calling
`getTranslations` directly in an async server component):

```tsx
import { getTranslations } from "next-intl/server";
import { buildWhatsAppLink } from "@/lib/whatsapp";

export async function WhatsAppFloatButton() {
  const t = await getTranslations("SiteHeader");
  const tw = await getTranslations("Whatsapp");

  return (
    <a
      href={buildWhatsAppLink(tw("prefilledMessage"))}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={t("whatsappLabel")}
      className="fixed bottom-6 right-6 z-50 inline-flex h-14 w-14 items-center justify-center rounded-full bg-whatsapp text-white shadow-lg transition-transform hover:scale-105"
    >
      <WhatsAppGlyph />
    </a>
  );
}

function WhatsAppGlyph() {
  return (
    <svg viewBox="0 0 448 512" fill="currentColor" aria-hidden="true" className="h-7 w-7">
      <path d="M380.9 97.1C339 55.1 283.2 32 223.9 32c-122.4 0-222 99.6-222 222 0 39.1 10.2 77.3 29.6 111L0 480l117.7-30.9c32.4 17.7 68.9 27 106.1 27h.1c122.3 0 224.1-99.6 224.1-222 0-59.3-25.2-115-67.1-157zm-157 341.6c-33.2 0-65.7-8.9-94-25.7l-6.7-4-69.8 18.3L72 359.2l-4.4-7c-18.5-29.4-28.2-63.3-28.2-98.2 0-101.7 82.8-184.5 184.6-184.5 49.3 0 95.6 19.2 130.4 54.1 34.8 34.9 56.2 81.2 56.1 130.5 0 101.8-84.9 184.6-186.6 184.6zm101.2-138.2c-5.5-2.8-32.8-16.2-37.9-18-5.1-1.9-8.8-2.8-12.5 2.8-3.7 5.6-14.3 18-17.6 21.8-3.2 3.7-6.5 4.2-12 1.4-32.6-16.3-54-29.1-75.5-66-5.7-9.8 5.7-9.1 16.3-30.3 1.8-3.7.9-6.9-.5-9.7-1.4-2.8-12.5-30.1-17.1-41.2-4.5-10.8-9.1-9.3-12.5-9.5-3.2-.2-6.9-.2-10.6-.2-3.7 0-9.7 1.4-14.8 6.9-5.1 5.6-19.4 19-19.4 46.3 0 27.3 19.9 53.7 22.6 57.4 2.8 3.7 39.1 59.7 94.8 83.8 35.2 15.2 49 16.5 66.6 13.9 10.7-1.6 32.8-13.4 37.4-26.4 4.6-13 4.6-24.1 3.2-26.4-1.3-2.5-5-3.9-10.5-6.6z" />
    </svg>
  );
}
```

`bg-whatsapp` resolves to the existing `--color-whatsapp` token
(`globals.css:17`) via the Tailwind theme mapping already used elsewhere
(e.g. `buttonClasses({ color: "whatsapp" })`) — no new CSS token needed.
`z-50` keeps it above `PageTransition` content; `bottom-6 right-6` gives a
24px inset from the viewport's bottom-right corner, a standard FAB offset.

**3c. Wire into `apps/web/app/[locale]/layout.tsx`:** import
`WhatsAppFloatButton` and render it as a sibling of `SiteFooter`, inside
`CartProvider` (no cart dependency, but keeps all page chrome inside the
same provider tree):

```tsx
import { WhatsAppFloatButton } from "@/components/whatsapp-float-button";
```

```tsx
<SiteFooter />
<WhatsAppFloatButton />
```

## Group 4 — `apps/web/app/[locale]/page.tsx`: hero CTA fix

Depends on: Group 1.

Add the import:

```ts
import { buildWhatsAppLink } from "@/lib/whatsapp";
```

Inside `Home()`, alongside the existing `const t = await
getTranslations("HomePage");`:

```ts
const tw = await getTranslations("Whatsapp");
```

Replace the existing hero anchor (current lines 91-97):

```tsx
<a
  href="https://wa.me/31630545277"
  role="button"
  className={buttonClasses({ variant: "outline", color: "whatsapp" })}
>
  {t("whatsappUs")}
</a>
```

with:

```tsx
<a
  href={buildWhatsAppLink(tw("prefilledMessage"))}
  target="_blank"
  rel="noopener noreferrer"
  role="button"
  className={buttonClasses({ variant: "outline", color: "whatsapp" })}
>
  {t("whatsappUs")}
</a>
```

(requirement.md Decision 5 — this is the fix for the hero link currently
missing `target`/`rel`.)

## Group 5 — `apps/web/components/site-footer.tsx`: href only

Depends on: Group 1. No visual change (requirement.md Decision 6).

Add the import:

```ts
import { buildWhatsAppLink } from "@/lib/whatsapp";
```

Add a second translation hook alongside the existing `t`:

```ts
const tw = useTranslations("Whatsapp");
```

Replace the existing `href="https://wa.me/31630545277"` (current line 86)
with:

```tsx
href={buildWhatsAppLink(tw("prefilledMessage"))}
```

`target="_blank" rel="noopener noreferrer"` are already present on this
link — unchanged.

## Group 6 — `apps/web/app/[locale]/contact/page.tsx`: href only

Depends on: Group 1. No visual change (requirement.md Decision 6).

Add the import:

```ts
import { buildWhatsAppLink } from "@/lib/whatsapp";
```

Alongside the existing `const t = await getTranslations("ContactPage");`:

```ts
const tw = await getTranslations("Whatsapp");
```

Replace the existing `href="https://wa.me/31630545277"` (current line 38)
with:

```tsx
href={buildWhatsAppLink(tw("prefilledMessage"))}
```

`target="_blank" rel="noopener noreferrer"` are already present on this
link — unchanged.

## Group 7 — Manual verification

No automated tests this phase (requirement.md Decision 7). Run
`/validate` once Groups 1–6 are done; its live smoke test plus a manual
click-through (see `validation.md`) is the only verification.
