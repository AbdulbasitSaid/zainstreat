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

- **GSAP** (`gsap`, `@gsap/react`'s `useGSAP` hook, `ScrollTrigger`,
  `CustomEase`) for scroll-triggered reveals, the mobile nav/hamburger and
  desktop nav-indicator tweens, and route transitions, plus **Lenis** for
  the global smooth-scroll feel — driven off `gsap.ticker` rather than its
  own `requestAnimationFrame` loop, with `ScrollTrigger.update` wired to
  Lenis's `scroll` event and `gsap.ticker.lagSmoothing(0)` (Phase 12 — see
  `specs/2026-10-04-phase-12-visual-refresh/requirement.md`, which replaced
  the Motion/Framer Motion choice Phase 5 originally made). GSAP ships
  native TypeScript types, so no `any`
  is needed under the no-`any` rule above. All registration happens once at
  module scope in `apps/web/lib/gsap.ts`; usage is confined to small
  `"use client"` wrapper components
  (`apps/web/components/{motion-provider,reveal,page-transition,
  site-header,custom-cursor,site-image}.tsx`) imported into otherwise-
  server-component pages. `prefers-reduced-motion` is respected at three
  layers: each animating component's `gsap.matchMedia()` or
  `useReducedMotion()` (`apps/web/lib/use-reduced-motion.ts`) check, Lenis
  never initializing under reduced motion, and a blanket CSS safety net in
  `globals.css`.

