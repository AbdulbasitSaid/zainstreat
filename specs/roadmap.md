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

## Phase 2 — Data Model

- Postgres migrations for `users`, `categories`, `menu_items`, `orders`,
  `order_items` (per `tech-stack.md` / README §31).
- Soft-delete columns (`deleted_at`) on `categories` and `menu_items`.
- Seed script with a few sample categories/menu items for local development.

## Phase 3 — Public Static Pages

- Home, About, Contact (static shell), Terms & Conditions, Food Regulations
  pages — no dynamic data yet.
- Brand system applied: color tokens, typography, logo placement, base
  layout/navigation (desktop + mobile hamburger).

## Phase 4 — Menu Browsing (Read-Only)

- API: list categories, list available menu items (with category filter).
- Menu page: category filtering, item cards (name, description, price,
  image), empty-category state, unavailable-item state.

## Phase 5 — Cart

- Client-side cart: add item, change quantity, remove item, subtotal.
- Empty-cart state and "continue shopping" / "proceed to order" actions.
- Unavailable items cannot be added to the cart.

## Phase 6 — Order Submission

- Customer details form (name, phone, email, pickup/delivery, notes).
- Order review step.
- API: submit order — persists `orders` + `order_items`, capturing item
  name/price at the time of the order (not a live reference to `menu_items`).
- Order confirmation page showing the order number and next steps.

## Phase 7 — Admin Auth

- Login page.
- API: argon2 password verification, session creation via
  `tower-sessions` (Postgres-backed), logout.
- Middleware protecting all `/admin` routes and admin API endpoints.

## Phase 8 — Admin: Orders

- Dashboard shell (nav: Dashboard, Orders, Menu, Categories, Settings).
- Order list (ID, customer, total, status) and order detail view (items,
  quantities, totals, delivery/pickup info, notes).
- Update order status.

## Phase 9 — Admin: Menu & Categories

- Add / edit menu item (name, description, price, category, image,
  available, featured).
- Archive menu item (soft delete; remains on historical orders).
- Availability toggle (available/unavailable).
- Category management: add, rename, archive.

## Phase 10 — Catering & Contact Workflows

- Catering/event enquiry form (name, phone, email, event type, date,
  guests, location, services required, message) — separate from the food
  cart, per the non-negotiable rule in `mission.md`.
- General contact form (name, email, phone, subject, message).
- Submissions persisted and, once Phase-11 email is wired in, forwarded by
  email to the business.

## Phase 11 — WhatsApp CTAs

- `wa.me` deep links placed in header, hero, contact page, and footer
  (per `tech-stack.md`).

## Phase 12 — Polish & Non-Functional Requirements

- SEO: page titles, meta descriptions, Open Graph tags, semantic HTML,
  clean URLs (`/`, `/about`, `/services`, `/menu`, `/order`, `/contact`,
  `/terms`, `/food-regulations`).
- Accessibility: keyboard navigation, visible focus states, form labels,
  alt text, sufficient contrast.
- Image optimization pass (formats, lazy loading).
- Responsive QA across mobile, tablet, desktop.
- Input validation/sanitization hardening on all forms and API endpoints.
- Wire in Phase-11-style email sending (order confirmations, contact/
  catering notifications) via `lettre` + transactional email provider.

## Phase 13 — Deployment

- Dockerize all services for production; Caddy reverse proxy with
  automatic HTTPS on the VPS.
- Backups configured for Postgres and MinIO.
- Production smoke test against the README §43 Definition of Done
  checklist.

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
