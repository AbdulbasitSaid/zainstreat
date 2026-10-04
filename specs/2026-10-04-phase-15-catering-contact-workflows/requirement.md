# Phase 15 — Catering & Contact Workflows: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 15 — the
original three bullets, expanded by this phase's planning to record the
decisions below:

> - Catering/event enquiry form (name, phone, email, event type, date,
>   guests, location, services required, message) — separate from the food
>   cart, per the non-negotiable rule in `mission.md`.
> - General contact form (name, email, phone, subject, message).
> - Submissions persisted and, once Phase-17 email is wired in, forwarded by
>   email to the business.

Also grounded in README §20 ("Catering / Event Enquiry") and §22 ("Contact
Page" / "Contact Form"), which supply the exact field lists and enum
option lists used below.

## Context

This is `mission.md`'s business goal 4 ("Make WhatsApp an easy... channel")
and goal 2 ("Increase customer enquiries")'s non-WhatsApp counterpart, and
directly implements the non-negotiable rule: *"Catering/event enquiries are
a quote workflow, separate from the food cart — a two-meal order should
never require a large catering form."* Phase 9/10 already gave customers
the food-cart path; this phase gives them the other two paths from
README §21's "two customer paths" diagram (quick WhatsApp chat is Phase 16;
structured catering/contact enquiries are this phase).

Pieces already on disk this phase builds on rather than rebuilds:

- `apps/web/app/[locale]/contact/page.tsx` — Phase 5's static shell already
  renders the general contact form's fields (name, email, phone, subject,
  message) as a disabled `<form>` with a "coming soon" `Notice`, plus the
  phone/email/WhatsApp/hours/social list. This phase replaces the disabled
  form with a working one; the rest of the page (the `SplitRow` info list)
  is untouched.
- `apps/web/app/[locale]/services/page.tsx` — the Catering and Event
  Rentals `SplitRow` sections already have `ButtonLink href="/contact"`
  CTAs ("Request Catering Quote" / "Request a Quote"), per Phase 5's own
  requirement.md note that "the enquiry form itself is Phase 15's job".
  Confirms the catering form's page: `/contact`, not a new route — also
  confirmed by `roadmap.md` Phase 17's canonical clean-URL list (`/`,
  `/about`, `/services`, `/menu`, `/order`, `/contact`, `/terms`,
  `/food-regulations`), which has no separate `/catering` entry.
- `apps/api/src/routes/orders.rs` — the only existing precedent for a
  public (unauthenticated), validated, persisted write endpoint. Its
  `is_valid_email`/`is_valid_phone` helpers are reused (Decision 3), not
  reimplemented.
- `apps/web/app/api/orders/route.ts` — the same-origin Route Handler proxy
  pattern (browser → Next.js route → `apps/api`) this phase's two new
  public proxies copy exactly.
- `apps/web/components/order-details-form.tsx` / `lib/orders.ts` — the
  client-side validate-then-submit shape (a `validate()` function, a
  `FieldErrors` record, a `SubmitXResult` discriminated union consuming the
  `{"error":"validation_error","fields":[...]}` envelope) this phase's two
  new forms copy.
- `apps/web/components/admin-nav.tsx`, `lib/admin-api.ts`
  (`adminApiFetch`/`adminApiJson`), `apps/web/app/admin/(protected)/layout.tsx`,
  the Phase 13 orders list/detail page shape (`lib/admin-orders-api.ts`'s
  `limit`/`offset` pagination, the `Field` detail-page helper) — reused for
  this phase's new minimal admin view.
- `apps/api/src/lib.rs`'s `AppState`/`FromRef` (Phase 14) — untouched; this
  phase's handlers only need `State<PgPool>`, same as `orders.rs`.

Branches off `master` (Phase 14 already merged; this phase touches no file
unique to any other unmerged branch).

## Decisions

1. **Two new tables, not one shared "enquiries" table with a
   discriminator column.** `catering_enquiries` and `contact_messages` have
   materially different shapes (9 fields including an array column vs. 5
   flat fields) — same one-concern-per-table precedent as `orders` staying
   separate from `menu_items`. Rejected a single polymorphic table: it
   would need a nullable superset of both field lists, which is exactly
   the "half the columns are always null" shape this schema avoids
   elsewhere.

