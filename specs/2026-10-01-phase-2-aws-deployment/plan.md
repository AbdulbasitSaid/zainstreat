# Phase 2 — AWS Deployment Infrastructure & CI/CD: Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed and
sanity-checked before moving to the next.

## Group 0 — Lightsail instance

0.1. Create a Lightsail instance: Amazon Linux 2023 blueprint, 2 GB
     RAM / 1 vCPU / 60 GB SSD plan, region `eu-central-1` (Frankfurt,
     AZ `a`).

0.2. Create (or import) an admin SSH key pair in Lightsail; disable
     password authentication on the instance (key-only). This admin key
     is for manual/break-glass access only — it is never given to GitHub
     Actions (see Group 1b).

0.3. Allocate and attach a Lightsail **static IP** to the instance.

0.4. Configure the Lightsail firewall: allow `22` (SSH), `80` (HTTP),
     `443` (HTTPS) only. Remove any other default rules.

0.5. Enable the Lightsail **automatic snapshot** add-on on the instance
     (daily).

## Group 1 — Server setup

Depends on: Group 0.

1.1. SSH in as admin (`ec2-user`). Update packages (`sudo dnf
     update -y`).

1.2. Install Docker Engine + the Compose plugin per Docker's official
     Amazon Linux / CentOS install steps (not the distro-packaged
     `docker`, to get a current Compose v2), and enable/start the
     `docker` service.

1.3. `git clone` this repository onto the instance (read-only deploy
     checkout, e.g. `/opt/zainstreat`) — only `docker-compose.yml`,
     `docker-compose.prod.yml`, `Caddyfile`, and `.env` are actually used
     on the server; app source lives in the prebuilt images, not here.

1.4. Create `/opt/zainstreat/.env` by hand from `.env.example`, filled
     with real (non-dev) `POSTGRES_PASSWORD` / `MINIO_ROOT_PASSWORD` /
     `DOMAIN` values generated for this server. **Never commit this
     file** — see `requirement.md`'s secrets-hygiene note on this repo's
     prior committed-secrets incident.

## Group 1a — Dockerfiles

Depends on: nothing (can be done in parallel with Group 0/1, authored
from a dev machine and committed like any other code change).

1a.1. `apps/web/Dockerfile` — multi-stage:
   - `deps` stage: `node:22-bookworm-slim`, installs dependencies via
     pnpm (`corepack enable`, `pnpm install --frozen-lockfile`).
   - `build` stage: copies source, runs `pnpm build`. `next.config.ts`
     needs `output: "standalone"` added so the build emits a minimal
     self-contained server.
   - Runtime stage: `node:22-bookworm-slim`, copies only
     `.next/standalone`, `.next/static`, and `public` from the build
     stage, runs `node server.js`, binds `0.0.0.0:3000`.

1a.2. `apps/api/Dockerfile` — multi-stage:
   - `build` stage: `rust:1-slim-bookworm`, `cargo build --release`.
   - Runtime stage: `debian:bookworm-slim` (+ `ca-certificates` for
     outbound TLS, needed later for `lettre`/transactional email), copies
     just the compiled `api` binary from the build stage, runs it,
     binds `0.0.0.0:8080` via the existing `PORT` env var.

1a.3. Build both locally (`docker build -t zainstreat-web apps/web` /
     `-t zainstreat-api apps/api`) and run them standalone (not via
     Compose) to confirm each starts and serves its existing Phase 1
     routes, before wiring up CI.

Deliberately basic — no non-root `USER`, no distroless base. That
hardening is Phase 14's job (see `requirement.md`).

## Group 1b — Deploy user

Depends on: Group 1 (Docker must be installed first, since this user
needs the `docker` group to exist).

1b.1. On the server, create a system user `deploy`: no `sudo`, shell
     disabled or restricted, added to the `docker` group only.

1b.2. Generate a new SSH keypair specifically for this user (not reused
     from the admin key in Group 0.2). Authorize only the public half in
     `deploy`'s `authorized_keys`.

1b.3. Store the private half as a GitHub Actions repository secret
     `DEPLOY_SSH_KEY`. Add two more repo secrets: `DEPLOY_HOST` (the
     static IP or domain) and `DEPLOY_USER` (`deploy`).

## Group 2 — Production compose override + Caddy

