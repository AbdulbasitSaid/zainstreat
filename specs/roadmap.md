# Roadmap

High-level implementation order for the MVP, broken into small,
dependency-ordered phases. Each phase should produce something that can be
run and checked on its own before moving to the next. No calendar estimates
— phases are ordered by what has to exist before the next thing can be
built, per `mission.md` and `tech-stack.md`.

Future work beyond the MVP (payments, notifications at scale, customer
accounts, analytics, etc.) is intentionally excluded — see "Beyond MVP"
below.

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

- SEO: page titles, meta descriptions, Open Graph tags, semantic HTML,
  clean URLs (`/`, `/about`, `/services`, `/menu`, `/order`, `/contact`,
  `/terms`, `/food-regulations`).
  (Note: since Phase 3 introduced locale-prefixed routing, every URL above
  actually lives under `/en/...` or `/nl/...` — e.g. `/en/menu`, `/nl/menu`.
  This is a necessary small addendum to this phase's own requirement.md
  when it's written.)
- Accessibility: keyboard navigation, visible focus states, form labels,
  alt text, sufficient contrast.
- Image optimization pass (formats, lazy loading).
- Responsive QA across mobile, tablet, desktop.
- Input validation/sanitization hardening on all forms and API endpoints.
- Wire in email sending (order confirmations, contact/catering
  notifications) via `lettre` + transactional email provider.

## Phase 18 — Production Hardening & Final Rollout

The AWS Lightsail instance, DNS, TLS, backups, production Dockerfiles, and
the GitHub Actions CI/CD pipeline already exist from Phase 2 — this phase
hardens what's running on them rather than standing anything up from
scratch.

- Harden `apps/web/Dockerfile` and `apps/api/Dockerfile` (non-root user,
  smaller/distroless base image where reasonable, resource limits).
- Verify a Lightsail snapshot can actually be restored (test restore) now
  that real menu/order data exists.
- Production smoke test against the README §43 Definition of Done
  checklist, run against the live domain (not localhost).

## Beyond MVP (not in this roadmap)

Deliberately deferred — see README §39 for detail:

- **Phase 2 (later):** online payments, WhatsApp/email order notifications
  at scale, customer order tracking, delivery fee calculation, event
  booking calendar, discount codes, catering packages.
- **Phase 3 (later):** customer accounts, order history, loyalty program,
  multiple administrators/staff roles, inventory management, sales
  reports/analytics.
- **Phase 4 (later):** broader food-business operations platform (kitchen
  workflow, pickup/delivery operations, integrated notifications).
