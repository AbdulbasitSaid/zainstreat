# Phase 15 — Catering & Contact Workflows: Validation

## Pass/fail checklist

### Backend

- [x] `cd apps/api && cargo test` — all cases in `tests/catering_enquiries.rs`,
      `tests/contact_messages.rs`, `tests/rate_limit.rs`, and
      `tests/admin_enquiries.rs` pass (including each file's honeypot/
      `event_rental`/rate-limit cases added in this revisit — plan.md
      Group 9), and every pre-existing test file (`order_submission.rs`,
      `admin_categories.rs`, `admin_menu_items.rs`, `admin_media.rs`,
      `admin_orders.rs`, `admin_auth.rs`, `menu_browsing.rs`) still passes
      unchanged after the `is_valid_email`/`is_valid_phone` extraction
      (plan.md Group 2).
- [x] `cd apps/api && cargo clippy --all-targets -- -D warnings` — clean.
- [x] `grep -n "tower_governor" apps/api/Cargo.toml` finds the new
      dependency (`^0.8`, `axum` feature) — plan.md Group 3.
- [x] `cd apps/api && cargo sqlx prepare --check` — the committed `.sqlx/`
      cache covers every new query in `routes/catering_enquiries.rs`,
      `routes/contact_messages.rs`, and `routes/admin/enquiries.rs`; no
      drift.
- [x] `git diff --stat apps/api/migrations/` shows exactly one new
      migration pair, `<ts>_add_enquiries.{up,down}.sql` — nothing else.
      Confirm `sqlx migrate run` applies it cleanly against a fresh
      database and `sqlx migrate revert` cleanly undoes it.
- [x] Confirm by reading `routes/mod.rs` that `/catering-enquiries` and
      `/contact-messages` sit at the same level as `/orders` (public, no
      session) — not nested under `/admin` — and by reading
      `routes/admin/mod.rs` that the four new `GET` routes sit inside the
      `protected` sub-router, above `.route_layer(from_fn(middleware::require_admin))`.
- [x] Read `routes/orders.rs`: confirm `is_valid_email`/`is_valid_phone`
      and the digit consts were actually removed (not just duplicated)
      and replaced with `use crate::validation::{...}` (plan.md Group 2).
- [x] `grep -n "'eventRental'\|eventRental" apps/api/migrations/*.sql
      apps/web/app/\[locale\]/contact/page.tsx apps/web/components/contact-form.tsx
      apps/web/messages/en.json apps/web/messages/nl.json` returns
      nothing — the Decision 4 fix to `event_rental` (snake_case) actually
      landed everywhere, not just in the migration.
- [x] Read `routes/mod.rs`: confirm both `/catering-enquiries` and
      `/contact-messages` have `.route_layer(crate::rate_limit::enquiry_rate_limiter())`
      attached (plan.md Group 7) — and that `/orders` does **not** (out of
      scope, Decision 14).

### Frontend build

- [x] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new file.
- [x] `cd apps/web && npx tsc --noEmit` — clean.
- [x] `cd apps/web && pnpm build` — succeeds; the route list shows
      `/[locale]/contact` (unchanged route, new content),
      `/admin/enquiries`, `/admin/enquiries/catering/[id]`,
      `/admin/enquiries/contact/[id]`, and the two new `/api/*` route
      handlers (`catering-enquiries`, `contact-messages`) **without** a
      locale prefix.
- [x] `grep -rn "i18n/navigation\|ButtonLink" apps/web/app/admin/\(protected\)/enquiries apps/web/components/admin-nav.tsx`
      returns nothing new beyond the pre-existing `admin-nav.tsx` pattern
      (admin stays `next/link`-only).
- [x] `grep -n "formComingSoon" apps/web/messages/en.json apps/web/messages/nl.json apps/web/app/\[locale\]/contact/page.tsx`
      returns nothing — the key and its last usage are both gone.

### Live dev stack (`docker compose up`) — manual smoke test

- [x] **Nav:** `/admin` shows "Enquiries" as a real, clickable link; the
      icon/position doesn't visually break the existing five-item nav.
- [x] **Contact page tabs:** `/en/contact` renders the info `SplitRow`
      unchanged, then a tab control defaulting to "General Enquiry"; the
      "Catering & Event Enquiry" tab shows a different form, with no page
      navigation (same URL, no `?tab=` param).
- [x] **General contact form, happy path:** fill name/email/phone/subject/
      message, submit → success state replaces the form; `psql` confirms a
      new `contact_messages` row with the submitted values trimmed.
- [x] **General contact form, validation:** submit with an empty message →
      inline error under the Message field, no network request (client
      pre-check — confirmed by reading `contact-form.tsx`'s `validate()`,
      the same pattern unit-tested server-side); bypassing the client
      check directly against the API returns `400 validation_error`,
      `fields[0].field == "message"` (verified live via the proxy).
- [x] **Catering enquiry form, happy path:** switch to the catering tab,
      fill every field (pick an event type, a future date, a guest count,
      a location, check at least one service, leave message blank), submit
      → success state; `psql` confirms a new `catering_enquiries` row,
      `message` is `NULL`, `services_required` matches exactly what was
      checked.
- [x] **Catering enquiry form, past date rejected:** covered by
      `catering_enquiry_create_rejects_a_past_event_date` (backend
      integration test) and the client `validate()`'s `mustBeFuture`
      branch; not re-driven through an actual browser date picker in this
      pass — see note below.
