# Phase 18 — Production Hardening & Final Rollout: Implementation Plan

Two kinds of work are mixed in this phase, and the groups are ordered
around that:

- **Ops groups (1, 2, 3, 8, 10, 11)** are manual steps in the Lightsail
  console, over SSH, or in a third-party dashboard. Nothing in the repo
  changes. Groups 1–3 come **first** because the site is down.
- **Repo groups (4–7, 9)** change files in the repo. Groups 4–7 must merge
  and deploy **together, in one push to `master`** (requirement.md Risk 4:
  the non-root `web` user and the `web_cache` volume must arrive in the
  same deploy).

Run `/validate` after Groups 4–7 are implemented locally, before merging.

## Group 0 — Branch

Branched `2026-10-11-phase-18-production-hardening` off `master`
(Phase 17 merged as `a4791bf`).

## Group 1 — Recover and capture evidence (ops, now)

Depends on: nothing. Do this before anything else.

1.1. Lightsail console → instance → **Metrics**. Screenshot the last
     24 h / 1 week of **CPU utilization** *and* **Burst capacity
     (percentage and minutes)**. If burst capacity hit 0%, CPU credit
     exhaustion played a part. Note it for requirement.md's "Confirmed
     cause" addendum.

1.2. Lightsail console → instance → **Reboot**. If it hasn't come back in
     ~5 minutes, **Stop** then **Start** (keeps the static IP and the
     disk).

1.3. As soon as SSH works, as the admin user:

```bash
uptime; free -m; swapon --show; df -h /
# Previous boot's kernel log: OOM-killer activity, hung tasks
sudo journalctl -k -b -1 --no-pager | grep -iE "out of memory|oom|killed process|hung_task|blocked for more than" | tail -40
# If -b -1 errors with "no persistent journal", note that (fixed in Group 3)
sudo journalctl --list-boots | tail -5
cd /opt/zainstreat
C="docker compose -f docker-compose.yml -f docker-compose.prod.yml"
$C ps -a
docker stats --no-stream
docker system df
# Which URLs were being hit before the hang (caddy keeps logs across restarts via json-file)
$C logs --since 6h caddy | grep -o '"uri":"[^"]*"' | sort | uniq -c | sort -rn | head -25
$C logs --since 6h caddy | grep -o '"User-Agent":\["[^"]*"' | sort | uniq -c | sort -rn | head -15
$C logs --since 6h web | tail -100
```

1.4. Paste the output into the conversation, or save it under the scratch
     notes. Write a short **"Confirmed cause"** addendum at the end of
     `requirement.md` (memory / disk / burst credits / crawler / other).
     If it's not memory, re-check Groups 3–6 against what the evidence
     shows (requirement.md Risk 1) before going on.

## Group 2 — Upgrade to the 4 GB plan (ops)

Depends on: Group 1 (evidence is captured from the old box first).
This is also the snapshot-restore test (requirement.md Decision 2).

2.1. **Stop** the old 2 GB instance (requirement.md Risk 2: no writes
     between snapshot and cutover).

2.2. Instance → **Snapshots** → **Create snapshot**, named
     `pre-4gb-upgrade-2026-10-11`. Wait until it shows *Available*.

2.3. From that snapshot → **Create new instance**: same region
     (`eu-central-1`) and AZ, plan **4 GB RAM**. Name it e.g.
     `zainstreat-prod-4gb`.

