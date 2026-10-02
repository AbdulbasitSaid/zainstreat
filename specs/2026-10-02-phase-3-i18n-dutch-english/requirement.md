# Phase 3 — Internationalization (Dutch/English): Requirements

## Source

No existing `specs/roadmap.md` bullet covered this — it is a new phase
inserted ahead of the original Phase 3 (Data Model) at the stakeholder's
explicit request, pushing Data Model and everything after it down by one
(new Phase 4 = Data Model ... new Phase 15 = Production Hardening). See
`specs/roadmap.md`'s new Phase 3 entry, added as part of this phase.

## Context

Phases 1 (scaffolding) and 2 (AWS deployment) are merged to `master`.
Nothing beyond `apps/web/app/page.tsx` (hardcoded "Zain's Treat n More" /
"Great food, coming soon to the web.") and `apps/web/app/layout.tsx`
(`<html lang="en">`) exists on the public site — no header/nav, no real
pages, no menu/cart/order copy. A stakeholder has requested a Dutch/
English language toggle on the public site. Rather than retrofit
translation into every subsequent phase's UI copy as it's written
(Static Pages onward), this is the cheapest point to establish the i18n
infrastructure and convention, before any real copy exists.

## Decisions

1. **Branch:** `2026-10-02-phase-3-i18n-dutch-english` off `master`.
2. **URL strategy:** locale-prefixed URLs (`/en/...`, `/nl/...`) — each
   language gets its own indexable URL for SEO.
3. **Default locale & detection:** English (`en`) is the default/fallback
   locale. The visitor's `Accept-Language` header is used to auto-redirect
   a first-time visitor with no stored preference to their matching
   locale (e.g. a Dutch browser → `/nl`). A manual toggle always lets the
   visitor override this, persisted via a cookie (`NEXT_LOCALE`) so the
   choice sticks across visits.
4. **Scope:** public-facing site only (home/about/contact/static pages,
   menu, cart, order flow, catering/contact forms, once those phases
   exist). The admin dashboard (new Phase 9/10: admin auth, order/menu
   management) stays English-only. The Rust API (`apps/api`) stays
   English-only for any error/validation messages — this is a
   frontend-only concern.
5. **Library: `next-intl@^4.14`** — verified compatible with this repo's
   pinned `"next": "^16"` / `"react": "^19"` (first-class App Router
   support, built-in middleware for locale-prefix + Accept-Language
   detection + cookie override). See `specs/tech-stack.md`'s new
   "Internationalization (i18n)" section for full rationale.
6. **Next.js 16 naming requirement:** the middleware file is
   `apps/web/proxy.ts` (not `middleware.ts`) exporting a function that
   wraps `next-intl`'s `createMiddleware`. Next.js 16 renamed this
   convention and locked it to the Node.js runtime; shipping both
   `middleware.ts` and `proxy.ts` is a hard build error.
7. **Locale codes:** `en` and `nl` (no region variants, e.g. not
   `en-GB`/`nl-NL`).
8. **Placeholder toggle scope:** this phase ships a minimal, unstyled
   language-toggle control directly on the existing hello page
   (`app/[locale]/page.tsx`), since no real header/nav exists until Phase
   5 (renumbered Static Pages). Phase 5 is responsible for relocating
   this control into the real header/nav using the same
   convention/components this phase establishes.
9. **Roadmap renumbering:** `specs/roadmap.md`'s phase numbers 3–14 shift
   to 4–15; only `roadmap.md`'s own text is edited — no existing spec
   folders are renamed (Phases 1–2 are already merged and historically
   accurate under their original dates/numbers; no Phase 4+ spec folders
   exist yet to rename).

## Out of scope (explicitly deferred)

- Admin dashboard i18n — stays English-only indefinitely (not a "later
  phase" item at all, per decision 4).
- `apps/api` error/validation message translation — frontend-only concern
  for now.
- Translating content that doesn't exist yet (static page copy, menu/cart
  copy, order flow copy, catering/contact form copy) — each future phase
  (renumbered Phase 5 onward) adds its own UI copy directly into
  `messages/en.json` / `messages/nl.json` using the convention this phase
  establishes, not retroactively populated here.
- Relocating the toggle into a real header/nav — Phase 5 (renumbered
  Static Pages)'s job, once a header/nav exists.
- Professional review of the Dutch translation — this phase's one
  placeholder string is a best-effort literal translation, not
  stakeholder-reviewed copy (see risk below).

## Open risks flagged during planning

- **Renumbering:** only `specs/roadmap.md`'s phase numbers need updating
  now — no future phase folders exist yet to rename. If any in-flight
  conversation/ticket elsewhere refers to "Phase 3 = Data Model", it is
  now stale.
- **Translation quality:** the actual Dutch copy in `messages/nl.json`
  for this phase is a single short string — low risk now, but who
  reviews/supplies accurate Dutch translations (native speaker vs.
  machine translation) should be decided before Phase 5 onward writes
  substantially more copy.
- **Next.js version assumption:** this plan assumes the repo is on actual
  Next.js 16 (confirmed by `apps/web/package.json`'s `"next": "^16"`), so
  the `proxy.ts` (not `middleware.ts`) convention applies. If a pin ever
  moves back to 15.x, the middleware file/export name would need to
  revert.