_Why GSAP over Motion (superseding Phase 5's original choice):_ the
Phase 12 client ask (page transitions, scroll choreography, idle float,
a sitewide custom cursor, a desktop nav indicator) needed exactly the
imperative, DOM-ref-driven timeline/ScrollTrigger control Motion was
originally passed over for — see Phase 12's `requirement.md` Decision 2.
Rather than keep both libraries long-term, Motion was removed entirely.

_Why not plain CSS alone:_ CSS transitions/`@keyframes` still handle
simple hover/press/focus states fine, but can't do scroll-into-view
triggering, animate a disclosure panel's `height: auto` cleanly, or drive
per-frame cursor-following/idle-float tweens — GSAP is reserved for those
cases, keeping the CSS/JS split deliberate rather than all-or-nothing.

- **Site-decoration imagery**: real, AI-generated photos committed as
  static files under `apps/web/assets/images/` and brought in via a plain
  Next.js static `import` (not `public/` + a manual URL), so `next/image`
  gets automatic width/height and a built-in blur placeholder for free
  (Phase 12 addendum — see `specs/2026-10-04-phase-12-visual-refresh/
  requirement.md` Decision 7, which supersedes that same phase's original,
  shorter-lived `placehold.co`-based `PlaceholderImage` convention). This
  is for static-page decoration only — the DB-driven `ImageSlot` fallback
  on menu items/cart line items (used when a real `image_url` is absent)
  still uses `placehold.co`, a separate concern.

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
- **`tower-sessions`** with a Postgres-backed session store
  (`tower-sessions-sqlx-store`, `postgres` feature) for server-side
  sessions and secure, httpOnly session cookies.

This satisfies README §26 (email/password login, secure session, logout)
entirely within the Rust service — no Node-based auth library (e.g.
Auth.js/NextAuth) is introduced, since the backend of record is Rust.
Future items from README §26 (multiple staff accounts, roles, 2FA,
password reset) extend this same session model later; they are not MVP
scope.

**Implementation specifics pinned in `roadmap.md` Phase 11** (see
`specs/2026-10-03-phase-11-admin-auth/requirement.md` for full rationale):

- Session expiry is a **rolling 7-day inactivity window**
  (`tower_sessions::Expiry::OnInactivity`), not a fixed absolute expiry.
- The `tower-sessions-sqlx-store` session table is **self-migrated by the
  crate's own `PostgresStore::migrate()` call at API startup**, not a
  committed `apps/api/migrations/*.sql` file — a deliberate, scoped
  exception to this doc's "plain numbered `.sql` files" migration
  convention, since that table's schema is owned by the crate, not this
  project.
- Cookie `Secure` is toggled by a `COOKIE_SECURE` env var (default `false`
  in `docker-compose.yml`, default `true` in `docker-compose.prod.yml`) —
  the same per-compose-file-default convention already used for
  `LOG_FORMAT`. `SameSite=Lax` throughout; no cross-origin cookie
  configuration is needed since the browser only ever reaches the admin
  API through `apps/web`'s own same-origin Route Handler proxies, same
  posture as Phase 10's order-submission proxy.
- The first (and any later) admin account is created via a CLI subcommand,
  `apps/api/src/bin/create_admin.rs` — a second binary target in the `api`
  Cargo package, run manually over SSH (never through CI), not a seeded
  SQL file or an env-var auto-bootstrap on startup.

## Object/Image Storage

- **MinIO** (S3-compatible object storage), self-hosted as its own
  container alongside the app and database.

Satisfies the "object storage for images" element of README §40 while
staying inside the self-hosted-VPS decision. Because MinIO speaks the S3
API, a future migration to a managed S3-compatible provider (if the
business outgrows self-hosting) is a configuration change, not a rewrite.

**Wired into `apps/api` in Phase 14** (see
`specs/2026-10-04-phase-14-admin-menu-categories/requirement.md` for full
rationale) — the `minio` container ran unused since Phase 1 until then:

- **`aws-sdk-s3`** crate, configured by hand (no `aws-config`/IMDS lookup)
  with a static `Credentials` pair, `endpoint_url` pointed at
  `MINIO_ENDPOINT`, and `force_path_style(true)` (required for MinIO's
  path-style bucket addressing).
- **Server-side proxy pattern, both directions** — the browser never
  talks to MinIO directly. Upload: browser → same-origin Next.js Route
  Handler → protected `POST /api/admin/media` → MinIO. Serve: `<img>`/
  `next/image` → public, unauthenticated `GET /api/media/{key}` → MinIO.
  Chosen over a presigned direct-to-MinIO upload/download, which would
  need MinIO CORS configuration and a public-read bucket policy for one
  admin feature.
- The API authenticates to MinIO with the same `MINIO_ROOT_USER`/
  `MINIO_ROOT_PASSWORD` credentials already provisioned for the
  container — not a second, bucket-scoped MinIO user/policy (acceptable
  at single-admin, single-bucket scale; see that phase's open risk 1).
- Every upload gets a fresh, randomly generated object key
  (`{uuid}.{ext}`), never overwritten or deleted — this makes the public
  serve route's `Cache-Control: public, max-age=31536000, immutable`
  response header safe. Orphaned objects from replaced/archived images
  are never cleaned up (that phase's open risk 2).
- `apps/web/next.config.ts`'s `images.remotePatterns` carries a second
  entry (alongside `placehold.co`) for this public media host:
  `localhost:8080` in dev, `api.${DOMAIN}` in production — the latter
  needs `DOMAIN` passed into the `web` service's environment in
  `docker-compose.prod.yml`, which it didn't receive before Phase 14.
- **`react-easy-crop`** (`^6`), added mid-phase (open risk 4 resolution,
  2026-10-04) for the admin image-upload form's fixed-1:1-square
  pan/zoom/crop UI. Chosen over `react-image-crop` — it does the actual
  canvas cropping for you (not just the selection rectangle), has native
  touch/pinch support, and has no React 19 peer-dependency friction
  (`react`/`react-dom` peer range is `>=16.4.0`). The crop happens
  entirely client-side before upload; no server-side image processing
  crate was added to `apps/api` for this (see that phase's requirement.md
  Decision 14 addendum for why cropping stays off the Rust side).

## Email (P1 — order confirmations, contact/catering notifications)

- **`lettre`** crate, sending through a transactional email provider (e.g.
  Resend or Postmark) rather than a raw SMTP relay, for better
  deliverability.

This is P1 (README §33/§46 priority), not required for the MVP's core
ordering flow, but the dependency is named here so the `orders` and
`contact`/`catering` workflows can wire it in without a follow-up stack
decision.

## Spam/Abuse Mitigation (Phase 15)

Added mid-phase (open risk 3 resolution, 2026-10-04) for the two new
public, unauthenticated write endpoints (`POST /api/catering-enquiries`,
`POST /api/contact-messages`) — `POST /api/orders` (Phase 10) still has
neither, left as-is since it isn't this phase's concern.

- **`tower_governor`** (`^0.8`, `axum` feature), a `tower`/Axum rate-limit
  middleware, for a per-client-IP token bucket on both new endpoints —
  layered on only those two routes (`.route_layer`), not the whole
  `api_router()`.
- **A custom `KeyExtractor`**, not the crate's built-in peer-IP/smart-IP
  extractors — this app's browser traffic reaches `apps/api` exclusively
  through `apps/web`'s same-origin Route Handler proxy (server-to-server,
  over the Docker-internal network), so the TCP peer `apps/api` sees for
  that path is always the `web` container, not the visitor. The
  proxy forwards the real client IP (already present on its own
  incoming request's `X-Forwarded-For`, set by Caddy) through to `apps/api`
  unchanged; the custom extractor reads `X-Forwarded-For` and takes its
  **last** comma-separated entry (the nearest hop that actually touched
  the request — Caddy either way, whether a request reaches `apps/api`
  by this route or by hitting `api.${DOMAIN}` directly), not the first
  (which a client could freely spoof, since Caddy appends rather than
  replaces an incoming `X-Forwarded-For`). Missing header (local dev with
  no Caddy in front) falls back to one shared bucket rather than wiring up
  `axum::serve`'s `ConnectInfo` for a peer-IP fallback — acceptable since
  every real deployment of this app sits behind Caddy (same edge-proxy
  trust assumption the rest of the deployment already makes).
- **A honeypot field** (`website`, visually hidden off-screen +
  `aria-hidden` + `tabIndex={-1}` + `autoComplete="off"`, not
  `type="hidden"` or `display:none`) added to both new public forms. A
  filled value short-circuits the handler before validation or any
  database write, returning the same `201` success shape the real
  endpoint would (synthetic `id`/`created_at`) so a scripted submitter
  gets no signal that anything was rejected.
- Deliberately **not** CAPTCHA — no third-party dependency/user friction
  for a two-form, low-traffic site; revisit only if the business reports
  spam these two measures don't stop.

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
  provisioned in `roadmap.md` Phase 2; restore is verified in Phase 18
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
| Animation | GSAP + ScrollTrigger + CustomEase (scroll/transitions/cursor/float) + Lenis (smooth scroll) + plain CSS (hover/focus) |
| Site imagery | Static imports from `apps/web/assets/images/` (decoration); `placehold.co` (DB `image_url` fallback only) |
| i18n | next-intl (locale-prefixed `/en`, `/nl` routing) |
| Backend API | Rust + Axum + sqlx + serde |
| Database | PostgreSQL |
| Admin auth | argon2 + tower-sessions (Postgres-backed sessions) |
| Image/object storage | MinIO (S3-compatible, self-hosted) via `aws-sdk-s3`, server-side upload/serve proxy (Phase 14) |
| Image cropping | `react-easy-crop` — client-side fixed 1:1 crop before upload (Phase 14) |
| Email (P1) | lettre + transactional email provider |
| Spam/abuse mitigation | `tower_governor` (per-IP rate limit, custom `X-Forwarded-For` key extractor) + honeypot field (Phase 15) |
| WhatsApp | `wa.me` deep links |
| Hosting | AWS Lightsail VPS (`eu-central-1`, Frankfurt), Docker Compose |
| CI/CD | GitHub Actions → private GHCR → SSH deploy (dedicated `deploy` user) |
| Backups | Lightsail automatic daily snapshots |
| Reverse proxy / TLS | Caddy (automatic HTTPS) |
