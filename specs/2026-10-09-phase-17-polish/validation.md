# Phase 17 — Polish & Non-Functional Requirements: Validation

## Pass/fail checklist

### Frontend build

- [x] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new/changed
      file.
- [x] `cd apps/web && npx tsc --noEmit` — clean.
- [x] `cd apps/web && pnpm build` — succeeds; new routes `/sitemap.xml`
      and `/robots.txt` appear in the build output route list.

### Backend build

- [x] `cd apps/api && cargo build --release` — succeeds with `lettre`
      added; `SQLX_OFFLINE=true` if running without a live DB connection
      (matches the Dockerfile's build-stage convention). (Verified with
      `cargo build`, debug profile; release build uses the same code
      path.)
- [x] `cd apps/api && cargo test` — the new `is_valid_email` unit tests
      (if added per plan.md Group 4) pass. Full suite (118 tests across
      all integration + unit tests) passes against the running dev stack.
- [x] `cd apps/api && cargo clippy` — clean (no new warnings introduced).

### SEO (manual + curl against the running dev stack)

- [x] `curl -s http://localhost:3000/en | grep -o '<title>[^<]*</title>'`
      and the `/nl` equivalent — every one of the 9 public pages (`/`,
      `/about`, `/services`, `/menu`, `/cart`, `/order`, `/contact`,
      `/terms`, `/food-regulations`) returns a non-generic, page-specific
      `<title>` in both locales (not just the bare "Zain's Treat n More"
      fallback).
- [x] View source on at least one page per locale — `<meta
      name="description">` and `<meta property="og:title">`/`og:description`/
      `og:image` are present and populated (not empty/missing).
- [x] `curl -s http://localhost:3000/sitemap.xml` — returns valid XML with
      one `<url>` entry per locale per content route (14 total: 7 routes
      × `en`/`nl`), and does **not** include `/cart` or `/order`.
- [x] `curl -s http://localhost:3000/robots.txt` — disallows `/admin` and
      `/api`, references the sitemap URL.
- [x] `apps/web/app/[locale]/layout.tsx`'s `metadataBase` resolves
      correctly — no `TypeError: Invalid URL` at build or runtime when
      `DOMAIN` is unset locally (should fall back to
      `http://localhost:3000`). Confirmed: dev build/runtime used the
      `http://localhost:3000` fallback with no errors.

### Accessibility (manual)

- [ ] Tab through the Contact form and Order form with keyboard only —
      every field reachable, visible focus ring on each (including the
      header hamburger button and nav links, now with explicit
      `focus-visible` styling).
- [ ] Trigger a validation error on the Contact form (e.g. submit empty)
      — inspect the DOM: the errored `<input>` has `aria-invalid="true"`
      and `aria-describedby` pointing at the error `<p>`'s `id`. Same
      check on the Order form.
- [ ] Screen reader spot-check (VoiceOver/NVDA, whichever is available)
      on the Contact form: submitting with an error announces the error
      text when the field receives focus, not silently.
- [ ] Contrast spot-check: primary text/background, button
      text/background, and form-field error text (red-ish) against the
      cream background all meet WCAG AA (4.5:1 for normal text) — use a
      browser contrast checker, no new tooling installed for this.
- [ ] Mobile hamburger menu: `aria-expanded` toggles correctly, closed
      panel is `inert` (already correct pre-Phase-17 — regression check
      only).

### Image optimization

- [x] With a real uploaded menu-item photo (via `/admin/menu`) and the
      **dev** stack running: `grep -n "unoptimized" apps/web/components/
      {menu-item-card,cart-line-item,admin-menu-list}.tsx` shows the
      `.startsWith("http://")` form in all three, and the image still
      renders correctly in dev (unoptimized bypass still applies there).
- [ ] Against the user's separate staging/preview environment (same
      `DOMAIN`/`PUBLIC_API_URL=https://api.$DOMAIN` wiring as prod —
      requirement.md's Open Risks Resolution, not real Lightsail
      production) — confirm a real uploaded menu-item photo now actually
      gets optimized (check the response `Content-Type` of the
      `/_next/image?...` request in dev tools is `image/webp` or
      `image/avif`, not the original `image/jpeg`) and isn't broken (this
      is the open risk from requirement.md — if it's broken, revert the
      `unoptimized` condition to its pre-Phase-17 form and note it in this
      file before shipping).
- [x] `grep -n "formats" apps/web/next.config.ts` shows
      `["image/avif", "image/webp"]`.
- [x] No new raw `<img>` tags introduced (`grep -rn "<img" apps/web/app
      apps/web/components` still shows only the pre-existing
      `admin-image-upload.tsx` one).

### Responsive QA (manual)

- [ ] Walk `/`, `/menu`, `/cart`, `/order`, `/services`, `/contact` at
      ~375px (mobile), ~768px (tablet), and ~1440px (desktop) in both
      locales — no horizontal scroll, no overlapping text/images, the
      floating WhatsApp button (Phase 16) doesn't obscure the cart/
      checkout CTA or the contact form's submit button at any of the
      three widths.
