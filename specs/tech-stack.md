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
as containers on one self-hosted VPS, fronted by a reverse proxy.

## Frontend

- **Next.js (App Router), React, TypeScript** — server-rendered pages for
  SEO (README §33) on public routes, with client components for the cart
  and admin interactivity.
- **Pico CSS** — a classless/semantic-HTML CSS framework that matches the
  brand's visual direction (rounded cards, soft corners, subtle borders —
  README §34) out of the box, with minimal markup overhead. Brand color
  tokens (README §3) are wired up via Pico's CSS custom properties
  (`--pico-primary`, etc.) rather than a utility-class theme.
- **`next/image`** for image optimization, lazy loading, and modern formats
  (README §33 performance requirements).

_Why not Astro for the marketing pages:_ a single Next.js app is simpler to
build, deploy, and maintain than two separate frontends, and Next.js still
meets the SEO/SSR requirements in README §33.

## Backend API

- **Rust + Axum** — the HTTP framework for the API service.
- **`sqlx`** — compile-time-checked SQL queries against Postgres (no ORM
  magic, matches the straightforward relational data model in README §31).
- **`serde`** — JSON (de)serialization for API request/response bodies.

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

- **Self-hosted VPS**, all services run via **Docker Compose**:
  - `web` — Next.js app
  - `api` — Rust/Axum API
  - `postgres` — database
  - `minio` — object storage
  - `caddy` — reverse proxy, automatic HTTPS via Let's Encrypt
- Postgres and MinIO data are backed up on a regular schedule from the VPS
  (mechanism to be defined during the deployment phase — see
  `roadmap.md`).

_Why self-hosted over a managed PaaS (e.g. Vercel + managed Postgres):_
decided explicitly — gives full control over the Rust API runtime and
keeps all infrastructure (compute, DB, storage) on one bill and one box,
which fits a small single-business deployment.

## Summary Table

| Concern | Choice |
|---|---|
| Frontend | Next.js (App Router) + React + TypeScript + Pico CSS |
| Backend API | Rust + Axum + sqlx + serde |
| Database | PostgreSQL |
| Admin auth | argon2 + tower-sessions (Postgres-backed sessions) |
| Image/object storage | MinIO (S3-compatible, self-hosted) |
| Email (P1) | lettre + transactional email provider |
| WhatsApp | `wa.me` deep links |
| Hosting | Self-hosted VPS, Docker Compose |
| Reverse proxy / TLS | Caddy (automatic HTTPS) |
