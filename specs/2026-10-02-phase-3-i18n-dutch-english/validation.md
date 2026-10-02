# Phase 3 — Internationalization (Dutch/English): Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

- [ ] `docker compose up` (or `pnpm dev` inside `apps/web`) starts cleanly
      with no build errors from the `next-intl` plugin wiring.
- [ ] Visiting `http://localhost:3000/` with `Accept-Language: en` (or no
      header) redirects to `http://localhost:3000/en`.
- [ ] Visiting `http://localhost:3000/` with `Accept-Language: nl` (e.g.
      `curl -H "Accept-Language: nl" -i http://localhost:3000/`) redirects
      to `/nl`.
- [ ] `http://localhost:3000/en` renders the hello page with the English
      strings from `messages/en.json` ("Zain's Treat n More" / "Great
      food, coming soon to the web.").
- [ ] `http://localhost:3000/nl` renders the hello page with the Dutch
      strings from `messages/nl.json`.
- [ ] `<html lang="en">` on `/en` and `<html lang="nl">` on `/nl` —
      confirmed by viewing page source, not just the rendered DOM.
- [ ] The language-toggle control is visible on the hello page, switches
      locale (URL changes from `/en` to `/nl` or vice versa) without
      error, and the displayed strings update.
- [ ] After toggling, a browser cookie named `NEXT_LOCALE` is set; a fresh
      reload of `/` re-lands on the previously toggled locale, not back on
      the `Accept-Language` default — confirms persistence.
- [ ] `apps/web/proxy.ts` exists; `apps/web/middleware.ts` does **not**
      exist (Next.js 16 treats having both as a hard build error).
- [ ] `apps/web/app/layout.tsx` and `apps/web/app/page.tsx` (the old
      non-locale versions) no longer exist — superseded by
      `apps/web/app/[locale]/layout.tsx` and
      `apps/web/app/[locale]/page.tsx`.
- [ ] No admin-dashboard files exist yet to accidentally touch (none are
      built until renumbered Phase 9/10), and no `apps/api` files were
      modified — confirms the API/admin-stays-English-only scope was
      respected.
- [ ] `apps/web/package.json` lists `next-intl` under `dependencies` only,
      and `apps/web/pnpm-lock.yaml` is updated/committed.
- [ ] `specs/roadmap.md` phase numbers are internally consistent: the new
      Phase 3 (Internationalization) exists, old Phase 3 (Data Model)
      through old Phase 14 (Production Hardening) are renumbered to 4–15
      with no duplicate or skipped numbers, and no other file in the repo
      contains a now-stale "Phase 3 = Data Model" or similar reference.
- [ ] `git diff master --stat` for this branch touches only `apps/web/**`,
      `specs/roadmap.md`, `specs/tech-stack.md`, `docs/local-development.md`,
      and the new `specs/2026-10-02-phase-3-i18n-dutch-english/` folder —
      nothing in `apps/api/**`, `docker-compose*.yml`, or
      `.github/workflows/**`.

## Definition of done

Phase 3 is complete when every box above is checked: the public site
auto-detects and redirects to `/en` or `/nl` based on browser language,
the manual toggle overrides and persists that choice via the `NEXT_LOCALE`
cookie, both locales render correctly translated hello-page content with a
locale-correct `<html lang>`, the `messages/{locale}.json` convention is
in place for future phases to extend, `specs/roadmap.md` and
`specs/tech-stack.md` reflect the new phase and tech decision, and no
admin or API surface was touched. Ready for the renumbered Phase 4 (Data
Model) to proceed unaffected.
