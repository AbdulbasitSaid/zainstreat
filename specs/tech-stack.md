# Tech Stack

These are the decided technology choices for the MVP. The README's
"Suggested Technical Architecture" section intentionally left the stack
open; this document pins it down so implementation can start without
re-litigating infrastructure decisions.

## Architecture Overview

```text
┌────────────────────┐
│   Next.js (web)    │  Public site + cart UI + admin UI
└─────────┬──────────┘
          │ HTTPS / JSON
          ▼
┌────────────────────┐
│  Rust / Axum (api)  │  Business logic, auth, validation
└─────────┬──────────┘
          │
  ┌───────┼────────┐
  ▼       ▼        ▼
Postgres MinIO   Email
(data)  (images) (notifications)
```

A single Next.js app talks to a single Rust API over HTTPS/JSON. Both run
as containers on one AWS Lightsail VPS, fronted by a reverse proxy.

## Frontend

- **Next.js (App Router), React, TypeScript** — server-rendered pages for
  SEO (README §33) on public routes, with client components for the cart
  and admin interactivity.
- `apps/web` performs real data fetching as of Phase 6 (menu browsing) —
  server components call `apps/api` over HTTP using the server-only
  `API_BASE_URL` env var (Docker-internal DNS, same convention in dev and
  prod; see `docs/local-development.md`), always with `cache: "no-store"`.
- **Tailwind CSS v4** (replaced Pico CSS — see
  `specs/2026-10-02-phase-5-public-static-pages/requirement.md`'s
  "Addendum — Tailwind CSS v4 migration") — CSS-first configuration via an
  `@theme` block directly in `apps/web/app/globals.css` (no
  `tailwind.config.ts`), with the `@tailwindcss/postcss` plugin
  (`apps/web/postcss.config.mjs`). The README §3 brand palette, card
  radius, container width, section rhythm, and easing/animation tokens are
  all defined as native Tailwind design tokens (`bg-primary`, `rounded-
  card`, `max-w-brand`, `py-section`, `ease-out-expo`, `animate-marquee`)
  rather than raw custom properties aliased onto a third-party framework's
  token names. A handful of repeated multi-property patterns (the button
  treatment, the alternating text/image layout, the form-control look) are
  small shared React components (`apps/web/components/{button,split-row,
  check-list,notice,page-hero,eyebrow}.tsx`) rather than global CSS
  classes — `.container` and `.field` are the only names still kept in
  `@layer components`, since both apply identically everywhere with no
  conditional logic. `color-scheme: light` is still pinned explicitly (no
  dark-mode toggle — white background + food photography is the brand
  direction, README §34). Typography is unchanged: a self-hosted
  `next/font/google` display face for headings paired with a sans face for
  body copy (README §34), deliberately kept outside the Tailwind `@theme`
  namespace since `next/font/google` already owns those exact CSS custom
  property names.
- **`next/image`** for image optimization, lazy loading, and modern formats
  (README §33 performance requirements).
- **Strict TypeScript, no `any`:** `tsconfig.json`'s `strict: true` rejects
  implicit `any`; `eslint.config.mjs` additionally sets
  `@typescript-eslint/no-explicit-any` to `error` so explicit `any`
  annotations are rejected too. Use precise types, `unknown` with a type
  guard/narrowing, or a generic instead.

_Why not Astro for the marketing pages:_ a single Next.js app is simpler to
build, deploy, and maintain than two separate frontends, and Next.js still
meets the SEO/SSR requirements in README §33.

_Why Tailwind now, superseding Pico:_ Pico's classless model fit raw
semantic HTML, but this codebase is React-component-heavy (most views are
already own-built components, not bare tags), and a later redesign pass on
nav/forms/buttons/layout needed real per-component control Pico wasn't
giving without a fight — the footer newsletter input was already stripping
Pico's input chrome with `!important` overrides and had no replacement
focus-visible style at all. Tailwind v4's CSS-first `@theme` keeps the
same token-driven approach the team already liked about the Pico-remapping
setup, just as native Tailwind utilities instead of custom properties
aliased onto someone else's token names.

- **Motion** (`motion`, formerly Framer Motion) for scroll-triggered reveals,
  gesture/hover-driven animation, and the mobile nav/route transitions, plus
  **Lenis** for the global smooth-scroll feel (Phase 5 addendum — see
  `specs/2026-10-02-phase-5-public-static-pages/requirement.md`). Both ship
  native TypeScript types, so no `any` is needed under the no-`any` rule
  above. Motion/Lenis usage is confined to small `"use client"` wrapper
  components (`apps/web/components/{motion-provider,reveal,page-transition}.tsx`)
  imported into otherwise-server-component pages — Motion throws a build
  error if imported directly into a Server Component, so this boundary is
  enforced by the framework, not just convention. `prefers-reduced-motion`
  is respected at three layers: each animating component's
  `useReducedMotion()` check, Lenis never initializing under reduced
  motion, and a blanket CSS safety net in `globals.css`.

