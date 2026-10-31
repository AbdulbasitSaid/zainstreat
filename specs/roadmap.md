# Roadmap

High-level implementation order for the MVP, broken into small,
dependency-ordered phases. Each phase should produce something that can be
run and checked on its own before moving to the next. No calendar estimates
— phases are ordered by what has to exist before the next thing can be
built, per `mission.md` and `tech-stack.md`.

Future work beyond the MVP (customer accounts, loyalty, inventory, etc.)
is intentionally excluded — see "Beyond MVP" below. Phases 20 and 22–29
were added from the 2026-10-18 client call
(`specs/client-notes/2026-10-18-client-call.md`), which pulled
bank-transfer payments, order tracking, owner notifications and analytics
forward from "Beyond MVP" ahead of the public launch. Phase 21 (pipeline
refactor) was inserted on 2026-10-31.

## Phase 1 — Scaffolding

- Repo layout for the Next.js app and the Rust/Axum API.
- `docker-compose` for local dev: `web`, `api`, `postgres`, `minio`.
- Basic health-check route on the API; basic "hello" page on the web app,
  both reachable through the dev Compose stack.

## Phase 2 — AWS Deployment Infrastructure & CI/CD

Deploying early, right after scaffolding, so every later phase ships to a
real environment instead of one big-bang deploy at the end (per
`tech-stack.md`'s hosting decision).

- Provision an AWS Lightsail instance: Ubuntu LTS blueprint, 2 GB plan,
  `eu-central-1` (Frankfurt — closest AWS region to the Netherlands).
  Attach a static IP, configure the Lightsail firewall (22/80/443 only),
  install Docker + the Compose plugin.
- Create a dedicated, sudo-less `deploy` system user on the box (`docker`
  group only) with its own GitHub Actions-only SSH keypair — kept
  separate from the admin key used for manual/break-glass access.
- Author basic production multi-stage Dockerfiles: `apps/web/Dockerfile`
  (Next.js standalone build, `node:22-bookworm-slim` runtime) and
  `apps/api/Dockerfile` (`cargo build --release`, copied onto
  `debian:bookworm-slim`). Deliberately basic — not yet hardened
  (no non-root user, no distroless base); that polish is Phase 18's job.
- `.github/workflows/deploy.yml`: on every push to `master`, build the
  `web` and `api` images, push them to a **private** GHCR registry, then
  SSH to the Lightsail box as the `deploy` user and run `docker compose
  -f docker-compose.yml -f docker-compose.prod.yml pull && ... up -d`.
  True continuous deployment — no manual approval step. No test suite
  exists yet, so nothing gates the deploy on tests passing.
- Point the business domain's DNS A record(s) at the static IP.
- `docker-compose.prod.yml` override: `web`/`api` reference prebuilt
  `ghcr.io/<owner>/zainstreat-web:latest` / `...-api:latest` images
  (no `build:` context, no bind mounts — the server only pulls and runs);
  adds a `caddy` service (reverse proxy, automatic HTTPS for the domain);
  `postgres`, `minio`, `web`, and `api` stop publishing ports to the
  public host — only `caddy` is internet-facing, everything else talks
  over the internal Docker network. The server authenticates to GHCR with
  a scoped read-only PAT configured once by hand (never committed).
- Enable Lightsail's automatic daily snapshot add-on — this resolves
  `tech-stack.md`'s previously-open "backup mechanism" item for now (see
  Phase 18 for verifying an actual restore once real data exists).
- `docs/deployment.md`: how the CI/CD flow works (push to `master`, watch
  Actions, live), plus a manual/break-glass rollback runbook (SSH as
  admin, pull a previous image tag and `up -d`, or re-run a prior
  successful Actions run).

## Phase 3 — Internationalization (Dutch/English)

- `next-intl` wired into `apps/web` with locale-prefixed routing
  (`/en/...`, `/nl/...`), English as the default/fallback locale.
- Middleware (`apps/web/proxy.ts`) auto-detects the visitor's browser
  language (`Accept-Language`) on first visit with no stored preference,
  redirecting to the matching locale; a manual toggle always overrides
  this, persisted via the `NEXT_LOCALE` cookie so it sticks across visits.
