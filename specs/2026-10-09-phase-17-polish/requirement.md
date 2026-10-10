# Phase 17 — Polish & Non-Functional Requirements: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 17 — the
original bullet list, before this phase's planning expanded it:

> - SEO: page titles, meta descriptions, Open Graph tags, semantic HTML,
>   clean URLs (`/`, `/about`, `/services`, `/menu`, `/order`, `/contact`,
>   `/terms`, `/food-regulations`).
> - Accessibility: keyboard navigation, visible focus states, form labels,
>   alt text, sufficient contrast.
> - Image optimization pass (formats, lazy loading).
> - Responsive QA across mobile, tablet, desktop.
> - Input validation/sanitization hardening on all forms and API endpoints.
> - Wire in email sending (order confirmations, contact/catering
>   notifications) via `lettre` + transactional email provider.

## Context

Six largely-independent non-functional concerns, bundled into one phase
because none of them is large enough alone to justify its own branch, and
all six are genuine pre-launch gaps rather than net-new features — Phase
18 (Production Hardening & Final Rollout) assumes these are already done
when it runs its Definition-of-Done smoke test.

Planning research (read-only, against the current `master`) found each
area already has partial coverage, not a blank slate:

1. **SEO** — `apps/web/app/[locale]/menu/page.tsx`, `cart/page.tsx`, and
   `order/page.tsx` already have `generateMetadata` (title + description,
   via next-intl). Six other public pages (Home, About, Services, Contact,
   Terms, Food Regulations) have none, inheriting only the root layout's
   bare static `title: "Zain's Treat n More"` with no `description` and
   no Open Graph. `apps/web/app/[locale]/layout.tsx:37` already sets
   `<html lang={locale}>` correctly. No `sitemap.xml`/`robots.txt` exist
   yet.
2. **Accessibility** — `site-header.tsx`'s hamburger/mobile-nav already
   does `aria-expanded`, `aria-controls`, translated `aria-label`,
   `inert` on the closed mobile panel, and `aria-current="page"` — solid.
   `button.tsx` already has explicit `focus-visible:outline` styling, as
   do several other components (`locale-toggle.tsx`, admin components,
   the shared `.field` class in `globals.css`). Gaps: `site-header.tsx`'s
   own nav `<Link>`s/hamburger button don't carry that same explicit
   `focus-visible` styling, and `contact-form.tsx`/`order-details-form.tsx`
   render error text next to each field but never wire it to the input via
   `aria-describedby`/`aria-invalid` — a screen reader won't announce or
   associate the error. Every real `<label htmlFor>` pairing is already
   correct, and every customer-facing `next/image` already has a
   meaningful, dynamic `alt`.
