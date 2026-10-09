# Phase 16 — WhatsApp CTAs: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 16 — the
original one-bullet entry, expanded by this phase's planning:

> - `wa.me` deep links placed in header, hero, contact page, and footer
>   (per `tech-stack.md`).

Also grounded in README §21 ("WhatsApp Integration" — CTA copy examples,
WhatsApp green `#1DA332`, the "two customer paths" diagram) and §9.1
("Header / Navigation" — "On mobile: ... WhatsApp CTA where appropriate").

## Context

This is `mission.md` business goal 9 ("Make WhatsApp an easy, low-friction
customer-contact channel") and Journey C from README §36
(`Homepage / Contact → WhatsApp Us → WhatsApp Conversation`). Phase 15 gave
customers the structured enquiry path; this phase gives them the "quick
conversation" path from README §21's diagram.

Planning this phase's research turned up that it isn't starting from a
blank slate — three of the four placements already have a bare
`https://wa.me/31630545277` link, added incidentally while those phases
built unrelated features:

- `apps/web/app/[locale]/page.tsx:92-97` — the homepage hero's "WhatsApp
  Us" CTA (added alongside Phase 5/12's hero work).
- `apps/web/components/site-footer.tsx:85-93` — an icon-only link in the
  footer's social-icons row (added alongside Phase 5's footer build).
- `apps/web/app/[locale]/contact/page.tsx:38` — a plain text link in the
  Contact page's info list (added alongside Phase 5/15's contact work).

None of the three have the prefilled `?text=` message
`tech-stack.md`'s "WhatsApp Integration" section already specifies
(`https://wa.me/<number>?text=...`), and the number itself is a literal
string duplicated three times with no shared constant. The header
(`apps/web/components/site-header.tsx`) has no WhatsApp CTA at all — the
one placement from README §9.1 that's genuinely new work.

This phase's real scope, decided with the user (see Decisions below): add
the missing header CTA, and bring the three existing links up to the
`tech-stack.md`-specified shape (shared number constant, prefilled
message, consistent new-tab behavior) rather than leaving them as
pre-existing, undocumented, inconsistent one-offs.

Pieces already on disk this phase builds on rather than rebuilds:

- `apps/web/app/globals.css:17-18` — `--color-whatsapp` /
  `--color-whatsapp-dark` tokens already defined (Phase 5), used as-is.
- `apps/web/components/button.tsx` — `buttonClasses({ variant, color })`
  already supports `color: "whatsapp"` (used by the hero CTA today); its
  `ButtonLink` wraps next-intl's internal-routing `Link` and can't be used
  for an external `wa.me` link, so every placement hand-rolls a raw `<a>`
  with `buttonClasses` and/or custom Tailwind classes, matching the
  existing hero/footer precedent.
- Each component's established pattern of defining its own small inline
  SVG icon function locally (`WhatsAppIcon` in `site-footer.tsx`, the
  trust-indicator icons in `page.tsx`) rather than a shared icon module —
  this phase's new header icon follows the same convention, not a new
  shared-icons file.

Branches off `master` (Phase 15 already merged; this phase touches no file
unique to any other unmerged branch).

## Decisions

1. **Gap-fill and upgrade all four placements, not just add the missing
   header CTA.** Decided with the user: leaving the hero/footer/contact
   links as bare, undocumented one-offs would mean `tech-stack.md`'s
   already-pinned "prefilled message" decision stays permanently
   unimplemented, and the duplicated number stays duplicated. All four
   placements end this phase using the same shared constant and the same
   prefilled message.

2. **`apps/web/lib/whatsapp.ts` (new)**: a shared `WHATSAPP_NUMBER`
   constant (`"31630545277"`, the same number already live in production)
   and a `buildWhatsAppLink(message: string)` helper that returns
   `` `https://wa.me/${WHATSAPP_NUMBER}?text=${encodeURIComponent(message)}` ``.
   Decided with the user over a new `NEXT_PUBLIC_WHATSAPP_NUMBER` env var:
   the number isn't a secret, doesn't vary between dev and prod, and
   `apps/web` has no existing `NEXT_PUBLIC_*` convention to extend — a
   plain shared constant needs no new infrastructure.

3. **One generic prefilled message, not per-placement contextual text.**
   Decided with the user: a single translated message (new `Whatsapp`
   i18n namespace, key `prefilledMessage`) is used by all four placements.
   This is the first namespace in this codebase shared verbatim across
   multiple components/pages, rather than each component owning its own
   duplicate copy (the existing convention — e.g. `HomePage.whatsappUs` vs.
   `SiteFooter.whatsappLabel` are separate keys for separate button-label
   text) — justified here because, unlike those button *labels* (which
   differ in length/context per placement), the prefilled *message* is
   meant to read identically regardless of where the customer clicked from.
   Each placement's visible label/aria-label text stays a separate,
   component-scoped key (`SiteHeader.whatsappLabel`,
   `HomePage.whatsappUs`, `SiteFooter.whatsappLabel`,
   `ContactPage.whatsappLabel`/`whatsappValue`), unchanged in approach from
   today.
   - `en`: "Hi Zain's Treat n More! I'd like to know more about your menu
     and services."
   - `nl`: "Hallo Zain's Treat n More! Ik wil graag meer weten over jullie
     menu en diensten."

4. **Superseded 2026-10-09 (post-implementation revision) — see Decision 4
   (revised) below.** Originally: a header-nav icon-only button, visible
   on both mobile and desktop, added as a new `<li>` inside
   `site-header.tsx`'s existing nav `<ul>`. This was implemented, then
   replaced per the revision below before this phase was validated.