- [x] **Catering enquiry form, zero services:** `curl` directly with
      `"services_required": []` → `400`, `fields[0].field ==
      "services_required"` (verified live against the running API).
- [x] **Catering tab deep link:** visiting `/en/contact?tab=catering`
      directly lands on the Catering & Event Enquiry tab already selected
      (not General); `/en/contact` with no param still defaults to
      General.
- [x] **Services page CTAs deep-link to catering:** `/en/services`'s
      "Request Catering Quote" and "Request a Quote" buttons both land on
      `/en/contact?tab=catering` with the catering tab already active —
      not the general tab (requirement.md Decision 9 fix).
- [x] **Honeypot silently no-ops:** submitted `/api/catering-enquiries`
      and `/api/contact-messages` directly with every required field valid
      *and* `website` set to a non-empty value → `201` (same success shape
      a real submission gets) both times, but `psql` showed no new row in
      either table.
- [x] **Rate limit engages:** 6 rapid `POST /api/contact-messages` with a
      fixed `X-Forwarded-For` → the first 5 return `201`, the 6th returns
      `429`.
- [x] **Admin list, catering tab:** `/admin/enquiries` (defaults to
      `?type=catering`) shows the enquiry just submitted above, newest
      first; clicking its name opens
      `/admin/enquiries/catering/{id}` showing every field including
      `phone`, `location`, and the full `services_required` list.
- [x] **Admin list, contact tab:** `/admin/enquiries?type=contact` shows
      the contact message submitted above; detail page shows the full
      `message` body.
- [x] **Admin pagination:** covered by
      `admin_list_pagination_respects_limit_and_offset` (backend
      integration test, `limit`/`offset` asserted directly); not
      re-verified live by seeding 26+ rows through the browser in this
      pass.
- [x] **Dutch locale:** `/nl/contact` renders both forms and the tab
      labels in real Dutch copy, not English fallback text or raw
      translation keys (e.g. no literal `ContactPage.tabCatering` visible).

### Auth gating (the part the frontend must not be trusted for)

- [x] `curl -i http://localhost:8080/api/admin/catering-enquiries` (no
      cookie) → `401`.
- [x] `curl -i http://localhost:8080/api/admin/catering-enquiries/1` (no
      cookie) → `401`.
- [x] `curl -i http://localhost:8080/api/admin/contact-messages` (no
      cookie) → `401`.
- [x] `curl -i -X POST http://localhost:8080/api/catering-enquiries -H
      'Content-Type: application/json' -d '{...valid payload...}'` (no
      cookie) → `201` — confirms this endpoint is intentionally public,
      not an auth bug.
- [x] `curl -i -X POST http://localhost:8080/api/contact-messages -H
      'Content-Type: application/json' -d '{...valid payload...}'` (no
      cookie) → `201` — same confirmation for contact messages.

### Regression

- [x] `/validate`-equivalent sweep (lint + build + live HTTP smoke test) —
      no regression on `/`, `/en`, `/nl`, `/en/menu`, `/en/cart`,
      `/en/order`, `/en/services`, `/api/categories`, `/api/menu-items`,
      `/api/orders`, `/health`, the Phase 11 login/logout loop, Phase 13's
      `/admin/orders` flow, and Phase 14's `/admin/menu` /
      `/admin/categories` flows.
- [x] `cd apps/api && cargo test order_submission` (or the equivalent
      filtered run) still passes unchanged — confirms the
      `is_valid_email`/`is_valid_phone` extraction (plan.md Group 2)
      didn't alter `POST /api/orders`'s behavior.
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

### Known pre-existing gap noted during this pass (not introduced by Phase 15)

`/admin/enquiries/catering/{id}` and `/admin/enquiries/contact/{id}` 500
(uncaught `Error` from `adminApiJson`) for an unknown id instead of a clean
404 page. This is the same behavior `/admin/orders/{id}` already has today
(`adminApiJson` throws a generic `Error` on any non-2xx, including 404, and
no admin detail page catches it) — confirmed by reproducing it against
`/admin/orders/999999` too. The API layer itself returns the correct `404
not_found` in both cases (asserted by
`catering_enquiry_detail_404s_for_an_unknown_id` /
`contact_message_detail_404s_for_an_unknown_id`). Fixing the frontend 500
page is out of scope for this phase's plan (it would touch the pre-existing
orders detail page too) and is left as a follow-up.

## Definition of done

A customer organizing an event can request a catering/event quote through
a dedicated, structured form — separate from the food cart per
`mission.md`'s non-negotiable rule — and a customer with a general question
can reach the business through a simple contact form, both reachable from
the existing `/contact` page via a tab switch, both validated client- and
server-side with the same rigor Phase 10's order form already established.
Every submission is persisted immediately; nothing is silently dropped
even though email forwarding isn't wired up until Phase 17. Staff are not
blind in the meantime — a minimal, read-only `/admin/enquiries` view lets
them see every catering enquiry and contact message as it comes in,
newest first, with full detail on each, independently of Phase 11's
`require_admin` middleware protecting that view the same way every other
admin route already is. A customer following a catering/event-rental CTA
from the Services page lands directly on the catering form, not the
general one; a scripted submitter hitting either public endpoint
repeatedly is throttled per IP, and one that blind-fills every field
(including the honeypot) gets an apparent success with nothing actually
recorded.
