# Phase 18 — Production Hardening & Final Rollout: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 18, as
written before this phase's planning:

> The AWS Lightsail instance, DNS, TLS, backups, production Dockerfiles, and
> the GitHub Actions CI/CD pipeline already exist from Phase 2 — this phase
> hardens what's running on them rather than standing anything up from
> scratch.
>
> - Harden `apps/web/Dockerfile` and `apps/api/Dockerfile` (non-root user,
>   smaller/distroless base image where reasonable, resource limits).
> - Verify a Lightsail snapshot can actually be restored (test restore) now
>   that real menu/order data exists.
> - Swap Phase 17's placeholder `EMAIL_FROM_ADDRESS` /
>   `EMAIL_BUSINESS_NOTIFY_ADDRESS` values for real ones, verify the
>   sending domain in the SES console, and request SES production access
>   (exit the sandbox) — flagged as an open risk in that phase's
>   requirement.md so it isn't missed here.
> - Production smoke test against the README §43 Definition of Done
>   checklist, run against the live domain (not localhost).

Planning expanded this after a production outage (below).

## Context

**Incident, 2026-10-11.** The production Lightsail instance (2 GB,
`eu-central-1`) stopped responding completely: `zainstreat.com`,
`api.zainstreat.com`, and `logs.zainstreat.com` (Dozzle) were all
unreachable. The Lightsail CPU chart showed hours idle at ~3%, then a jump
at ~11:05 (UTC+1) to 36%, a peak of ~58% around 11:25–11:35, and then
irregular 5–20% readings afterwards. The burstable baseline for this plan
is ~10%. The user confirmed **this is not the first outage, and earlier
ones happened before Phase 17 shipped**, so Phase 17 isn't the only cause.

The CPU never came close to 100%, yet the whole box stopped responding,
even Caddy + Dozzle, which do almost nothing. That pattern usually means
**memory exhaustion**, not real CPU saturation: the kernel thrashes while
freeing memory, so CPU rises moderately and nothing answers. Root cause is
**not yet confirmed**. Group 1's evidence capture is what confirms it.

Planning research (read-only, against `master` at `a4791bf`) found the
conditions for that:

1. **No memory or CPU limits on any container.** `docker-compose.prod.yml`
   runs `postgres`, `minio`, `api`, `web`, `caddy`, and `dozzle` with no
   `mem_limit`/`cpus`, so any one of them can starve the rest, plus
   `sshd`/`dockerd`.
2. **No swap.** Neither `docs/deployment.md` nor the Phase 2 plan
   configures any. Lightsail images ship with none, so the first memory
   spike goes straight to the OOM killer, or to thrashing first.
3. **2 GB shared by six containers.** `tech-stack.md` called 2 GB
   "comfortable headroom" when there were five services and no on-the-fly
   image encoding. Dozzle (Phase 8) and the `next/image` optimizer
   (Phase 17) were added since.