2. **Both submissions are an immutable, unauthenticated write with no
   status/workflow column.** Unlike `orders.status`, nothing in this phase
   ever transitions a catering enquiry or contact message from one state to
   another — decided with the user specifically to keep the admin view
   read-only (see Decision 8). No `updated_at` either: the row is written
   once at submission time and never changed again, so there is nothing
   for it to track. `created_at` (`TIMESTAMPTZ NOT NULL DEFAULT now()`) is
   the only timestamp either table needs.

3. **`apps/api/src/validation.rs` (new): `is_valid_email`/`is_valid_phone`
   extracted out of `orders.rs` verbatim**, which now imports them instead
   of defining them inline. Both new public create handlers
   (`catering_enquiries.rs`, `contact_messages.rs`) use the same two
   functions — without the extraction, this phase would be the third
   copy-paste of logic (including the ascending/descending-digit fake-number
   guard) that's already nontrivial. `orders.rs`'s own behavior is
   unchanged; this is a pure move, not a rewrite. The frontend keeps its
   own independent TypeScript copy per form (Decision 6) — same
   client/server duplication precedent `order-details-form.tsx` already
   has against `orders.rs`, not a new pattern.

4. **Enum values stored as plain `TEXT` with a `CHECK (... IN (...))`
   constraint**, same convention as `orders.delivery_type`/`orders.status` —
   validated again at the application layer before insert (same
   belt-and-suspenders the order/menu-item code already does), not relied
   on as the only validation:
   - `catering_enquiries.event_type`: `'wedding' | 'birthday' | 'corporate'
     | 'family_gathering' | 'outdoor_event' | 'other'` (README §20's event
     types, snake_cased — no existing precedent to match, so this phase's
     own convention, consistent with every other DB enum in this schema).
   - `contact_messages.subject`: `'general' | 'catering' | 'event_rental' |
     'menu' | 'order' | 'other'` — snake_case, consistent with every other
     enum in this schema. Phase 5's static `<option value="eventRental">`
     markup in `contact/page.tsx` is camelCase, but that markup is still
     part of the disabled "coming soon" stub this phase replaces wholesale
     (Group 13) — it has never been wired to a submit handler and no row
     has ever been written with that value, so renaming it costs nothing
     and isn't "changing shipped behavior." Originally planned as a
     deliberate camelCase exception (see open risk 1 in the prior revision
     of this document); revisited and fixed once it was confirmed nothing
     depends on the old value — see "Risks resolved in this revisit" below.

5. **`catering_enquiries.services_required` is a `TEXT[] NOT NULL`
   column, not a join table.** README §20 lists four fixed options
   (`catering`, `meal_delivery`, `event_rental`, `other`) with no admin
   management need (unlike `categories`, which admins create/rename) — a
   fixed small enum array needs no relational table. Validated
   application-side: every element must be one of the four allowed values
   (same pattern as Decision 4), and the array must be non-empty (a
   catering enquiry naming zero services is meaningless) — enforced in
   Rust, not a DB `CHECK`, since a `CHECK` across every element of an array
   column needs a non-trivial SQL expression, not worth it for a single
   invariant application code already validates everywhere else in this
   schema (e.g. Phase 7's flat-price/price-options invariant is also
   app-layer only).

6. **`catering_enquiries.message` is nullable (optional); `contact_messages.message`
   is `NOT NULL` and must be non-empty.** The catering form's eight other
   structured fields (event type, date, guest count, location, services)
   already convey the core request, so `message` there is genuinely
   supplementary detail. The general contact form has no other structured
   content — a blank message would be a submission with nothing to say.
   Decided without a user prompt as a straightforward product-sense call;
   flagged here rather than silently assumed in case it should go the
   other way.

7. **`catering_enquiries.event_date` must not be in the past at
   submission time** (`DATE`, checked in Rust against
   `chrono::Utc::now().date_naive()`, not a DB constraint — `now()` isn't
   stable across a migration/restore the way a `CHECK` needs to be, same
   reasoning the schema already avoids time-relative `CHECK`s elsewhere).
   `guest_count` is a positive `INTEGER` (`CHECK (guest_count > 0)`, same
   style as `order_items.quantity`). `location` has the same
   `ADDRESS_MIN_LENGTH`-style minimum-length check `orders.rs` already
   applies to `delivery_address`.