4. **(Revised) New global floating WhatsApp CTA, not a header-nav item.**
   Decided with the user after reviewing the header-icon implementation:
   the WhatsApp CTA is instead a `fixed` floating action button pinned to
   the bottom-right corner of the viewport, rendered once in
   `apps/web/app/[locale]/layout.tsx` (a sibling of `SiteHeader` and
   `SiteFooter`, inside `CartProvider`) so it persists across every page
   and scroll position rather than living only inside the header
   component. The header's nav `<ul>` no longer gets a WhatsApp list item
   — `site-header.tsx`'s `WhatsAppIcon` function, its `tw` translation
   hook, and the `<li>` added for the original Decision 4 are all removed.
   The floating button:
   - Uses WhatsApp's official brand glyph (the recognizable
     speech-bubble/phone mark from WhatsApp's own branding) as an inline
     SVG — not the hand-drawn `WhatsAppIcon` originally written for the
     header button.
   - Is a filled circle using the existing `--color-whatsapp` design token
     as its background (matching the brand's green-circle treatment) with
     the glyph in white, rather than the header icon's transparent/
     hover-background treatment (that styling assumed a nav list item,
     which no longer applies).
   - Keeps the same `buildWhatsAppLink(tw("prefilledMessage"))` href,
     `target="_blank" rel="noopener noreferrer"`, and `aria-label` (reusing
     `SiteHeader.whatsappLabel`) as the original header implementation —
     only the placement and visual treatment change, not the link
     behavior or i18n plumbing from Decisions 2-3.
   - Is a plain anchor with no scroll-triggered show/hide behavior and no
     GSAP animation — out of scope for this revision (see "Out of scope"
     below).

5. **Consistent `target="_blank" rel="noopener noreferrer"` on every
   placement.** The footer and contact page links already have this; the
   hero's current link is missing it (clicking it today navigates away
   from the site instead of opening WhatsApp in a new tab) — fixed as part
   of this phase's hero update, not left as a separate bug report.

6. **No visual restyle of the footer or contact page placements** — only
   their `href` changes (shared constant + prefilled message + consistent
   `target`/`rel`). The footer keeps its icon-only treatment; the contact
   page keeps its plain-text-link-in-an-info-list treatment. Only the
   header gets a new visual element, since it's the only placement that
   had nothing before. Decided without a separate user prompt as the
   narrowest change that satisfies "all four placements use the shared
   constant and prefilled message" (Decisions 1–3) without also taking on
   an unrequested visual-consistency pass across already-shipped pages.

7. **No backend work, no new tests, no new env vars.** This phase is
   `apps/web`-only, static-content-only — no `apps/api` route, no database
   table, nothing to integration-test the way Phases 10/15 test their
   public write endpoints. Validated manually via `/validate`'s live smoke
   test and by reading the rendered `href` values, same precedent as every
   prior phase's frontend-only work.

## Out of scope

- A WhatsApp Business API/webhook integration, or anything beyond a
  static `wa.me` link — explicitly ruled out already in `tech-stack.md`'s
  "WhatsApp Integration" section.
- Per-placement contextual prefilled messages (e.g. a catering-specific
  message from the Contact page's catering tab) — Decision 3. Revisit
  only if the business reports the generic message reads oddly in a
  specific context.
- A `NEXT_PUBLIC_WHATSAPP_NUMBER` env var or any other deployment-time
  configurability for the number — Decision 2. If the business's WhatsApp
  number ever changes, it's a one-line edit to `lib/whatsapp.ts`, not a
  deploy-config change.
- Restyling the footer icon or the contact page's info-list link into a
  button to visually match the hero treatment — Decision 6.
- Scroll-triggered show/hide, enter/exit animation, or a dismiss/close
  control for the floating button (Decision 4 revised) — it's a plain
  always-visible fixed anchor. Revisit only if the client reports it
  obscuring content on a specific page.
- Re-adding any WhatsApp affordance inside `site-header.tsx` — Decision 4
  (revised) removes it from the header entirely in favor of the single
  global floating button.
- Any homepage section beyond the existing hero CTA — README §10's
  recommended homepage section order lists a separate, later "WhatsApp
  CTA" section (between "About Preview" and "Footer"); the roadmap's
  Phase 16 bullet only ever named "header, hero, contact page, and
  footer", so that extra homepage section is treated as already covered
  by the existing hero CTA, not a fifth placement to add. Revisit if the
  client specifically wants a dedicated mid-page WhatsApp section later.
- The `UtilityBar` marquee's existing `"WhatsApp Ordering"` text
  (`ServicesPage.whatsapp` key, `apps/web/components/utility-bar.tsx`) —
  it's `aria-hidden` decorative text, not a link, and out of scope here.

## Open risks flagged during planning

Both of this phase's original open risks were revisited and resolved
before implementation started (see "Risks resolved in this revisit"
below) — none are carried forward.

## Risks resolved in this revisit (2026-10-09)

1. **The shared `Whatsapp.prefilledMessage` i18n namespace is a new
   pattern** — every other translated string in this codebase lives under
   a component/page-scoped namespace. Resolved: the namespace stays
   narrowly scoped to WhatsApp-related shared content (this one key) and
   is **not** generalized into a broader `Common` bucket — introducing a
   speculative shared-strings namespace now, before any other phase
   actually needs one, would be exactly the kind of premature abstraction
   this codebase avoids elsewhere. If a future phase needs another
   cross-cutting string, it decides then whether to extend `Whatsapp`,
   rename it, or add something new; this phase commits to neither.
2. **The generic prefilled message is a judgment call on business voice**,
   not something pinned anywhere in the README or `mission.md`. Resolved
   with the user: the drafted copy (Decision 3's `en`/`nl` text) is kept
   as-is — no further wordsmithing needed before implementation. Easy to
   tweak in the message files post-launch if the business wants different
   wording.