_Why Motion + Lenis over GSAP:_ Motion is React-idiomatic (declarative
`variants`/`whileInView`, hooks-based), which fits this codebase's existing
component model better than GSAP's imperative, DOM-ref-driven API — and it
keeps the dependency surface and learning curve smaller for a site this
size. GSAP's timeline/ScrollTrigger power isn't needed for the reveal/hover/
transition scope decided here; this can be revisited if a future phase
needs more elaborate scroll choreography.

_Why not plain CSS alone:_ CSS transitions/`@keyframes` handle hover,
press, focus, and the ambient decorative shapes fine (and are used for
exactly those), but can't do scroll-into-view triggering or animate a
disclosure panel's `height: auto` cleanly — Motion is reserved for those
two cases only, keeping the CSS/JS split deliberate rather than
all-or-nothing.

## Internationalization (i18n)

- **`next-intl@^4.14`** — locale-prefixed routing (`/en/...`, `/nl/...`),
  server/client translation hooks, and a `proxy.ts`-based middleware that
  auto-detects the visitor's locale from `Accept-Language` and persists an
  explicit override via a `NEXT_LOCALE` cookie.

Chosen over hand-rolled `Accept-Language` parsing or `next-i18next` (a
Pages-Router-era library retrofitted onto the App Router): `next-intl` has
first-class App Router + Server Component support and ships the exact
locale-prefix + auto-detect + cookie-override middleware this project
needs, with no custom redirect logic to build and maintain. Scope is the
public-facing site only — the admin dashboard and the Rust API stay
English-only (see `roadmap.md` Phase 3).

## Backend API

- **Rust + Axum** — the HTTP framework for the API service.
- **`sqlx`** — compile-time-checked SQL queries against Postgres (no ORM
  magic, matches the straightforward relational data model in README §31).
- **`serde`** — JSON (de)serialization for API request/response bodies.
- **Migrations**: sqlx's built-in migrator — plain numbered `.sql` files
  under `apps/api/migrations/`, embedded into the binary via
  `sqlx::migrate!()` and run automatically against the database on API
  startup. Fits this project's single-VPS, no-manual-approval continuous
  deployment (see Hosting & Deployment below) with no separate CI/deploy
  migration step to maintain (see `roadmap.md` Phase 4).
- **Offline query checking for CI**: `sqlx::query!`/`query_as!` macros
  need a live database (or a cached query catalog) at compile time, but
  `apps/api/Dockerfile`'s `cargo build --release` stage has no network
  access to Postgres. `apps/api/.sqlx/` (generated locally via
  `cargo sqlx prepare`, committed to the repo) plus
  `ENV SQLX_OFFLINE=true` in the Dockerfile's build stage lets the Docker
  build succeed with no live database.

_Why a separate Rust API instead of Next.js API routes:_ decided
explicitly — keeps business logic (pricing, order integrity, admin auth) in
one typed, testable service independent of the frontend framework.

## Database

- **PostgreSQL**, schema derived directly from README §31:
  `users`, `categories`, `menu_items`, `orders`, `order_items`.
- Soft deletes (`deleted_at`) on `categories` and `menu_items` to satisfy
  the archive-not-delete rule (README §28, §30, §41).
- `order_items` stores a denormalized `item_name` and `unit_price` captured
  at order time — menu price changes must never alter historical orders
  (README §31, §41).

## Admin Authentication (Rust-native)

- **`argon2`** crate for password hashing (memory-hard, current best
  practice for credential storage).
- **`tower-sessions`** with a Postgres-backed session store for
  server-side sessions and secure, httpOnly session cookies.

This satisfies README §26 (email/password login, secure session, logout)
entirely within the Rust service — no Node-based auth library (e.g.
Auth.js/NextAuth) is introduced, since the backend of record is Rust.
Future items from README §26 (multiple staff accounts, roles, 2FA,
password reset) extend this same session model later; they are not MVP
scope.

## Object/Image Storage

- **MinIO** (S3-compatible object storage), self-hosted as its own
  container alongside the app and database.

Satisfies the "object storage for images" element of README §40 while
staying inside the self-hosted-VPS decision. Because MinIO speaks the S3
API, a future migration to a managed S3-compatible provider (if the
business outgrows self-hosting) is a configuration change, not a rewrite.