Depends on: Groups 1, 1a (image names referenced below must match what
Group 2a's workflow pushes).

2.1. `docker-compose.prod.yml` (override, applied with `-f
     docker-compose.yml -f docker-compose.prod.yml`):
   - `web` and `api` use `image: ghcr.io/<owner>/zainstreat-web:latest`
     / `ghcr.io/<owner>/zainstreat-api:latest` — **no** `build:` context
     and **no** bind mounts (the server only pulls and runs; see
     `requirement.md` decision 9).
   - Removes the `ports:` mapping from `postgres`, `minio`, `web`, and
     `api` entirely (none published to the host — reachable only on the
     internal Compose network; only `caddy` is public).
   - Adds a `caddy` service: official `caddy:2-alpine` image, bind-mounts
     `./Caddyfile:/etc/caddy/Caddyfile`, named volumes
     `caddy_data:/data` and `caddy_config:/config` (so Let's Encrypt
     certs persist across restarts), ports `80:80` and `443:443`.
   - Top-level `volumes:` additions: `caddy_data`, `caddy_config`.

2.2. Root `Caddyfile`:
```caddyfile
{$DOMAIN} {
	reverse_proxy web:3000
}

api.{$DOMAIN} {
	reverse_proxy api:8080
}
```
     `DOMAIN` is read from `.env`. Caddy requests/renews Let's Encrypt
     certificates for both hostnames automatically — no manual certbot
     steps.

2.3. Add `DOMAIN=` to `.env.example` (placeholder value) and to the
     server's real `.env` (Group 1.4).

2.4. On the server, `docker login ghcr.io -u <github-username> -p
     <read-only PAT>` once, as the `deploy` user, using a PAT scoped to
     `read:packages` only. This is configured by hand and never committed
     (same hygiene as `.env`) — it persists in the `deploy` user's Docker
     config so subsequent `pull`s are non-interactive.

## Group 2a — GitHub Actions workflow

Depends on: Groups 1a (Dockerfiles must exist to build), 1b (secrets must
exist), 2.4 (server must be able to pull before the workflow's deploy step
can succeed).

2a.1. `.github/workflows/deploy.yml`, triggered on `push: branches:
     [master]`:
   - **build-and-push** job: checks out the repo, logs in to GHCR using
     the workflow's built-in `GITHUB_TOKEN` (`packages: write`
     permission — no extra secret needed to *push*), builds
     `apps/web/Dockerfile` and `apps/api/Dockerfile` with Buildx, tags
     each `ghcr.io/<owner>/zainstreat-web:latest` /
     `:${{ github.sha }}` (and the `api` equivalent), pushes both tags.
   - **deploy** job (needs: build-and-push): SSHes to `${{
     secrets.DEPLOY_HOST }}` as `${{ secrets.DEPLOY_USER }}` using `${{
     secrets.DEPLOY_SSH_KEY }}` (e.g. via `appleboy/ssh-action`), and
     runs `cd /opt/zainstreat && docker compose -f docker-compose.yml -f
     docker-compose.prod.yml pull && docker compose -f
     docker-compose.yml -f docker-compose.prod.yml up -d`.
   - No test/lint job gates this — none exists yet (see `requirement.md`
     decision 11).

2a.2. Confirm the GHCR packages created by the first workflow run are
     **private** (GitHub defaults new packages from a private repo to
     private, but verify explicitly in the package settings).

## Group 3 — DNS

Depends on: Group 0.3 (static IP must exist first).

3.1. In the domain's existing DNS provider, add/update A records:
   - `@` (apex) → the Lightsail static IP.
   - `api` → the Lightsail static IP.
   (No registrar migration to Route 53 — out of scope per
   `requirement.md`.)

3.2. Wait for propagation; confirm with `dig`/`nslookup` from a machine
     off the office network before attempting HTTPS.

## Group 4 — First deploy

Depends on: Groups 2, 2a, 3.

4.1. Merge this phase's branch to `master`. This push triggers
     `.github/workflows/deploy.yml` — watch the Actions run build, push
     to GHCR, and deploy, rather than running any command by hand on the
     server.

4.2. Confirm Caddy obtains valid certificates for both hostnames (check
     `docker compose logs caddy` on the server for ACME success, no
     errors).

4.3. Re-confirm the Lightsail automatic snapshot add-on (Group 0.5) is
     still enabled and has produced at least one snapshot after this
     first deploy.

## Group 5 — Docs

Depends on: Group 4 (steps must be proven to work before being written down).

5.1. `docs/deployment.md`:
   - How deploys happen now: push/merge to `master` → GitHub Actions
     builds, pushes to GHCR, deploys automatically. No manual step in
     the normal path.
   - Prerequisites for the pipeline to work (secrets already configured,
     server already bootstrapped per Groups 0–2).
   - **Manual/break-glass rollback runbook**: SSH in as admin,
     `docker compose -f docker-compose.yml -f docker-compose.prod.yml
     pull` a previous image tag (`ghcr.io/.../web:<previous-sha>`) and
     `up -d`, or re-run a prior successful Actions run from the GitHub
     UI.
   - Where backups live (Lightsail console → Snapshots) and that restores
     are untested until Phase 14.

## Group 6 — Verification

See `validation.md` for the full pass/fail checklist.