4. **On-the-fly AVIF encoding (Phase 17).** `apps/web/next.config.ts` sets
   `images.formats: ["image/avif", "image/webp"]`. AVIF encoding through
   `sharp` is CPU- and memory-heavy. Phase 17 also switched the optimizer
   on for production menu photos and shipped `sitemap.ts`/`robots.ts`,
   which invites crawlers to request every page and so every image size.
   This is a likely trigger for the latest outage, not the whole cause
   (see the user's answer above).
5. **The image cache doesn't survive deploys.** `next/image` caches
   encoded output in `/app/.next/cache/images` inside the `web`
   container's writable layer. Every deploy (`up -d` with a new image)
   recreates the container and drops that cache, so after a deploy every
   image is encoded again on first request.
6. **Deploys never prune old images.** `.github/workflows/deploy.yml`
   runs `pull` + `up -d` with no `docker image prune`. Each deploy leaves
   the previous `web`/`api` images on disk, so disk use grows without
   bound. Probably not the cause yet, but it would be the next one.
7. **Logs live only on the box.** Dozzle and `docker compose logs` both
   need the box to be up, so when it hangs there's nothing to look at. No
   external uptime check exists either. The user found out about the
   outage themselves.
8. **Docs disagree about the OS.** `tech-stack.md` says "Ubuntu LTS
   blueprint"; `docs/deployment.md` and the Phase 2 plan say Amazon Linux
   2023, which is what's actually running. This phase corrects
   `tech-stack.md`.

The original Phase 18 bullets still apply. The Dockerfile "resource
limits" bullet is now urgent, and the snapshot-restore bullet now has a
real reason to run: the plan upgrade **is** a restore from a snapshot.

Branches off `master` (Phase 17 merged as `a4791bf`).

## Decisions

1. **Upgrade to the 4 GB Lightsail plan, and harden anyway.** Decided
   with the user (Recommended option). The upgrade is done the only way
   Lightsail allows: snapshot → new instance from that snapshot on the
   larger plan → move the static IP over. Moving the static IP means DNS
   doesn't change. The upgrade alone isn't enough: without limits, one
   runaway container can still fill 4 GB. So swap and per-container limits
   (Decisions 3–4) ship too.
2. **The upgrade doubles as the snapshot-restore test.** The original
   Phase 18 bullet "verify a Lightsail snapshot can actually be restored"
   is satisfied by actually restoring one: the new 4 GB instance boots
   from a snapshot of the old box with real menu/order data, and the
   smoke test (Group 8) proves it works. The old 2 GB instance is kept,
   **stopped and not deleted**, until the new one has run cleanly for
   7 days. Rollback is moving the static IP back.
3. **A 2 GB swap file on the host.** A cushion, not extra capacity. With
   `vm.swappiness=10`, swap absorbs a short spike instead of setting off
   the OOM killer or a thrash spiral. Configured once by hand on the
   server, like the rest of the host setup in `docs/deployment.md`.
4. **Memory and CPU limits on every container, in
   `docker-compose.prod.yml` only.** Dev stays unlimited. The budget for
   the 4 GB box:

   | Service | `mem_limit` | `cpus` | Notes |
   |---|---|---|---|
   | `postgres` | 768m | 1.0 | Default `shared_buffers` (128 MB) fits comfortably |
   | `minio` | 512m | 0.5 | |
   | `api` | 256m | 1.0 | Rust/Axum; small resident set |
   | `web` | 1g | 1.5 | Next.js SSR + `sharp`, the one that spikes |
   | `caddy` | 128m | 0.5 | |
   | `dozzle` | 128m | 0.25 | |
   | **Total** | **~2.8 GB** | | Leaves ~1.2 GB + swap for the OS, `dockerd`, `sshd`, page cache |

   With limits in place, a spike gets *that* container OOM-killed and
   restarted (`restart: unless-stopped`) instead of hanging the whole box.
   SSH, Caddy, and Dozzle stay up, so the incident can be diagnosed.
   `memswap_limit` is set equal to `mem_limit` so a capped container can't
   quietly use up the host's swap. These numbers are starting points;
   Group 9 revisits them against a week of `docker stats`.
5. **Drop AVIF, keep WebP.** Decided with the user (Recommended option).
   `images.formats` becomes `["image/webp"]`. This partly reverses
   Phase 17's image-optimization bullet: WebP keeps most of the size
   savings at a fraction of the encoding cost. Phase 17's other image
   changes (production photos going through the optimizer, `sizes`
   props, `priority` audit) stay.
6. **Keep the optimizer cache across deploys, and keep it longer.** A
   named volume `web_cache` is mounted at `/app/.next/cache`, so a deploy
   no longer forces every image to be encoded again.
   `images.minimumCacheTTL` is raised to 30 days (2,592,000 s). Menu
   photos are already served `immutable` with a 1-year max-age by
   `apps/api/src/routes/media.rs:47`, and the static site images only
   change on a deploy. Each encode then happens about once per
   image/width combination, not once per cache expiry.
7. **Lightsail metric alarms plus an external uptime check, both
   off-box.** Decided with the user (Recommended option). This
   covers detecting an outage and keeping enough evidence to diagnose it
   even when the box can't be reached:
   - **Lightsail alarms** (console, emailed to the owner): CPU
     utilization above 70% for 2 consecutive 5-minute periods; **burst
     capacity percentage** below 20%; instance status check failed and
     system status check failed (each ≥1 for 1 period).
   - **External HTTP uptime check**: a free-tier monitor at a hosted
     uptime service, set up and chosen by hand, polling `https://zainstreat.com`
     and `https://api.zainstreat.com/health` every 5 minutes and alerting
     by email. No account or service name is pinned here, and no code
     depends on it.
   - **Persistent systemd journal** on the host
     (`Storage=persistent`), so after a hang and reboot,
     `journalctl -k -b -1` still has the previous boot's OOM-killer and
     kernel messages. This is the cheap version of off-box logs; a
     CloudWatch agent was offered and declined (Out of scope).
8. **Dockerfile hardening: non-root, keep the Debian-slim bases.** Both
   runtime stages run as a non-root user: `node` (already in the
   `node:22-bookworm-slim` image) for `web`, and a new `app` system user
   for `api`. **No distroless switch.** The base compose file's `api`
   healthcheck runs `curl`, and the runtime image installs `curl` for
   exactly that. Distroless has no shell or `curl`, so switching would
   mean rewriting the healthcheck around a custom binary, which is
   churn without a clear benefit at this scale. The roadmap's "where
   reasonable" covers this. The `web` runtime stage creates
   `/app/.next/cache` owned by `node` before switching user, so the
   Decision 6 volume is writable. The "resource limits" in the original
   bullet are applied in compose (Decision 4), not in the Dockerfiles,
   which can't express them.
9. **Deploys prune dangling images.** `.github/workflows/deploy.yml`'s
   SSH script adds `docker image prune -f` after `up -d`. This removes
   only untagged images left behind by the newer `:latest`, never a
   running container's image and never volumes. Named volumes
   (`postgres_data`, `minio_data`, `caddy_data`, `web_cache`) are out of
   reach of `image prune`.
10. **Incident runbook goes in `docs/deployment.md`.** A new "When the
    site is down" section: reboot from the Lightsail console, then the
    exact evidence-capture commands (previous-boot kernel log for OOM
    kills, `free -m`, `df -h`, `docker stats --no-stream`, the most
    requested Caddy URIs), and where to look in the Lightsail metrics
    (CPU *and* burst capacity). It's written down so the next outage
    leaves evidence, unlike this one.
11. **The original SES and smoke-test bullets are unchanged in scope.**
    The real `EMAIL_FROM_ADDRESS`/`EMAIL_BUSINESS_NOTIFY_ADDRESS`, SES
    domain verification, and the production-access request stay manual
    console/`.env` steps (Phase 17 Decision 2 / Risk 3). The README §43
    Definition-of-Done smoke test runs **last**, against the live domain,
    on the upgraded and hardened box, so it validates the final state.

## Out of scope

- **A CloudWatch agent / shipping logs and memory metrics off-box.**
  Offered and declined (Decision 7); revisit if a future outage still
  can't be diagnosed from the persistent journal plus Lightsail metrics.
- **Distroless runtime images.** Decision 8.
- **Rate limiting or bot-blocking `/_next/image` at Caddy.** Stock
  `caddy:2-alpine` has no rate-limit module; that would mean a custom
  Caddy build. Dropping AVIF and the persistent cache (Decisions 5–6)
  should make crawler load cheap enough. Revisit only if the evidence
  shows abusive crawl traffic specifically.
- **Pre-optimizing static images at build time / bypassing the
  optimizer.** Offered as an alternative to Decision 5 and not chosen.
- **Moving off Lightsail** (EC2, managed Postgres, etc.).
  `tech-stack.md`'s "revisit if the app ever outgrows one Lightsail
  instance" still holds; 4 GB is well within one box.
- **Postgres tuning** beyond fitting its default config inside the
  Decision 4 limit.
- Everything already deferred to "Beyond MVP" in `roadmap.md`.

## Open risks flagged during planning

1. **Root cause is a hypothesis until Group 1 runs.** If the evidence
   shows something other than memory pressure (a full disk, one specific
   container crash-looping, burst credits at zero from steady load), the
   Group 2+ order still holds, but Group 9's limit-tuning step must
   address what the evidence actually shows, and `requirement.md` gets a
   short "Confirmed cause" addendum.
2. **Upgrade downtime.** Snapshot → new instance → static-IP move is
   roughly 15–30 minutes of downtime, plus however long the site is
   already down. The snapshot captures disk state as it is at snapshot
   time, so any order placed on the old box after the snapshot (if it
   comes back up in between) is lost when the IP moves. Mitigation: run
   Group 2 with the old instance **stopped** before the snapshot is
   taken.
3. **Containers getting OOM-killed is the intended failure mode now.**
   With limits, a spike kills and restarts `web` (a few seconds of 502s
   from Caddy) instead of hanging the box. That's the trade-off, but if
   `web` keeps hitting its limit in normal traffic, the 1 GB number is
   wrong, not the approach. Group 9 checks this.
4. **Non-root `web` plus an existing cache volume.** If `web_cache` is
   ever created by a root-running container first, the `node` user can't
   write to it and the optimizer fails. Prevented by shipping the
   Dockerfile change (Group 4) in the same deploy as the volume (Group 5),
   never the volume alone.
5. **SES production access isn't instant.** AWS reviews the request,
   which can take a day or more. The smoke test's email checks can't pass
   until it's approved, so Phase 18 may sit "done except SES" for that
   long.