## Email (P1 — order confirmations, contact/catering notifications)

- **`lettre`** crate, sending through a transactional email provider (e.g.
  Resend or Postmark) rather than a raw SMTP relay, for better
  deliverability.

This is P1 (README §33/§46 priority), not required for the MVP's core
ordering flow, but the dependency is named here so the `orders` and
`contact`/`catering` workflows can wire it in without a follow-up stack
decision.

## WhatsApp Integration

- **`wa.me` deep links** with a prefilled message (e.g.
  `https://wa.me/<number>?text=...`) for "WhatsApp Us" / "Order via
  WhatsApp" CTAs (README §21).

No WhatsApp Business API/webhook integration is needed for the MVP — it's
a link, not a service dependency. This keeps WhatsApp as a contact channel
alongside, not a replacement for, the online ordering system.

## Hosting & Deployment

- **AWS Lightsail VPS**, Ubuntu LTS blueprint, **`eu-central-1` (Frankfurt)**
  — AWS has no region physically in the Netherlands; Frankfurt is the
  closest full region and the standard low-latency choice for NL-based
  traffic. 2 GB RAM plan (comfortable headroom for Postgres + MinIO + the
  `web`/`api` containers + Caddy running concurrently), with a static IP
  attached.
- Lightsail's built-in firewall allows only `22` (SSH), `80`, and `443` —
  no other ports are exposed publicly.
- All services run via **Docker Compose**:
  - `web` — Next.js app
  - `api` — Rust/Axum API
  - `postgres` — database
  - `minio` — object storage
  - `caddy` — reverse proxy, automatic HTTPS via Let's Encrypt
- In production, only `caddy` publishes ports to the host (`80`/`443`);
  `web`, `api`, `postgres`, and `minio` are reachable solely over the
  internal Docker network. A `docker-compose.prod.yml` override expresses
  this (see `roadmap.md` Phase 2).
- **CI/CD:** GitHub Actions builds the `web` and `api` Docker images on
  every push to `master` and pushes them to a **private GHCR** registry,
  then deploys over SSH as a dedicated, sudo-less `deploy` user (`docker`
  group only — a separate key from the admin account used for manual
  access). The Lightsail box never builds images itself; it only pulls
  and runs them. True continuous deployment — no manual approval gate.
- **Backups:** Lightsail's automatic daily instance snapshots (built-in
  add-on) — no hand-rolled `pg_dump`/`mc mirror` scripts needed. This is
  provisioned in `roadmap.md` Phase 2; restore is verified in Phase 14
  once real data exists.

_Why self-hosted over a managed PaaS (e.g. Vercel + managed Postgres):_
decided explicitly — gives full control over the Rust API runtime and
keeps all infrastructure (compute, DB, storage) on one bill and one box,
which fits a small single-business deployment.

_Why Lightsail over raw EC2:_ decided explicitly — Lightsail's flat
monthly price already bundles a static IP, a data-transfer allowance, and
snapshot backups, so there's no separate Elastic IP/EBS/security-group
bookkeeping to maintain for a single-box deployment. Raw EC2 would offer
more flexibility the business doesn't need yet; it can be revisited if the
app ever outgrows one Lightsail instance.

_Why build in CI and push to GHCR instead of building on the box:_ a 2 GB
Lightsail instance is shared with Postgres, MinIO, and live traffic —
running a Rust release compile and a Next.js build on it too is wasteful
and would periodically starve the running services. GitHub's hosted
runners absorb that cost for free, and the box's job shrinks to just
"pull and run."

## Summary Table

| Concern | Choice |
|---|---|
| Frontend | Next.js (App Router) + React + TypeScript + Tailwind CSS v4 |
| Animation | Motion (scroll/gesture/transitions) + Lenis (smooth scroll) + plain CSS (hover/focus/decorative) |
| i18n | next-intl (locale-prefixed `/en`, `/nl` routing) |
| Backend API | Rust + Axum + sqlx + serde |
| Database | PostgreSQL |
| Admin auth | argon2 + tower-sessions (Postgres-backed sessions) |
| Image/object storage | MinIO (S3-compatible, self-hosted) |
| Email (P1) | lettre + transactional email provider |
| WhatsApp | `wa.me` deep links |
| Hosting | AWS Lightsail VPS (`eu-central-1`, Frankfurt), Docker Compose |
| CI/CD | GitHub Actions → private GHCR → SSH deploy (dedicated `deploy` user) |
| Backups | Lightsail automatic daily snapshots |
| Reverse proxy / TLS | Caddy (automatic HTTPS) |
