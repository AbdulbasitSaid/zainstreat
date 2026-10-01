# Phase 2 — AWS Deployment Infrastructure & CI/CD: Requirements

## Source

`specs/roadmap.md`, Phase 2:

> - Provision an AWS Lightsail instance: Ubuntu LTS blueprint, 2 GB plan,
>   `eu-central-1` (Frankfurt — closest AWS region to the Netherlands).
>   Attach a static IP, configure the Lightsail firewall (22/80/443 only),
>   install Docker + the Compose plugin.
> - Create a dedicated, sudo-less `deploy` system user on the box (`docker`
>   group only) with its own GitHub Actions-only SSH keypair — kept
>   separate from the admin key used for manual/break-glass access.
> - Author basic production multi-stage Dockerfiles: `apps/web/Dockerfile`
>   (Next.js standalone build, `node:22-bookworm-slim` runtime) and
>   `apps/api/Dockerfile` (`cargo build --release`, copied onto
>   `debian:bookworm-slim`). Deliberately basic — not yet hardened
>   (no non-root user, no distroless base); that polish is Phase 14's job.
> - `.github/workflows/deploy.yml`: on every push to `master`, build the
>   `web` and `api` images, push them to a **private** GHCR registry, then
>   SSH to the Lightsail box as the `deploy` user and run `docker compose
>   -f docker-compose.yml -f docker-compose.prod.yml pull && ... up -d`.
>   True continuous deployment — no manual approval step. No test suite
>   exists yet, so nothing gates the deploy on tests passing.
> - Point the business domain's DNS A record(s) at the static IP.
> - `docker-compose.prod.yml` override: `web`/`api` reference prebuilt
>   `ghcr.io/<owner>/zainstreat-web:latest` / `...-api:latest` images
>   (no `build:` context, no bind mounts — the server only pulls and runs);
>   adds a `caddy` service (reverse proxy, automatic HTTPS for the domain);
>   `postgres`, `minio`, `web`, and `api` stop publishing ports to the
>   public host — only `caddy` is internet-facing, everything else talks
>   over the internal Docker network. The server authenticates to GHCR with
>   a scoped read-only PAT configured once by hand (never committed).
> - Enable Lightsail's automatic daily snapshot add-on — this resolves
>   `tech-stack.md`'s previously-open "backup mechanism" item for now (see
>   Phase 14 for verifying an actual restore once real data exists).
> - `docs/deployment.md`: how the CI/CD flow works (push to `master`, watch
>   Actions, live), plus a manual/break-glass rollback runbook.

## Context

Phase 1 (scaffolding) is merged to `master`: `apps/web` (Next.js hello
page) and `apps/api` (Rust/Axum `/health` route) run locally via
`docker-compose.yml`, alongside `postgres` and `minio`, using bind mounts
and dev-server commands (`pnpm dev`, `cargo run`). Nothing has been
deployed anywhere outside the developer's machine.

The original roadmap put deployment last (what is now Phase 14), after the
full MVP feature set. That plan is changed: deployment moves to right
after scaffolding, so the business gets a real, public, HTTPS-reachable
environment immediately, and every later phase (data model through
polish) ships to that same live box incrementally instead of one
untested, big-bang deploy at the end.

This phase was initially scoped as infrastructure-only with a manual SSH
deploy ("CI/CD explicitly out of scope"). That was revised during
planning: deploys now go through a GitHub Actions CI/CD pipeline instead
of a human typing commands over SSH. That choice has a direct consequence
— pushing prebuilt images to GHCR requires real Dockerfiles, not the
Phase 1 dev setup — so **basic production Dockerfiles for `web`/`api` are
now part of this phase**, pulled forward from what was originally Phase
14's job. Phase 14 still exists; it now hardens these Dockerfiles
(non-root user, smaller base images) rather than introducing them from
scratch.