- Translation message files (`messages/en.json`, `messages/nl.json`)
  established as the convention every later phase's UI copy will extend.
- The existing hello page (`app/page.tsx`) migrated under `app/[locale]/`
  and its hardcoded strings moved into the message files, as the first
  real usage example — plus a minimal placeholder language-toggle control
  on that page (no real header/nav exists until Phase 5).
- Scope: public-facing site only. The admin dashboard (Phase 11/13) and
  the Rust API (`apps/api`) remain English-only.

## Phase 4 — Data Model

- Postgres migrations for `users`, `categories`, `menu_items`, `orders`,
  `order_items` (per `tech-stack.md` / README §31).
- Soft-delete columns (`deleted_at`) on `categories` and `menu_items`.
- Seed script with a few sample categories/menu items for local development.

## Phase 5 — Public Static Pages

- Home, About, Services, Contact (static shell), Terms & Conditions, Food
  Regulations pages — no dynamic data yet.
- Services page (Meals, Snacks, Catering, Event Rentals detail sections
  per README §12, each with a "Request a Quote" CTA linking to Contact —
  the enquiry form itself is Phase 15's job).
- Brand system applied: color tokens, typography, logo placement, base
  layout/navigation (desktop + mobile hamburger).

## Phase 6 — Menu Browsing (Read-Only)

- API: list categories, list available menu items (with category filter).
- Menu page: category filtering, item cards (name, description, price,
  image), empty-category state, unavailable-item state.

## Phase 7 — Menu Price Variants

- Data model: `display_order` on `categories`/`menu_items` so the public
  menu's ordering can be controlled explicitly, and a
  `menu_item_price_options` child table so one item can offer several
  priced sizes/options (e.g. a soup in 2 L or 3 L, a bulk order as a full
  or half size) instead of a single flat price.
- API: `/api/menu-items` returns a nullable flat `price` plus a
  `price_options` array (empty when the item is flat-priced); both list
  endpoints order by `display_order`.
- Menu page: item cards render either the existing single price or a
  label/price list per option — display only, no selection UI yet.
- Full replacement of the Phase 4/6 placeholder menu seed data with the
  real client menu.

## Phase 8 — Telemetry & Observability

Not an original roadmap phase — inserted after the site's first
production 502 revealed there was no way to see why. Self-hosted only
(no external SaaS), consistent with the single-VPS hosting decision in
`tech-stack.md`.

- `apps/api`: structured logging (`tracing` + `tracing-subscriber`),
  request-level logging with latency and a request ID
  (`tower_http::trace::TraceLayer` + `request_id`), and `sqlx::Error`
  variants mapped to distinct HTTP status codes (404/409/500/503)
  instead of everything collapsing to a bare 500.
- Replace the `eprintln!`/panicking `.expect()`/`.unwrap()` calls in
  `main.rs`/`error.rs` with structured `tracing::error!` logging before a
  controlled exit, plus a panic hook.
- Caddy: JSON access/error logs on every site block, so a 502 (upstream
  unreachable) is timestamped and correlatable with the API's own logs.
- Dozzle (self-hosted, browser-based log viewer) added to
  `docker-compose.prod.yml`, reverse-proxied through Caddy and gated by
  HTTP Basic Auth (it can read every container's logs, so it isn't
  exposed directly).
- `restart: unless-stopped` and bounded `json-file` log retention
  (`max-size`/`max-file`) added to every prod Compose service — a
  crashed `api` container currently stays down until someone manually
  SSHes in, which directly compounds a 502.
- `docs/deployment.md`: new "Viewing production logs" section.

## Phase 9 — Cart

- Client-side cart: add item, change quantity, remove item, subtotal.
- Empty-cart state and "continue shopping" / "proceed to order" actions.
- Unavailable items cannot be added to the cart.

## Phase 10 — Order Submission

- Customer details form (name, phone, email, pickup/delivery, notes).
- Order review step.
- API: submit order — persists `orders` + `order_items`, capturing item
  name/price at the time of the order (not a live reference to `menu_items`).
- Order confirmation page showing the order number and next steps.

## Phase 11 — Admin Auth

- Login page.
- API: argon2 password verification, session creation via
  `tower-sessions` (Postgres-backed), logout.
- Middleware protecting all `/admin` routes and admin API endpoints.

## Phase 12 — Visual & Interactive Refresh

Not an original roadmap phase — inserted ahead of the admin-dashboard work
at the client's request, to make the public site "more interactive and more
beautiful" before continuing the MVP build-out.

- Remove the decorative "floating shapes" (`DecorativeShape`) entirely from
  the homepage and about page — no replacement decoration, just gone.
- Replace Framer Motion (the `motion` package) with GSAP + ScrollTrigger as
  the site's animation system, for page transitions and scroll-triggered
  reveals; Lenis (smooth scroll) stays, driven off `gsap.ticker`.
- Fill in the site's missing imagery (homepage, about, services, contact)
  with seeded stock/placeholder photography until real brand photography is
  available — real photos remain a Phase 17 "Image optimization pass"
  follow-up (or sooner, once the client supplies them).

## Phase 13 — Admin: Orders

Builds on the authentication seam Phase 11 deliberately left behind: the
protected `/api/admin/*` route group + `require_admin` middleware, and the
`app/admin/(protected)/` route group whose bare "Welcome, {name}" page this
phase replaces.

- Dashboard shell (nav: Dashboard, Orders, Menu, Categories, Settings) —
  all five rendered, with Menu/Categories (Phase 14) and Settings (no phase
  yet) as visibly disabled, non-navigable items rather than links to
  dead ends.
- Dashboard landing page (`/admin`): per-status order counts, a "placed
  today" count, and the five most recent orders as links into detail.
- Order list (ID, customer, type, total, status, placed-at) with a status
  filter and `limit`/`offset` pagination, newest first.
- Order detail view (items, option labels, quantities, unit prices, line
  subtotals, order totals, delivery/pickup info and address, notes,
  timestamps).
- Update order status — any of the six `orders.status` CHECK values from
  any other, no enforced forward-only workflow.
- Still read-only for everything else: no editing an order's items,
  customer details or totals, and no deleting orders.

## Phase 14 — Admin: Menu & Categories

- Add / edit menu item (name, description, price or multiple priced size
  options, category, image, available, featured).
- Image upload: the first real use of the MinIO container running since
  Phase 1 — admins upload a photo from their device rather than pasting a
  URL, served back out through a new public `GET /api/media/{key}` route
  on the already-public `api.{$DOMAIN}` Caddy site (see
  `specs/2026-10-04-phase-14-admin-menu-categories/requirement.md`
  Decisions 1–4, 10, 12–13 for the full upload/serve/env-var design).
- Archive menu item (soft delete; remains on historical orders).
- Availability toggle (available/unavailable) — a dedicated one-field
  endpoint, separate from the full edit form.
- Category management: add, rename, archive — archiving is blocked with
  a `409` while the category still has active (non-archived) menu items.

## Phase 15 — Catering & Contact Workflows

- Catering/event enquiry form (name, phone, email, event type, date,
  guests, location, services required, message) — separate from the food
  cart, per the non-negotiable rule in `mission.md`. Lives on the existing
  `/contact` page (the Phase 5 Services page CTAs already link there) as a
  second form, switched to via a tab alongside the general contact form.
- General contact form (name, email, phone, subject, message) — the
  Phase 5 static stub on the Contact page, wired up for real.
- Submissions persisted and, once Phase-17 email is wired in, forwarded by
  email to the business. No status workflow on either (unlike Orders).
- A minimal read-only admin view (`/admin/enquiries`) listing both, so
  staff aren't blind to submissions before Phase 17's email lands.

## Phase 16 — WhatsApp CTAs

Hero (Phase 5/12), the footer (Phase 5), and the contact page (Phase 5/15)
already grew bare `https://wa.me/<number>` links ad hoc while those phases
were built. This phase closes the actual gaps: the still-missing header
CTA, and the prefilled `?text=` message `tech-stack.md` already specifies
but no existing link implements.

- A new icon-only WhatsApp CTA in the header (`site-header.tsx`), visible
  on both desktop and mobile — the one placement from README §9.1 that
  doesn't exist yet.
- `apps/web/lib/whatsapp.ts`: a shared `WHATSAPP_NUMBER` constant and a
  `buildWhatsAppLink(message)` helper, replacing the number hardcoded
  three times (hero, footer, contact page).
- All four placements (header, hero, footer, contact page) link through
  `buildWhatsAppLink` with the same generic prefilled message (translated
  per locale), and consistently open in a new tab
  (`target="_blank" rel="noopener noreferrer"` — the existing hero link is
  missing this today).

## Phase 17 — Polish & Non-Functional Requirements

- SEO: `generateMetadata` (title/description) added to every public page
  that's still missing it (Home, About, Services, Contact, Terms, Food
  Regulations — Menu/Cart/Order already had it since their own phases); a
  sitewide `metadataBase` + default Open Graph image (the logo) on the
  root locale layout; a locale-aware `app/sitemap.ts` and `app/robots.ts`.
  Clean URLs are already satisfied by the existing App Router structure
  (`/en|nl/`, `/about`, `/services`, `/menu`, `/order`, `/contact`,
  `/terms`, `/food-regulations` — locale-prefixed since Phase 3).