- [ ] Any layout bug found during this pass gets fixed within this same
      phase before checking this box, not deferred.

### Input validation hardening

- [x] `cargo test` output confirms (or manual `curl -X POST
      /api/contact-messages` spot checks confirm) `is_valid_email` now
      rejects: `""`, `"no-at-sign"`, `"a@b"` (no TLD dot),
      `"a@.com"` (empty label), `"a@b..com"` (consecutive dots),
      `"a@b-.com"` (label ends in hyphen), and a 300-character address —
      and still accepts `"name@example.com"` and
      `"a.b+c@sub.example.co.uk"`.
- [x] Regression: existing valid submissions on all three forms (orders,
      contact, catering) that passed validation before this phase still
      pass (no false-positive rejections introduced). Full `cargo test`
      suite — including `order_submission.rs`, `contact_messages.rs`,
      `catering_enquiries.rs` — passes (118 tests, 0 failed).

### Email

- [x] Local dev, no SES env vars set: submit a real order and a real
      contact message — both still succeed (`201`), and the API logs
      show `"email sending disabled"` (startup) and
      `"email disabled, skipping send"` (debug level, per submission) —
      no panic, no error, no delay in the HTTP response. Verified live
      against the dev stack. Note: the per-submission debug log only
      fires for orders (`create_order` always calls `spawn_send`);
      contact/catering only call it when `business_notify_address()` is
      `Some`, so with email disabled they skip the call entirely
      (`plan.md` Group 5f/5g's own designed behavior) — no functional
      gap, just no extra debug line for those two routes when disabled.
      **Found and fixed during this validation pass:** `docker-
      compose.yml` defaults each SES/email var to an empty string rather
      than leaving it unset, so `std::env::var` originally returned
      `Ok("")` instead of `Err` — `EmailConfig::from_env()` would have
      treated blank values as "configured" and attempted real sends with
      empty credentials on every submission. Fixed in `email.rs` to treat
      blank env vars the same as unset ones.
- [ ] If the user has supplied temporary SES SMTP credentials and a
      verified test recipient before this validation pass: set all five
      env vars, submit a real order, confirm the confirmation email
      actually arrives at the test address with the order number and
      total. Submit a real contact message, confirm the business-notify
      address receives it. **As of this phase's risk review, no test
      credentials are available** (requirement.md's Open Risks
      Resolution) — skip these two and note them as not-yet-verified
      rather than checking the box; real delivery stays unverified until
      credentials are supplied, possibly as late as Phase 18.
- [x] Simulate a failed send (e.g. deliberately wrong SMTP password) —
      confirm the order/contact-message request still returns `201` and
      the row is still persisted; only the log shows
      `"failed to send email"` (requirement.md Decision 3). Verified live
      with temporary fake SMTP credentials against the real SES SMTP
      endpoint: order returned `201` immediately and persisted; the
      spawned send failed asynchronously a few seconds later with
      `permanent error (535): Authentication Credentials Invalid`, logged
      via `tracing::error!` and never surfaced to the HTTP response.
- [x] `grep -n "lettre" apps/api/Cargo.toml` confirms the dependency; no
      `aws-sdk-sesv2` or other new AWS SDK crate was added (tech-stack.md
      Decision — SMTP only).

### Regression

- [x] `/validate`-equivalent sweep (lint + build + live HTTP smoke test)
      — no regression on `/`, `/en`, `/nl`, `/en/menu`, `/en/cart`,
      `/en/order`, `/en/services`, `/en/contact`, `/api/categories`,
      `/api/menu-items`, `/health` all return `200` against the rebuilt
      dev stack; full `cargo test` suite (118 tests across admin auth,
      categories, enquiries, media, menu items, orders, contact messages,
      catering enquiries, menu browsing, order submission, rate limiting)
      passes with 0 failures. Phase 11 login/logout and Phase 13/14/15/16
      admin/WhatsApp-CTA flows were not re-walked manually in a browser
      this pass — covered by the existing automated test suite, not a
      live click-through.
- [ ] CI green on PR.

## Definition of done

Every public page has a real, page-specific title/description and
Open Graph tags; `/sitemap.xml` and `/robots.txt` exist and are correct.
The Contact and Order forms' validation errors are announced to assistive
tech, and the header's interactive elements show a visible focus state.
Production-served menu-item photos are optimized (format conversion,
`sizes`-aware) instead of always bypassing `next/image`'s optimizer; dev
keeps its necessary bypass. `is_valid_email` performs real structural
validation instead of a bare "has an `@` and a dot" check. Order
confirmations and contact/catering business notifications send through
Amazon SES via `lettre`'s SMTP transport, fire-and-forget — a failed or
disabled send never blocks, delays, or fails the underlying
order/enquiry request, and local dev runs with zero SES setup required.
Real SES domain verification, production-access approval, and swapping
the placeholder from/notify addresses for real ones remain manual,
pre-launch operations steps, tracked here and in Phase 18's Definition of
Done rather than done as part of this phase's code.
