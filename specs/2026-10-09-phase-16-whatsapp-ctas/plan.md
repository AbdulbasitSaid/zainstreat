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

## Group 3 — `apps/web/components/site-header.tsx`: header CTA

Depends on: Groups 1–2.

Add the import:

```ts
import { buildWhatsAppLink } from "@/lib/whatsapp";
```

Add a second translation hook alongside the existing one (`useTranslations`
is already used for `SiteHeader`; next-intl supports multiple namespace
hooks in the same component):

```ts
const tw = useTranslations("Whatsapp");
```

Add a small local icon function (same convention as
`site-footer.tsx`'s `WhatsAppIcon` — copy its SVG body verbatim; this
phase does not introduce a shared icons module, matching the existing
per-file-icon convention):

```tsx
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
```

New `<li>` inside the existing nav `<ul>` (`navListRef`), placed after the
`LocaleToggle` `<li>` (line 160 in the current file) and before the
"Order Now" `<li>` (requirement.md Decision 4):

```tsx
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
