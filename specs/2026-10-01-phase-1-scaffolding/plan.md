# Phase 1 — Scaffolding: Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed and
sanity-checked before moving to the next.

## Group 0 — Branch & root housekeeping

0.1. Create branch `2026-10-01-phase-1-scaffolding` off `master` (done — see
     `requirement.md` for why the old `phase-1-scaffolding` branch is left
     alone).

0.2. Add root `/.gitignore`:
```
# JS / web
apps/web/node_modules/
apps/web/.next/
apps/web/.pnpm-store/

# Rust / api
apps/api/target/

# env
.env
.env.*.local

# OS
.DS_Store
```

0.3. Add root `/.env.example` (committed, no secrets):
```
POSTGRES_USER=zainstreat
POSTGRES_PASSWORD=zainstreat_dev
POSTGRES_DB=zainstreat
POSTGRES_PORT=5432

MINIO_ROOT_USER=zainstreat
MINIO_ROOT_PASSWORD=zainstreat_dev
MINIO_PORT=9000
MINIO_CONSOLE_PORT=9001

API_PORT=8080
WEB_PORT=3000
```
Compose should also define inline fallback defaults
(`${POSTGRES_USER:-zainstreat}`) so `docker compose up` still works without
a local `.env` file.

## Group 1 — Next.js web scaffold (`apps/web`)

Depends on: Group 0 (`.gitignore` in place first).

1.1. `apps/web/package.json` — name `web`, `"packageManager": "pnpm@12.6.0"`,
     scripts: `dev` → `next dev -H 0.0.0.0 -p 3000` (bind to all interfaces
     so the Docker port mapping can reach it), `build`, `start`, `lint`.
     Dependencies: `next@^16`, `react@^19`, `react-dom@^19`,
     `@picocss/pico@^2`. Dev dependencies: `typescript@^5.9`, `@types/node`,
     `@types/react`, `@types/react-dom`, `eslint@^9`, `eslint-config-next@^16`.

1.2. `apps/web/tsconfig.json` — standard Next.js App Router config
     (`strict: true`, `jsx: preserve`, path alias `@/*`).

1.3. `apps/web/next.config.ts` — minimal empty `NextConfig` export.

1.4. `apps/web/app/globals.css` — `@import "@picocss/pico/css/pico.min.css";`
     only. No PostCSS config needed — Next.js handles the plain CSS import
     directly.

1.5. `apps/web/app/layout.tsx` — minimal `RootLayout`, imports
     `./globals.css`, sets `metadata.title = "Zain's Treat n More"`.

1.6. `apps/web/app/page.tsx` — the "hello" page: business name + a
     one-line placeholder message, wrapped in semantic `<main>`/`<article>`
     markup (Pico styles bare HTML elements, no utility classes needed) to
     prove the stylesheet loads. No brand styling yet.

1.7. `apps/web/eslint.config.mjs` — flat config extending
     `next/core-web-vitals` + `next/typescript`.

1.8. `apps/web/public/.gitkeep` (git doesn't track empty directories).

1.9. Run `pnpm install` inside `apps/web` once locally to generate and
     commit `apps/web/pnpm-lock.yaml`.

## Group 2 — Rust/Axum API scaffold (`apps/api`)

Depends on: Group 0. Independent of Group 1 — can be done in parallel.

2.1. `apps/api/Cargo.toml`:
```toml
[package]
name = "api"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

2.2. `apps/api/src/main.rs` — `#[tokio::main]`, an `axum::Router` with a
     single `GET /health` route returning `200` + `{"status":"ok"}`
     (`axum::Json`), binding to `0.0.0.0:$PORT` (env var, default `8080`),
     with a short startup log line.

2.3. `apps/api/Dockerfile.dev`:
```dockerfile
FROM rust:1-slim-bookworm
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
```
     (curl only — compiling happens via the bind-mounted source and a
     `command: cargo run`, not a build stage, so local edits are picked up
     without rebuilding the image.)

2.4. `apps/api/.dockerignore`:
```
target/
```

2.5. Run `cargo build` locally once to generate and commit
     `apps/api/Cargo.lock`.

No route module split yet — a single route doesn't justify it. Phase 2
will introduce structure once sqlx/DB wiring lands.

## Group 3 — `docker-compose.yml` + docs (root)

Depends on: Groups 1 and 2.

3.1. Root `docker-compose.yml`, four services:
   - **postgres** — `postgres:17-alpine`, env from `.env`
     (`POSTGRES_USER/PASSWORD/DB`), named volume
     `postgres_data:/var/lib/postgresql/data`, port
     `${POSTGRES_PORT:-5432}:5432`, healthcheck
     `pg_isready -U ${POSTGRES_USER} -d ${POSTGRES_DB}`.
   - **minio** — `minio/minio` (pin to a concrete release tag at
     implementation time), `command: server /data --console-address ":9001"`,
     env `MINIO_ROOT_USER/PASSWORD`, named volume `minio_data:/data`, ports
     `${MINIO_PORT:-9000}:9000` and `${MINIO_CONSOLE_PORT:-9001}:9001`,
     healthcheck `curl -f http://localhost:9000/minio/health/live`.
   - **api** — `build: { context: ./apps/api, dockerfile: Dockerfile.dev }`,
     `working_dir: /app`, bind mount `./apps/api:/app`, named volume
     `api_cargo_target:/app/target` (avoids recompiling from scratch every
     `up`), optional `cargo_registry_cache:/usr/local/cargo/registry`,
     `command: cargo run`, port `${API_PORT:-8080}:8080`,
     `depends_on: postgres (service_healthy), minio (service_healthy)`,
     healthcheck `curl -f http://localhost:8080/health`.
   - **web** — `image: node:22-bookworm-slim` (not alpine — see
     `requirement.md` risk note), bind mount `./apps/web:/app`, named
     volume `web_node_modules:/app/node_modules`, `command: sh -c "pnpm
     install && pnpm dev"`, port `${WEB_PORT:-3000}:3000`, **no**
     `depends_on` (decoupled from `api` per decision in `requirement.md`).
   - Top-level `volumes:` block: `postgres_data`, `minio_data`,
     `api_cargo_target`, `web_node_modules` (and optionally
     `cargo_registry_cache`).

3.2. `docs/local-development.md` — short "how to run this locally" doc:
     `cp .env.example .env`, `docker compose up`, the two URLs to check,
     `docker compose down`.

## Group 4 — Verification

See `validation.md` for the full pass/fail checklist. Summary:

4.1. `cp .env.example .env`.
4.2. `docker compose up`.
4.3. `docker compose ps` — all four services report `healthy`.
4.4. `curl -i http://localhost:8080/health` → `200` + `{"status":"ok"}`.
4.5. `curl -s http://localhost:3000` → hello page HTML, Pico CSS
     stylesheet link/content present.
4.6. `docker compose down`.