3. **Image optimization** — `next.config.ts` has no explicit
   `images.formats` (AVIF/WebP) configured. More significantly:
   `menu-item-card.tsx`, `cart-line-item.tsx`, and `admin-menu-list.tsx`
   all set `unoptimized={item.image_url.startsWith("http")}` — a Phase 14
   decision made because, in **dev**, the `web` container can't reach the
   `api` container at the browser-facing `localhost:8080` host. But
   `item.image_url` is an absolute URL in both environments
   (`http://localhost:8080/api/media/{key}` in dev,
   `https://api.$DOMAIN/api/media/{key}` in prod — confirmed from
   `apps/api/src/routes/admin/media.rs:74-76`'s `PUBLIC_API_URL`-based
   construction and both compose files' env values), so the current
   `startsWith("http")` check disables optimization in **production too**,
   not just dev — every real menu-item photo on the live site ships as a
   full-size JPEG with no format conversion or compression. Only one
   `sizes` prop exists anywhere (`site-image.tsx:40`); the menu/cart image
   usages have none. Exactly one raw `<img>` exists
   (`admin-image-upload.tsx:175`, an admin-only crop preview with
   `alt=""`).
4. **Responsive QA** — no existing viewport-testing infrastructure
   (expected; this is a manual pass, not a tooling gap).
5. **Input validation/sanitization hardening** —
   `apps/api/src/validation.rs`'s `is_valid_email` already carries its own
   comment: *"Deliberately loose — just 'looks like an email'. Decision 10
   scopes real validation hardening to roadmap.md Phase 17."* — a direct,
   in-repo pointer to this phase. `is_valid_phone` is already reasonably
   strict (digit-count bounds, rejects same-digit/sequential-digit fakes).
   No validation crate exists on either side (`apps/api/Cargo.toml` has no
   `validator`; `apps/web/package.json` has no `zod`/`yup`); both sides use
   hand-rolled manual checks, which this phase continues rather than
   introduces new dependencies for.
6. **Email** — `lettre` is not yet in `apps/api/Cargo.toml`. The three
   POST handlers' commit/insert → response points are
   `orders.rs::create_order` (commit at `orders.rs:301`, response at
   303-320), `contact_messages.rs::create_contact_message` (insert at
   81-94, response at 96), and `catering_enquiries.rs::create_catering_enquiry`
   (insert at 115-133, response at 135) — all three already have an
   honeypot short-circuit that returns *before* reaching the real
   insert, so a spam submission never triggers an email send either.

Branches off `master` (Phase 16 already merged; no other unmerged branch
touches these files).

## Decisions

1. **Email provider: Amazon SES, not Resend/Postmark.** Decided with the
   user — they preferred staying on AWS rather than adding a third-party
   email vendor, given the rest of the stack is already AWS
   (Lightsail). `lettre`'s SMTP transport talks to SES's SMTP endpoint
   (`email-smtp.eu-central-1.amazonaws.com:587`, STARTTLS) using
   SES-generated SMTP credentials — this keeps `lettre` as the
   already-pinned crate and needs no AWS SDK/credential-chain code, just
   host/port/username/password as env vars (same shape as every other
   secret already in `.env.example`). See `tech-stack.md`'s Email section
   for the full rationale this supersedes.
2. **Placeholder sender/recipient addresses for now.** Decided with the
   user: no verified sending domain exists yet. `EMAIL_FROM_ADDRESS` and
   `EMAIL_BUSINESS_NOTIFY_ADDRESS` are read from env vars with obviously-
   fake placeholder defaults in `.env.example`
   (`no-reply@zainstreat.com` / `orders@zainstreat.com`). Verifying the
   real domain in SES and requesting production access (SES starts every
   new account in a sandbox that only delivers to pre-verified
   recipients) is a manual, pre-launch operations step — not something
   this phase's code does or can do. Local dev needs no SES credentials
   at all: a missing/incomplete env-var set makes `EmailConfig::from_env()`
   return `None`, every send-site becomes a logged no-op
   (`tracing::warn!` once at startup), and no local SMTP catcher
   (MailHog, etc.) is introduced.
3. **Fire-and-forget delivery; email failure never fails the real
   request.** Decided with the user (Recommended option, accepted as-is):
   the order/enquiry is already persisted by the time an email would be
   sent; a `tokio::spawn`ed send that fails only logs
   (`tracing::error!`, consistent with Phase 8's structured logging) —
   it never blocks, delays past that point, or rolls back the customer-
   facing response. Matches `mission.md`: the persisted record is the
   source of truth, the email is a courtesy notification layered on top.
4. **Two distinct email directions, not one generic "send an email"
   helper call pattern.** Order confirmations go **to the customer**
   (the email address they submitted on the order form). Contact and
   catering-enquiry notifications go **to the business**
   (`EMAIL_BUSINESS_NOTIFY_ADDRESS`), per `roadmap.md`'s Phase 15 note
   ("forwarded by email to the business") — the business, not the
   customer, is blind to submissions without this. A shared
   `apps/api/src/email.rs::send_email()` primitive is reused for both
   directions; only the recipient and body content differ per call site.
5. **Open Graph default image: the existing logo asset, not a stock
   hero photo.** Decided with the user (Recommended option) — on-brand,
   no new asset needed, already exists, works until real brand
   photography gets dedicated OG crops later. Set once on the root
   locale layout's `openGraph.images`, inherited by every page that
   doesn't set its own (Next.js merges layout and page metadata
   field-by-field).
6. **No title template; every page keeps a fully-suffixed literal
   title string**, matching the existing Menu/Cart/Order precedent
   (`"Menu | Zain's Treat n More"`, not a bare `"Menu"` composed via a
   parent `title.template`). Adding a `template` now would double-suffix
   those three already-shipped pages unless they were also rewritten —
   narrower to keep the existing convention than to introduce a template
   and touch three working pages to accommodate it.
7. **`metadataBase` derived from the same `DOMAIN` env var `apps/web`
   already receives in production** (Phase 14's `docker-compose.prod.yml`
   wiring), falling back to `http://localhost:3000` in dev — no new env
   var. Used for `metadataBase` (root layout) and as the base URL in
   `sitemap.ts`/`robots.ts`.
8. **Sitemap covers content pages only, not the cart/order flow.**
   `/`, `/about`, `/services`, `/menu`, `/contact`, `/terms`,
   `/food-regulations` — each emitted once per locale (`/en/...`,
   `/nl/...`). `/cart` and `/order` are session-scoped transactional
   pages with nothing to index, not content; they're excluded from
   `sitemap.ts` and not specially called out in `robots.ts` (not
   disallowed either — nothing stops a crawler reaching them, they're
   just not advertised). `robots.ts` explicitly disallows `/admin` and
   `/api`.
9. **Image optimization: narrow the existing `unoptimized` bypass by
   scheme, not remove it.** `item.image_url.startsWith("http://")`
   (dev's `http://localhost:8080/...`) stays unoptimized — the container-
   networking constraint Phase 14 identified is still real. Production's
   `https://api.$DOMAIN/...` URLs no longer match, so Next's optimizer
   runs on them in production — gaining AVIF/WebP conversion and
   compression even though the backend still only ever serves one fixed
   pre-cropped size (Phase 14 decided no resized-variant generation is
   needed, and this phase doesn't revisit that). Flagged as an open risk
   below: this assumes the `web` container can reach `api.$DOMAIN` over
   the public internet in production, which is architecturally expected
   (Caddy fronts it, same as any browser) but unverified until deployed.
10. **Validation hardening stays hand-rolled, no new crate.**
    `is_valid_email` gets real structural checks (non-empty local part
    with no leading/trailing/consecutive dots and only legal characters;
    domain with at least one dot, non-empty alphanumeric-with-internal-
    hyphens labels, and an alphabetic TLD label of length ≥ 2; an overall
    254-character cap per RFC 5321) written in the same plain
    character-matching style `is_valid_phone` already uses — not a new
    `regex`/`validator` crate dependency. No client-side schema-validation
    library (e.g. `zod`) is introduced either; `contact-form.tsx`/
    `order-details-form.tsx` keep their existing manual `validate()`
    functions. Sanitization: React already escapes all rendered text by
    default (no `dangerouslySetInnerHTML` anywhere in the admin enquiries/
    orders views), so XSS from submitted text is already handled; this
    phase doesn't add a sanitization layer on top, just documents that
    baseline is sufficient.
11. **Accessibility fixes are targeted, not a new tooling pass.** No
    `axe-core`/automated a11y test suite is introduced. Concrete code
    changes: `aria-invalid`/`aria-describedby` on the two forms' erroring
    fields, explicit `focus-visible` utility classes on `site-header.tsx`'s
    nav links and hamburger button (matching `button.tsx`'s existing
    pattern). Contrast and full keyboard-navigation verification are a
    manual pass (`validation.md`), not a code change, since the color
    tokens are already fixed brand decisions from earlier phases.
    `admin-image-upload.tsx`'s `alt=""` crop-preview stays as-is — it's an
    admin-only, inherently decorative preview of what the admin just
    selected, not customer-facing content.

## Out of scope

- Real SES domain verification and requesting production sending access
  — a manual, pre-launch operations step (Decision 2), not phase code.
  Tracked as an open risk below and as a Phase 18 prerequisite.
- A local SMTP catcher (MailHog, Mailpit, etc.) for dev — Decision 2;
  dev simply runs with email disabled.
- Generating multiple resized image variants at upload time — Phase 14's
  decision to serve one fixed pre-cropped size stands; this phase only
  changes whether Next's optimizer is allowed to touch what's already
  being served (Decision 9).
- A client-side validation/schema library (`zod`, etc.) — Decision 10.
- CAPTCHA or any new spam/rate-limiting beyond what Phase 15 already
  built — still out of scope, as `tech-stack.md`'s Spam/Abuse Mitigation
  section already states.
- Automated accessibility testing tooling/CI gate — Decision 11; this
  phase is fixes plus a manual pass, not new infrastructure.
- A sitewide title-template restructure — Decision 6.
- Restyling or redesigning any page for the responsive QA pass beyond
  fixing concretely broken layouts found during manual testing — this
  phase fixes bugs, it doesn't redesign.

## Open risks flagged during planning

1. **Production image-optimization reachability is assumed, not yet
   verified.** Decision 9 depends on the `web` container being able to
   reach `https://api.$DOMAIN` from inside the production Docker network
   (it would, same as any public client, since Caddy fronts that
   hostname) — but this has never been exercised, since optimization was
   always bypassed before. If `next/image`'s server-side fetch fails
   there for some reason (firewall/DNS edge case), the fallback is
   reverting the `unoptimized` check to its pre-Phase-17
   `startsWith("http")` form — a one-line revert, not a redesign. Verify
   during this phase's own validation pass against a real deployed
   preview, not just local dev (local dev can't exercise this path at
   all, since dev always takes the `http://` bypass branch).
2. **AWS SES sandbox mode means no real end-to-end email test is
   possible during this phase's own validation** unless the user
   supplies a verified test recipient address and temporary SES SMTP
   credentials before `/validate` runs. Absent that, validation confirms
   the code path (logged "email disabled" no-op, or a mocked/local send
   attempt that fails gracefully) rather than an actual delivered email.
3. **Placeholder addresses must be swapped before go-live** — flagged
   again here so Phase 18's Definition of Done checklist doesn't miss it;
   this phase intentionally ships with fake defaults (Decision 2).

### Resolution (risk review with the user, 2026-10-09)

- **Risk 1** — the user has a separate staging/preview environment (not
  real Lightsail production) with the same `DOMAIN`/`PUBLIC_API_URL`
  wiring as prod. This phase's `/validate` pass verifies the production
  image-optimization path there instead of against real prod — lower
  blast radius, same networking shape. Staging host details (hostname,
  access) to be supplied when this phase reaches its validation pass;
  the one-line `unoptimized` revert stays the documented fallback if it
  fails.
- **Risk 2** — no test SES credentials/recipient are available yet.
  This phase's `/validate` pass is scoped to the no-op (email disabled)
  and fire-and-forget-failure code paths only; real end-to-end delivery
  stays unverified until credentials are supplied, which may not happen
  until Phase 18's SES production-access step.
- **Risk 3** — now also an explicit `roadmap.md` Phase 18 bullet (swap
  placeholder addresses, verify sending domain, request SES production
  access), not just a note buried in this phase's own risk list.
