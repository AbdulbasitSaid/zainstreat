# Deployment

Production runs on a single AWS Lightsail instance (Amazon Linux 2023,
2 GB RAM, `eu-central-1`) reachable at `zainstreat.com` /
`api.zainstreat.com`.
Images are built on GitHub's runners, pushed to a private GHCR registry,
and pulled by the server — the server never builds anything itself.

## Day-to-day: how a deploy happens

1. Merge (or push directly) to `master`.
2. `.github/workflows/deploy.yml` runs automatically:
   - **build-and-push**: builds `apps/web/Dockerfile` and
     `apps/api/Dockerfile`, tags each `:latest` and `:<commit-sha>`,
     pushes both to `ghcr.io/abdulbasitsaid/zainstreat-web` /
     `zainstreat-api` (private packages).
   - **deploy**: SSHes into the server as the `deploy` user and runs
     `docker compose -f docker-compose.yml -f docker-compose.prod.yml
     pull && ... up -d` in `/opt/zainstreat`.
3. Watch progress under the repo's **Actions** tab. No manual approval
   step — this is true continuous deployment, and there's no test suite
   yet to gate it on.

That's the whole normal path. Nothing below this point happens on every
push — it's one-time setup and the rollback procedure.

## One-time manual setup (not automated)

The workflow above only works once the following exists. None of it is
created by code in this repo — see `specs/2026-10-01-phase-2-aws-deployment/
plan.md` for full step-by-step detail; summarized here:

- **Lightsail instance** (`plan.md` Group 0): Amazon Linux 2023, 2 GB
  plan, `eu-central-1`, a static IP attached, firewall restricted to
  `22`/`80`/`443`, automatic daily snapshot add-on enabled.
- **Server bootstrap** (`plan.md` Group 1): Docker Engine + Compose
  plugin installed (Docker's official steps, not `docker.io`), this repo
  checked out read-only at `/opt/zainstreat`, and a real `.env` created
  by hand at `/opt/zainstreat/.env` from `.env.example` (including a real
  `DOMAIN=zainstreat.com`). **Never commit this file.**
- **`deploy` system user** (`plan.md` Group 1b): sudo-less, `docker`
  group only, its own SSH keypair (distinct from the admin key). The
  private half becomes the `DEPLOY_SSH_KEY` GitHub Actions secret; the
  server's IP/domain and `deploy` become `DEPLOY_HOST` / `DEPLOY_USER`.
- **GHCR pull auth on the server** (`plan.md` §2.4): as the `deploy`
  user, `docker login ghcr.io` once with a PAT scoped to `read:packages`
  only. Never committed.
- **DNS** (`plan.md` Group 3): `@` and `api` A records at the domain's
  existing DNS provider pointed at the Lightsail static IP.

Until all of the above exists, the `deploy` job in the workflow will fail
at the SSH step (or the secrets simply won't be set) — that's expected
for a repo that hasn't had its server bootstrapped yet.

## Manual / break-glass rollback

Use this if a bad deploy needs to be reverted outside the normal
push-to-`master` flow. Requires the **admin** SSH key (not the `deploy`
user's).

1. SSH into the server as admin.
2. Find the previous good commit SHA (GitHub Actions run history, or
   `git log` on `/opt/zainstreat`).
3. Pull and run that specific tag instead of `latest`, e.g.:
   ```bash
   cd /opt/zainstreat
   docker compose -f docker-compose.yml -f docker-compose.prod.yml \
     pull  # or manually: docker pull ghcr.io/abdulbasitsaid/zainstreat-web:<previous-sha>
   # then re-point the running containers at that tag and:
   docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
   ```
   The simplest version in practice: re-run a prior **green** Actions run
   from the GitHub Actions UI ("Re-run jobs") — it rebuilds and redeploys
   that exact commit, including re-tagging it `:latest`.
4. Confirm with `docker compose -f docker-compose.yml -f
   docker-compose.prod.yml ps` and `curl -i https://api.zainstreat.com/
   health`.

## Seeding the production database

`apps/api/seed.sql` holds the real client menu (categories, menu items,
price options — see Phase 7). Since no admin CMS exists yet (Phase 13) to
edit the live menu any other way, this file is also used to (re)populate
production. Running it **replaces** all categories/menu items/price
options (it opens with `TRUNCATE ... CASCADE`), so only run it when that's
actually intended.

Do this via `.github/workflows/seed-production.yml`, not by hand:

1. GitHub → **Actions** tab → **Seed Production Database** → **Run
   workflow**.
2. In the `confirm` input, type exactly `SEED PRODUCTION`. Any other value
   (or leaving it blank) fails the job immediately before it touches the
   server.
3. The job, over the same `deploy`-user SSH credentials `deploy.yml` uses:
   - `pg_dump`s the current database to
     `/opt/zainstreat/backups/backup-<timestamp>.sql` on the server (fails
     the job if the dump comes back empty), covered thereafter by
     Lightsail's daily snapshot like the rest of the box's data.
   - Pipes `apps/api/seed.sql` into `psql -v ON_ERROR_STOP=1` against the
     `postgres` container.
   - Prints `categories`/`menu_items` row counts to the run log to
     eyeball against `seed.sql`'s known contents.
4. Confirm with `curl -i https://api.zainstreat.com/health` and by loading
   the live menu page in a browser.

To restore a prior backup by hand if a seed went wrong: SSH in as admin,
`cat /opt/zainstreat/backups/backup-<timestamp>.sql | docker compose -f
docker-compose.yml -f docker-compose.prod.yml exec -T postgres psql -U
$POSTGRES_USER -d $POSTGRES_DB`.

## Viewing production logs

Two ways to check what the running stack is doing, from quickest to most
convenient:

1. **Direct SSH** (no setup required): `docker compose -f
   docker-compose.yml -f docker-compose.prod.yml logs -f api` (swap
   `api` for `web`/`caddy`/`postgres`/`minio`/`dozzle`; add `--tail=200`
   for recent history without following). `api` and `caddy` both emit
   structured JSON log lines (request method/path/status/latency for
   `api`, access logs for `caddy`) as of Phase 8.
2. **Dozzle** (browser-based log viewer): `https://logs.<domain>`,
   gated by HTTP Basic Auth (`DOZZLE_BASIC_AUTH_USER` /
   `DOZZLE_BASIC_AUTH_HASH` in the server's `.env` — the hash is
   generated once via `docker run --rm caddy:2-alpine caddy
   hash-password`, never committed). It lists and tails **every**
   container's logs, including `postgres` and `minio`, which is exactly
   why it's access-gated rather than exposed on its own port — the
   Lightsail firewall only opens `22`/`80`/`443` anyway, so it's only
   reachable through Caddy.

`RUST_LOG` and `LOG_FORMAT` (`apps/api`'s log level/format) can be tuned
live by editing the server's `.env` and running `docker compose -f
docker-compose.yml -f docker-compose.prod.yml restart api` — no image
rebuild or redeploy needed.

## Backups

Lightsail's automatic daily instance snapshot add-on is enabled on the
instance (Lightsail console → instance → **Snapshots**). This is the
current backup mechanism for the whole box, including the `postgres` and
`minio` data volumes. Restoring from a snapshot has **not** been tested
yet — Phase 17 verifies an actual restore once real data exists.
