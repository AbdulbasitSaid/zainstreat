# Phase 1 — Scaffolding: Requirements

## Source

`specs/roadmap.md`, Phase 1:

> - Repo layout for the Next.js app and the Rust/Axum API.
> - `docker-compose` for local dev: `web`, `api`, `postgres`, `minio`.
> - Basic health-check route on the API; basic "hello" page on the web app,
>   both reachable through the dev Compose stack.

## Context

This is the first implementation phase of the MVP (per `specs/mission.md` and
`specs/tech-stack.md`). Nothing has been built yet — the repo only has
`README.md` and the `specs/` docs tracked in git. A prior, abandoned attempt
left empty placeholder directories (`apps/web/{app,public}`,
`apps/api/src`) and stale, untracked `node_modules`/`target` build caches,
but no actual source files, `package.json`, or `Cargo.toml`. A branch named
`phase-1-scaffolding` also exists but has no commits (identical to `master`).

Per `roadmap.md`'s ordering principle, each phase must produce something
runnable and checkable on its own before the next phase starts. Phase 2
(data model) depends on Phase 1 providing a working API process and a
Postgres instance wired into the dev stack — so this phase's job is purely
structural: get both apps running, talking to nothing yet, inside Docker
Compose.

## Decisions

These were confirmed with the user or resolved during planning (see
conversation / plan sub-agent design):

1. **Branch:** new branch `2026-10-01-phase-1-scaffolding` off `master`. The
   stale `phase-1-scaffolding` branch is left untouched (not reused, not
   deleted).
2. **Scaffold target:** build directly into the existing empty `apps/web`
   and `apps/api` directories, hand-authoring files rather than running
   `create-next-app` (which can abort when a target directory already has
   entries, even empty ones).
3. **Package manager:** pnpm for the web app. No root-level pnpm workspace
   and no Cargo workspace — each app is a single, self-contained package.
   Revisit only if a future phase introduces a second JS package or Rust
   crate (none is currently on the roadmap).
4. **Styling:** Pico CSS (classless, via npm package import — no build-step
   config) is wired in now so the stylesheet is in place, but **no brand
   theming** (colors, fonts, logo) — that's Phase 3's job.
5. **Rust API dependencies for this phase only:** `axum`, `tokio`, `serde`,
   `serde_json`. Explicitly deferred to later phases: `sqlx` (Phase 2),
   `argon2` + `tower-sessions` (Phase 7), `lettre` (Phase 12).
6. **docker-compose services:** `web`, `api`, `postgres`, `minio`.
   - `web`, `postgres`, `minio` run from stock official images with bind
     mounts and a dev-server `command:` — no custom Dockerfile needed.
   - `api` gets a small `apps/api/Dockerfile.dev` (adds `curl` on top of
     `rust:1-slim-bookworm`) so its healthcheck can do a real
     `curl -f http://localhost:8080/health` instead of a bare TCP check.
7. **Web/API integration:** the hello page does **not** call the API's
   `/health` route in this phase. The two are verified independently. This
   avoids pulling in a CORS dependency (`tower-http`) before anything
   requires cross-service calls.
8. **Repo hygiene gaps to fix now:** no `.gitignore` exists anywhere in the
   repo — add a root one. No `.env.example` exists — add one documenting
   every env var the compose stack uses.

## Out of scope (explicitly deferred to later phases)

- Database schema/migrations, seed data (Phase 2).
- Any public page content beyond a placeholder hello page (Phase 3).
- Menu/cart/order logic (Phases 4–6).
- Admin auth, dashboard, menu/category management (Phases 7–9).
- Catering/contact forms, WhatsApp links (Phases 10–11).
- SEO/accessibility polish, email sending (Phase 12).
- Production Dockerfiles, Caddy reverse proxy, backups (Phase 13).
- Any coupling between `web` and `api` (e.g. the hello page fetching live
  API data).

## Open risk flagged during planning

`node:22-alpine` was considered and rejected for the `web` service in favor
of `node:22-bookworm-slim`, because Next.js's SWC compiler ships native
bindings with known musl/Alpine compatibility issues, and we're deliberately
not adding a custom Dockerfile to work around that for `web`.