8. **Admin view is read-only: list + detail, no status field, no edit, no
   delete, no archive** — decided with the user over adding this phase's
   scope (a bare "see what came in" view closes the Phase-17 email gap
   without building a second order-status-style workflow for something
   that isn't actually a workflow yet). One new nav item, **"Enquiries"**
   (`apps/web/components/admin-nav.tsx`), landing on `/admin/enquiries`
   which defaults to listing catering enquiries with a tab to switch to
   contact messages — the same tab/toggle UX as the public Contact page
   (Decision 9), reused here for consistency rather than inventing a
   second nav structure. Detail pages:
   `/admin/enquiries/catering/{id}` and `/admin/enquiries/contact/{id}`.
   No dashboard landing-page changes (no new summary counts on `/admin`) —
   kept out of scope to match "minimal" (see Out of scope).

9. **The public `/contact` page shows one form at a time behind a
   tab/toggle — "General Enquiry" / "Catering & Event Enquiry" — rather
   than both forms stacked.** Decided with the user: the catering form's
   nine fields (including a multi-select) made "both forms, one after
   another, on one page" feel like a wall of inputs on mobile (this is a
   mobile-first site per `mission.md`). The toggle is still plain
   client-side state (`useState`) — switching tabs after the page has
   loaded doesn't become a URL navigation — but it now **seeds its initial
   value from an optional `?tab=catering` search param**, read once on
   mount, so a link can land a visitor directly on the catering form.
   Originally deferred as "no URL-state plumbing" (see open risk 2 in the
   prior revision of this document); revisited since the cost turned out
   to be one read of `searchParams`, not real routing — see "Risks
   resolved in this revisit" below. Services page CTAs for both Catering
   and Event Rentals now link to `/contact?tab=catering` instead of bare
   `/contact` (both are catering-form topics — Event Rentals is one of
   the catering form's `services_required` options).

10. **Two new public endpoints, registered directly on
    `routes::api_router()`** (same level as `/orders`, `/categories`,
    `/menu-items` — not nested under `/admin`, no session required):
    - `POST /api/catering-enquiries` → `routes/catering_enquiries.rs`
    - `POST /api/contact-messages` → `routes/contact_messages.rs`

    Two new same-origin Next.js Route Handler proxies with matching paths
    (`apps/web/app/api/catering-enquiries/route.ts`,
    `apps/web/app/api/contact-messages/route.ts`), copying
    `apps/web/app/api/orders/route.ts` verbatim (read the request body as
    text, forward with `Content-Type: application/json`, relay status +
    body back) — the browser never calls `apps/api` directly, same posture
    as every other write in this codebase since Phase 10.

11. **Four new admin (session-protected) `GET` endpoints, in one new
    module `apps/api/src/routes/admin/enquiries.rs`** (not split into two
    files the way `categories.rs`/`menu_items.rs` are) — both resources are
    thin, read-only, and share the exact same list/detail shape, so one
    proportionately small file covers both rather than two near-empty
    ones:
    - `GET /api/admin/catering-enquiries` — `limit`/`offset` pagination,
      newest first, same `ListOrdersQuery`-style shape as
      `routes/admin/orders.rs` minus the `status` filter (nothing to filter
      by — see Decision 8).
    - `GET /api/admin/catering-enquiries/{id}`
    - `GET /api/admin/contact-messages` — same pagination shape.
    - `GET /api/admin/contact-messages/{id}`

    No Route Handler proxy needed for any of these four — Server
    Components call them directly via `adminApiJson` (Phase 13's
    `lib/admin-orders-api.ts` precedent), which forwards the session
    cookie server-to-server without a browser round trip.

12. **Admin stays English-only; the two new public forms get real
    translations in both `apps/web/messages/{en,nl}.json`** (new
    `ContactPage.catering*`/`.tabs*` keys alongside the existing
    `ContactPage` keys, plus a shared `errors.*` block matching `Order`'s
    `{required, invalid, tooShort, tooLong}` shape exactly) — direct
    continuation of Phase 3's i18n scope (public site translated, admin
    not) and Phase 11 Decision 8 / Phase 13 Decisions 13–14 (admin forms
    use `next/link`, never `@/i18n/navigation`'s `Link`). The now-unused
    `ContactPage.formComingSoon` key is removed from both message files
    rather than left dangling.

13. **Backend gets integration tests** (`apps/api/tests/catering_enquiries.rs`,
    `apps/api/tests/contact_messages.rs`, `apps/api/tests/admin_enquiries.rs`).
    **The frontend continues to have none** — validated manually via
    `/validate`'s live smoke test, same precedent as every prior phase.

14. **Both public endpoints get a honeypot field plus a per-client-IP rate
    limit (`tower_governor`)** — not CAPTCHA, and `POST /api/orders`
    (Phase 10) is intentionally left as-is, out of scope here. Full design
    (the custom `X-Forwarded-For`-based key extractor this app's
    server-to-server proxy architecture requires, honeypot behavior,
    chosen limits) is pinned in `tech-stack.md`'s new "Spam/Abuse
    Mitigation (Phase 15)" section. Originally planned as "no protection,
    same as Phase 10" (see open risk 3 in the prior revision of this
    document); revisited with the user — see "Risks resolved in this
    revisit" below.

