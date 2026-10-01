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
  (no non-root user, no distroless base); that polish is Phase 14's job.
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
  Phase 14 for verifying an actual restore once real data exists).
- `docs/deployment.md`: how the CI/CD flow works (push to `master`, watch
  Actions, live), plus a manual/break-glass rollback runbook (SSH as
  admin, pull a previous image tag and `up -d`, or re-run a prior
  successful Actions run).

## Phase 3 — Data Model

- Postgres migrations for `users`, `categories`, `menu_items`, `orders`,
  `order_items` (per `tech-stack.md` / README §31).
- Soft-delete columns (`deleted_at`) on `categories` and `menu_items`.
- Seed script with a few sample categories/menu items for local development.

## Phase 4 — Public Static Pages

- Home, About, Contact (static shell), Terms & Conditions, Food Regulations
  pages — no dynamic data yet.
- Brand system applied: color tokens, typography, logo placement, base
  layout/navigation (desktop + mobile hamburger).

## Phase 5 — Menu Browsing (Read-Only)

- API: list categories, list available menu items (with category filter).
- Menu page: category filtering, item cards (name, description, price,
  image), empty-category state, unavailable-item state.

## Phase 6 — Cart

- Client-side cart: add item, change quantity, remove item, subtotal.
- Empty-cart state and "continue shopping" / "proceed to order" actions.
- Unavailable items cannot be added to the cart.

## Phase 7 — Order Submission

- Customer details form (name, phone, email, pickup/delivery, notes).
- Order review step.
- API: submit order — persists `orders` + `order_items`, capturing item
  name/price at the time of the order (not a live reference to `menu_items`).
- Order confirmation page showing the order number and next steps.

## Phase 8 — Admin Auth

- Login page.
- API: argon2 password verification, session creation via
  `tower-sessions` (Postgres-backed), logout.
- Middleware protecting all `/admin` routes and admin API endpoints.

## Phase 9 — Admin: Orders

- Dashboard shell (nav: Dashboard, Orders, Menu, Categories, Settings).
- Order list (ID, customer, total, status) and order detail view (items,
  quantities, totals, delivery/pickup info, notes).
- Update order status.

## Phase 10 — Admin: Menu & Categories

- Add / edit menu item (name, description, price, category, image,
  available, featured).
- Archive menu item (soft delete; remains on historical orders).
- Availability toggle (available/unavailable).
- Category management: add, rename, archive.

## Phase 11 — Catering & Contact Workflows

- Catering/event enquiry form (name, phone, email, event type, date,
  guests, location, services required, message) — separate from the food
  cart, per the non-negotiable rule in `mission.md`.
- General contact form (name, email, phone, subject, message).
- Submissions persisted and, once Phase-13 email is wired in, forwarded by
  email to the business.

## Phase 12 — WhatsApp CTAs

- `wa.me` deep links placed in header, hero, contact page, and footer
  (per `tech-stack.md`).

## Phase 13 — Polish & Non-Functional Requirements

- SEO: page titles, meta descriptions, Open Graph tags, semantic HTML,
  clean URLs (`/`, `/about`, `/services`, `/menu`, `/order`, `/contact`,
  `/terms`, `/food-regulations`).
- Accessibility: keyboard navigation, visible focus states, form labels,
  alt text, sufficient contrast.
- Image optimization pass (formats, lazy loading).
- Responsive QA across mobile, tablet, desktop.
- Input validation/sanitization hardening on all forms and API endpoints.
- Wire in email sending (order confirmations, contact/catering
  notifications) via `lettre` + transactional email provider.

## Phase 14 — Production Hardening & Final Rollout

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