2.4. On the new instance:
     - **Networking** → firewall: make sure it's exactly `22`, `80`,
       `443` (IPv4 *and* IPv6 tabs). Rules aren't guaranteed to carry
       over from the snapshot, so re-add `443` if it's missing.
     - **Snapshots** → turn **Automatic snapshots** back on (the add-on
       is per instance; it doesn't carry over).

2.5. **Networking** → static IP → **Detach** from the old instance →
     **Attach** to the new one. DNS (`@`, `www`, `api`, `logs` A records)
     points at the static IP, so nothing changes at the DNS provider.
     `DEPLOY_HOST` in GitHub Actions secrets is unchanged for the same
     reason.

2.6. SSH in (same host key, same users; it's the same disk) and:

```bash
free -m          # total ≈ 3.8 GiB
cd /opt/zainstreat
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
docker compose -f docker-compose.yml -f docker-compose.prod.yml ps
curl -sI https://zainstreat.com | head -1
curl -s https://api.zainstreat.com/health
```

2.7. Spot-check that real data survived the restore: the live `/en/menu`
     shows the real menu items with photos (MinIO data), and `/admin`
     login works and the order list shows the orders placed before
     today (Postgres data).

2.8. Leave the old 2 GB instance **stopped, not deleted**. Rollback is
     moving the static IP back (2.5 in reverse) and starting it. Delete
     it only in Group 11.

## Group 3 — Host hardening: swap + persistent journal (ops)

Depends on: Group 2 (do it on the new instance).

3.1. 2 GB swap file, low swappiness (requirement.md Decision 3):

```bash
sudo dd if=/dev/zero of=/swapfile bs=1M count=2048 status=progress
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
echo 'vm.swappiness=10' | sudo tee /etc/sysctl.d/99-swappiness.conf
sudo sysctl --system
swapon --show; free -m
```

3.2. Persistent journal, size-capped (requirement.md Decision 7):

```bash
sudo mkdir -p /etc/systemd/journald.conf.d /var/log/journal
printf '[Journal]\nStorage=persistent\nSystemMaxUse=200M\n' | sudo tee /etc/systemd/journald.conf.d/10-persistent.conf
sudo systemd-tmpfiles --create --prefix /var/log/journal
sudo systemctl restart systemd-journald
sudo journalctl --list-boots   # after the next reboot, -b -1 must work
```

3.3. Reboot once (`sudo reboot`) to prove swap comes back from `fstab`
     and `journalctl -b -1` now returns the previous boot.

## Group 4 — Dockerfile hardening (repo)

Depends on: nothing in the repo. Ships with Groups 5–7.

4a. **`apps/api/Dockerfile`**: runtime stage runs as a non-root `app`
    user. Keep `debian:bookworm-slim` + `curl` (requirement.md
    Decision 8; the base compose healthcheck uses `curl`):

```dockerfile
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system app && useradd --system --gid app --no-create-home app
COPY --from=build /app/target/release/api /usr/local/bin/api
COPY --from=build /app/target/release/create_admin /usr/local/bin/create_admin
USER app
EXPOSE 8080
CMD ["api"]
```

    (`create_admin` is run via `docker compose exec api create_admin`;
    it only talks to Postgres over the network, so a non-root user is
    fine. Check this in validation.)

4b. **`apps/web/Dockerfile`**: runtime stage runs as the image's
    built-in `node` user, and pre-creates `.next/cache` owned by `node`
    so the Group 5 volume starts out writable (requirement.md Risk 4).
    When Docker creates a new named volume over an existing image
    directory, it copies that directory's ownership:

```dockerfile
FROM node:22-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
COPY --from=build --chown=node:node /app/.next/standalone ./
COPY --from=build --chown=node:node /app/.next/static ./.next/static
COPY --from=build --chown=node:node /app/public ./public
RUN mkdir -p .next/cache && chown -R node:node .next/cache
USER node
EXPOSE 3000
CMD ["node", "server.js"]
```

## Group 5 — Container limits + persistent image cache (repo)

Depends on: Group 4 (same deploy). File: `docker-compose.prod.yml` only.
Dev's `docker-compose.yml` stays unlimited.

5a. Add `mem_limit` / `memswap_limit` (equal, so swap isn't used) /
    `cpus` per requirement.md Decision 4's table. Example for `web`,
    which also gets the cache volume (Decision 6):

```yaml
  web:
    image: ghcr.io/abdulbasitsaid/zainstreat-web:latest
    working_dir: !reset null
    volumes: !override
      - web_cache:/app/.next/cache
    command: !reset null
    ports: !reset []
    environment:
      DOMAIN: ${DOMAIN}
    mem_limit: 1g
    memswap_limit: 1g
    cpus: 1.5
    restart: unless-stopped
    <<: *default-logging
```

    `volumes: !override` (Compose's merge tag for "replace, don't
    append"; `!reset []` only empties) replaces the base file's dev bind
    mounts with just the cache volume. Check this with `docker compose -f
    docker-compose.yml -f docker-compose.prod.yml config` (validation).
    The others:

| Service | `mem_limit` / `memswap_limit` | `cpus` |
|---|---|---|
| `postgres` | `768m` | `1.0` |
| `minio` | `512m` | `0.5` |
| `api` | `256m` | `1.0` |
| `web` | `1g` | `1.5` |
| `caddy` | `128m` | `0.5` |
| `dozzle` | `128m` | `0.25` |

5b. Add `web_cache:` under the top-level `volumes:` of
    `docker-compose.prod.yml`, next to `caddy_data` / `caddy_config`.

## Group 6 — Image optimizer: WebP only, longer cache (repo)

Depends on: nothing. File: `apps/web/next.config.ts`.

```ts
    formats: ["image/webp"],
    // 30 days. Menu photos are served immutable by the API
    // (apps/api/src/routes/media.rs) and static images only change on a
    // deploy, so re-encoding sooner is wasted CPU (Phase 18).
    minimumCacheTTL: 2592000,
```

Replace the existing `formats: ["image/avif", "image/webp"]` line and add
`minimumCacheTTL` beside it. Nothing else in the `images` block changes.

## Group 7 — Deploy prunes dangling images (repo)

Depends on: nothing. File: `.github/workflows/deploy.yml`, `deploy`
job's SSH `script:`. Append one line after the existing `restart caddy`:

```yaml
            docker image prune -f
```

(`-f` without `-a` removes only dangling, untagged images, the
superseded `:latest` layers. It never touches volumes or a running
container's image; requirement.md Decision 9.)

## Group 8 — Off-box monitoring (ops)

Depends on: Group 2 (alarms are per instance; set them on the new one).

8.1. Lightsail console → **Account** → **Notification contacts**: add the
     owner's email and confirm the verification email.

8.2. New instance → **Metrics** → add alarms (requirement.md
     Decision 7), each notifying by email:

| Metric | Condition | Datapoints |
|---|---|---|
| CPU utilization | ≥ 70% | 2 of 2 (5-min periods) |
| Burst capacity percentage | ≤ 20% | 1 of 1 |
| Instance status check failures | ≥ 1 | 1 of 1 |
| System status check failures | ≥ 1 | 1 of 1 |

8.3. External uptime monitor (any free-tier hosted HTTP uptime service,
     chosen by the user): two HTTP(S) monitors, 5-minute interval, email
     alerts:
     - `https://zainstreat.com/en`, expect `200`
     - `https://api.zainstreat.com/health`, expect `200`

## Group 9 — Docs + specs (repo)

Depends on: Groups 1–8 (documents what was actually done).

9a. **`docs/deployment.md`**:
    - Intro + "One-time manual setup": 2 GB → **4 GB**; add the swap
      file, the persistent journal (Group 3), and the alarms and uptime
      monitor (Group 8) to the one-time setup list.
    - New section **"When the site is down"** (requirement.md
      Decision 10): check the external monitor / Lightsail metrics (CPU
      *and* burst capacity) → reboot from the console → the Group 1.3
      evidence commands (copied verbatim) → where to record findings.
    - New short section **"Resizing the instance"**: the Group 2
      procedure (stop → snapshot → new instance from snapshot → firewall
      + automatic-snapshot check → move static IP → keep old instance
      stopped for 7 days).
    - "Backups": replace "Restoring from a snapshot has **not** been
      tested yet" with the date and outcome of the Group 2 restore.

9b. `specs/tech-stack.md` and `specs/roadmap.md` are already updated as
    part of this planning step. Nothing more to do unless Group 1's
    evidence changes a decision.

## Group 10 — SES go-live (ops)

Depends on: nothing; can run in parallel. **Start early**:
production-access review isn't instant (requirement.md Risk 5).

10.1. SES console (`eu-central-1`) → **Identities** → verify the
      `zainstreat.com` domain (DKIM CNAMEs at the DNS provider).

10.2. **Account dashboard** → **Request production access** (use case:
      transactional order confirmations + business notifications, low
      volume).

10.3. Generate SES SMTP credentials if not done yet. On the server, edit
      `/opt/zainstreat/.env`: real `EMAIL_FROM_ADDRESS` and
      `EMAIL_BUSINESS_NOTIFY_ADDRESS`, plus `SES_SMTP_HOST` /
      `SES_SMTP_USERNAME` / `SES_SMTP_PASSWORD`. Then `docker compose -f
      docker-compose.yml -f docker-compose.prod.yml up -d api` (env
      changes need a recreate, not just `restart`).

## Group 11 — Smoke test, tune, clean up (ops)

Depends on: everything above, deployed.

11.1. README §43 Definition-of-Done smoke test against the **live**
      domain, as listed in `validation.md`.

11.2. After **7 days** on the new box: `docker stats --no-stream` plus
      `sudo journalctl -k --since "7 days ago" | grep -i "killed process"`.
      If any container has been OOM-killed under normal traffic, or
      sits above ~80% of its `mem_limit`, raise that limit in
      `docker-compose.prod.yml` (requirement.md Risk 3) and redeploy.

11.3. Once 11.2 is clean: delete the old stopped 2 GB instance. **Keep**
      the `pre-4gb-upgrade-2026-10-11` snapshot.