## Out of scope

- Email forwarding of either submission to the business — explicitly
  Phase 17's job (`lettre` is already pinned in `tech-stack.md` for this).
- Any status/workflow on a catering enquiry or contact message (contacted,
  quoted, closed, etc.) — Decision 2/8. If the business needs this later,
  it's a follow-up phase, not a silent addition here.
- Editing or deleting a submission from the admin view, or archiving one.
- Dashboard (`/admin`) summary counts or "N new enquiries" badges —
  Decision 8 keeps this phase's admin footprint to the nav item + list/
  detail pages only.
- A dedicated `/catering` (or similar) URL — Decision 9/Context; the form
  lives on `/contact` behind a tab, now reachable via a `?tab=catering`
  deep link rather than a route of its own.
- Spam/abuse protection *beyond* a honeypot and a per-IP rate limit —
  specifically, no CAPTCHA, no account/verification requirement. Decision
  14.
- A map/location embed on the Contact page (`mapPlaceholderLabel`'s
  "coming soon" state is untouched — not this phase's concern).

## Risks resolved in this revisit (2026-10-04)

This phase's initial planning pass flagged three open risks and
deliberately deferred all three. Before implementation started, they were
revisited with the user and all three were resolved rather than carried
forward:

1. **`contact_messages.subject` casing** — originally kept as camelCase
   (`eventRental`) to match Phase 5's static, never-submitted `<option>`
   markup verbatim. Since that markup is part of the disabled stub this
   phase replaces outright, and no row has ever been written with that
   value, there was no real backward-compatibility cost to fixing it.
   Resolved: switched to `event_rental`, snake_case like every other enum
   in the schema (Decision 4).
2. **Services page CTAs not deep-linking to the catering tab** —
   originally deferred to avoid adding URL-state plumbing to a
   deliberately simple `useState` tab toggle. Resolved: the toggle now
   reads an optional `?tab=catering` search param once on mount to seed
   its initial state (still plain client state after that — switching tabs
   doesn't itself navigate), and both Services page CTAs link to
   `/contact?tab=catering` (Decision 9).
3. **No spam/abuse protection on either new public endpoint** — originally
   accepted at current traffic scale, same posture as `POST /api/orders`.
   Resolved: a honeypot field on both forms plus a per-client-IP rate
   limit (`tower_governor`) on both endpoints, stopping short of CAPTCHA
   (Decision 14, `tech-stack.md`'s new "Spam/Abuse Mitigation (Phase 15)"
   section). The IP-extraction design needed real care — this app's
   browser traffic never reaches `apps/api` directly, only through
   `apps/web`'s same-origin proxy, so peer-IP rate limiting on `apps/api`
   alone would have rate-limited the `web` container, not individual
   visitors. See `tech-stack.md` for the resolution (a custom
   `X-Forwarded-For`-based key extractor, with the proxy forwarding the
   real client IP through).