The goal remains proving the path end-to-end — a real domain, over HTTPS,
reaching `web`/`api` containers on a cheap, backed-up AWS box that doesn't
expose anything it shouldn't — but delivery is now automatic: merging to
`master` ships to production without anyone touching the server by hand.

## Decisions

These were confirmed with the user during planning:

1. **Provider/product:** AWS **Lightsail**, not raw EC2. Lightsail's flat
   monthly price bundles a static IP, a data-transfer allowance, and
   snapshot backups, avoiding separate Elastic IP/EBS/security-group
   bookkeeping for a single-box deployment.
2. **Region:** `eu-central-1` (Frankfurt). AWS has no region physically in
   the Netherlands; Frankfurt is the closest full region and the standard
   low-latency choice for NL-based traffic.
3. **Instance size:** the 2 GB RAM Lightsail plan — comfortable headroom
   for Postgres + MinIO + the `web`/`api` containers + Caddy running
   concurrently (the box never builds images itself, see decision 9).
4. **Blueprint:** plain Ubuntu LTS (not a Lightsail "app" blueprint),
   since Docker + Compose are installed directly.
5. **Domain:** the business already has a domain ready to point at the
   instance, so Caddy's automatic HTTPS is wired up now rather than
   deferred to plain-HTTP/IP access.
6. **Backups:** Lightsail's automatic daily instance snapshot add-on is
   the backup mechanism — resolves `tech-stack.md`'s previously-open
   "mechanism to be defined" line. No custom `pg_dump`/`mc mirror`
   scripts in this phase.
7. **Deploy mechanism: GitHub Actions CI/CD**, triggered on every push to
   `master` (true continuous deployment, no manual approval gate). One
   workflow: build `web`/`api` images → push to GHCR → SSH deploy.
8. **Deploy auth:** a dedicated, sudo-less `deploy` system user on the
   Lightsail box (`docker` group only), authorized only for a GitHub
   Actions-specific SSH keypair. The existing admin SSH key is left
   untouched for manual/break-glass access and never given to Actions.
9. **Build location:** images are built on GitHub's hosted runners and
   pushed to **GHCR**, kept **private** (proprietary business images).
   The Lightsail box never compiles anything — it authenticates to GHCR
   with a scoped read-only PAT (configured once by hand, `read:packages`
   only) and just pulls + runs.
10. **Production Dockerfiles land now, basic only:** `apps/web/Dockerfile`
    (Next.js standalone output, `node:22-bookworm-slim` runtime) and
    `apps/api/Dockerfile` (`cargo build --release` onto
    `debian:bookworm-slim`). Correct and reasonably small, but not yet
    hardened — no non-root user, no distroless base. That hardening is
    Phase 14's job.
11. **No CI test gating:** no test suite exists yet (Phase 1 shipped
    none), so the workflow is build → push → deploy, nothing more.

## Out of scope (explicitly deferred)

- Hardening the production Dockerfiles (non-root user, distroless/smaller
  base images, resource limits) — Phase 14.
- Verifying an actual snapshot restore (Phase 14, once real data exists).
- Any CI test gating — there's nothing to test yet; add this once a
  later phase introduces tests.
- Any new application feature (data model, menu, cart, orders, admin,
  catering/contact, WhatsApp, polish) — Phases 3–13.
- Route 53 / migrating DNS registrars — the domain's existing DNS
  provider is used as-is; only an A record is added/changed there.

## Secrets hygiene

The server-side `.env` is created by hand over SSH from `.env.example`
and is never committed. This repo's history already includes a prior
incident of committed secrets (see commits `23b665e`/`e874e98`, "removed
and changed secretes") — the same mistake must not repeat on:
- the server's `.env`,
- the GHCR read-only PAT configured on the server,
- the `deploy` user's GitHub Actions SSH private key (`DEPLOY_SSH_KEY`
  GitHub secret) — this one lives only in GitHub's encrypted secrets
  store and the server's `authorized_keys`, never in the repo.