- Accessibility: `aria-invalid`/`aria-describedby` wiring on the Contact
  and Order forms' error messages, explicit `focus-visible` styling on
  the header's nav links/hamburger button, plus a manual keyboard-nav/
  contrast pass — no new a11y-testing dependency.
- Image optimization pass: `next.config.ts` AVIF/WebP `images.formats`,
  enabling `next/image`'s optimizer for production-served uploaded photos
  (currently always bypassed; the dev-only unreachable-host case is now
  isolated instead of blanket-disabled), `sizes` props on menu/cart
  images, and an audit of `priority` vs. default lazy-loading on
  above/below-the-fold images.
- Responsive QA across mobile, tablet, desktop — manual pass, any bugs
  found get fixed within this same phase.
- Input validation/sanitization hardening: tightens `is_valid_email` in
  `apps/api/src/validation.rs` (flagged in its own comment as deferred to
  this phase) with real structural checks, no new crate; confirms
  existing React-escaping/honeypot/rate-limit protections are sufficient
  otherwise.
- Wire in email sending (order confirmations to the customer,
  contact/catering notifications to the business) via `lettre`'s SMTP
  transport against **Amazon SES** (supersedes this doc's original
  "Resend or Postmark" naming — see `tech-stack.md`'s Email section).
  Fire-and-forget: a failed send is logged, never blocks or fails the
  underlying order/enquiry request. Ships with placeholder
  sender/business-notify addresses; real domain verification + SES
  production-access request is a manual pre-launch step, not phase code.

## Phase 18 — Production Hardening & Final Rollout

The AWS Lightsail instance, DNS, TLS, backups, production Dockerfiles, and
the GitHub Actions CI/CD pipeline already exist from Phase 2 — this phase
hardens what's running on them rather than standing anything up from
scratch.

Expanded after the 2026-10-11 production outage (the instance went fully
unresponsive, most likely from memory exhaustion; see
`specs/2026-10-11-phase-18-production-hardening/requirement.md`):

- Incident recovery + evidence capture, and a written "When the site is
  down" runbook in `docs/deployment.md`.
- Upgrade the Lightsail instance from 2 GB to **4 GB** (snapshot → new
  instance → move the static IP). This doubles as the snapshot-restore
  test below.
- Host hardening: 2 GB swap file and a persistent systemd journal.
- Per-container memory/CPU limits in `docker-compose.prod.yml`.
- Reduce `next/image` optimizer load: WebP only (drop Phase 17's AVIF),
  a 30-day `minimumCacheTTL`, and a `web_cache` volume so the cache
  survives deploys.
- Deploys prune dangling images.
- Off-box monitoring: Lightsail metric alarms (CPU, burst capacity,
  status checks) plus an external HTTP uptime monitor, both emailing the
  owner.
- Harden `apps/web/Dockerfile` and `apps/api/Dockerfile` (non-root user;
  Debian-slim bases kept rather than distroless; resource limits applied
  in compose).
- Verify a Lightsail snapshot can actually be restored (test restore) now
  that real menu/order data exists.
- Swap Phase 17's placeholder `EMAIL_FROM_ADDRESS` /
  `EMAIL_BUSINESS_NOTIFY_ADDRESS` values for real ones, verify the
  sending domain in the SES console, and request SES production access
  (exit the sandbox) — flagged as an open risk in that phase's
  requirement.md so it isn't missed here.
- Production smoke test against the README §43 Definition of Done
  checklist, run against the live domain (not localhost).

## Phase 19 — Menu Photo Fix

Added after a 2026-10-11 production bug: replacing a menu photo in admin
broke it on the public site (see
`specs/2026-10-11-phase-19-menu-photo-fix/requirement.md`).

- Pass `DOMAIN` into the web image at build time (CI build arg →
  Dockerfile `ARG`), so `next/image`'s `remotePatterns` allows
  `api.$DOMAIN`. Standalone output saves `next.config.ts` at build time.
  The same fix corrects `robots.txt`/`sitemap.xml`, which currently point
  to `localhost`.
- Fail the web Docker build if `DOMAIN` is missing.
- Cart thumbnails fall back to the placeholder when a photo was replaced,
  and re-adding an item refreshes its stored photo.
- Remove a dead A record (`18.192.249.64`) from DNS.

## Phase 20 — Bank-Transfer Payment & Order Lifecycle

From the 2026-10-18 client call: food is paid **before** it's prepared,
and only verified payments are processed. No payment provider yet — the
customer pays by bank transfer and the owner verifies the bank alert by
hand. No VAT is added (the business is under the KOR scheme).

- Order lifecycle reworked around payment, with branches for pickup and
  delivery: *awaiting payment* → *payment claimed* (customer says they
  paid) → *confirmed / processing* (owner verified the payment; one
  admin action, automatically moving the order to processing) → *ready
  for collection* (pickup) or *on the way* (delivery) → *completed*, plus
  *cancelled*. Replaces the current `new/confirmed/preparing/ready/
  completed/cancelled` CHECK values via a migration that maps existing
  orders.
- An invoice for each order (order number, items, total, the business's
  bank account details and payment reference), shown on the confirmation
  page and emailed to the customer. States clearly that only verified
  payments are processed.
- An "I've paid" action for the customer on their order.
- Admin: a "Confirm payment" action and status transitions that follow
  the order's pickup/delivery type. The checkout's existing
  pickup/delivery choice (Phase 10) drives the branch.
- The owner is emailed on every new order and every "I've paid" claim.

## Phase 21 — Pipeline Refactor: Staging, No-SSH Ops, `cms.` Admin

Inserted 2026-10-31 (see
`specs/2026-10-31-phase-21-pipeline-refactor/requirement.md`).

- A **staging** environment on an ephemeral Lightsail instance: a
  "staging up" workflow creates it from a launch script, deploys and
  seeds it; "staging down" deletes it, so it costs nothing while unused.
  No custom domain — it's served on `<ip>.sslip.io`.
- Merging to `master` builds images tagged by commit SHA and deploys
  staging (when it's up). Production deploys only through a manual,
  approval-gated "promote" of an already-built SHA; rollback is a promote
  of an older SHA.
- Remove routine manual SSH: the server `.env` is rendered from GitHub
  Environment secrets on every deploy; new instances are bootstrapped by
  script; `create_admin`, backup (to S3) and restore become workflows.
  AWS access from Actions uses OIDC, not stored keys.
- Admin moves from `/admin` to **`cms.zainstreat.com`** (`cms.<ip>.sslip.io`
  on staging); old `/admin` links redirect.

## Phase 22 — Customer Order Tracking & Status Emails

- A public order-status page reachable by an unguessable link (no
  customer accounts), showing the order's progress through the Phase 20
  lifecycle.
- An email to the customer on each status change (payment confirmed,
  ready for collection / on the way, completed), linking to that page.
- Completion: the customer can confirm they received or collected the
  order. The owner can also complete it from admin.
- New orders appear in admin without a reload while it's open (polling
  is enough; real push is Phase 27).

## Phase 23 — Catering Quotations

Event orders stay out of the cart flow (`mission.md`). This phase builds
on Phase 15's catering enquiry.

- Admin: turn a catering enquiry into a quotation (line items, total,
  notes) and send it to the customer by email.
- Customer accepts the quote via a link. From there it follows the same
  bank-transfer → owner confirms → processing → pickup/delivery →
  completed lifecycle as Phase 20/22.

## Phase 24 — French Locale

- Add `fr` as a third public locale alongside `en`/`nl` (`next-intl`
  routing, `messages/fr.json`, language toggle, sitemap,
  `Accept-Language` detection). Admin and the API stay English-only.
- Translate the customer emails into French as well.

## Phase 25 — Admin-Editable Site Content

- Admin can replace site imagery that is currently hardcoded: homepage
  images, the kitchen photo, the contact-page photo, and the **logo**,
  all uploaded to MinIO like menu photos (Phase 14).
- Admin can edit the homepage text, per locale.
- Swap all remaining stock/placeholder homepage imagery for the client's
  real food photos (some supplied already; the client is sending the
  rest).

## Phase 26 — Analytics & Search Presence

- Google Analytics (GA4) on the public site only (never the `cms.` admin), with a
  cookie-consent banner, which GDPR/Dutch rules require for analytics
  cookies.
- Google Search Console: verify the domain and submit the sitemap. The
  client wants to be findable on Google but **without publishing the
  business address** for now, so no address in structured data or
  Google Business Profile.
- A follow-up SEO pass based on what Search Console and Analytics show.

## Phase 27 — Installable Admin (PWA) & Push Notifications

To be investigated first: the client uses an iPhone, and iOS only
supports web push for home-screen-installed PWAs (iOS 16.4+).

- Web app manifest and service worker so the admin can be installed to
  the home screen.
- Web push to the owner for new orders and payment claims, even when the
  admin isn't open. Tapping a notification opens the order in admin.

## Phase 28 — WhatsApp Owner Notifications

- WhatsApp Business (Cloud) API integration on the client's existing
  WhatsApp Business account: the owner gets a WhatsApp message for new
  orders, payment claims and catering enquiries, with a link to the
  admin.
- Fire-and-forget like email (Phase 17): a failed send is logged and
  never blocks an order.

## Phase 29 — Move to Client-Owned AWS Account

Deadline-driven: the developer's AWS free trial expires a few weeks
after 2026-10-18, so this phase may need to happen before Phases 27–28.

- Recreate the Lightsail instance, static IP, snapshots, alarms and SES
  identity in the client's own AWS account. Phase 21's bootstrap script,
  OIDC role and backup/restore workflows do most of this: provision,
  restore the latest S3 backup, repoint DNS. Migrate Postgres and MinIO
  data, repoint DNS and the CI deploy secrets.
- Update `docs/deployment.md`. Decommission the old instance only after
  the new one is verified live.

## Later — iDEAL Payments

Agreed in the 2026-10-18 call but blocked on the client opening a
payment-provider account (needs their business details).

- iDEAL as a second checkout payment option next to bank transfer,
  confirmed automatically by webhook instead of manual verification.
- VAT: a payment platform reports revenue, so VAT must be handled for
  these payments (added to the price or absorbed by the business).
  Decide before building.

## Beyond MVP (not in this roadmap)

Deliberately deferred — see README §39 for detail:

- **Phase 2 (later):** delivery fee calculation, event booking calendar,
  discount codes, catering packages.
- **Phase 3 (later):** customer accounts, order history, loyalty program,
  multiple administrators/staff roles, inventory management, sales
  reports.
- **Phase 4 (later):** broader food-business operations platform (kitchen
  workflow, pickup/delivery operations, integrated notifications).
