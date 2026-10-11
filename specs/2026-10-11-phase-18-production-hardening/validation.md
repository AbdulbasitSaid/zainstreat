# Phase 18 — Production Hardening & Final Rollout: Validation

## Pass/fail checklist

### Incident evidence (Group 1)

- [ ] Lightsail CPU **and** burst-capacity charts for the outage window
      captured.
- [ ] Group 1.3 commands run on the recovered box; output saved.
- [ ] `requirement.md` has a "Confirmed cause" addendum (or "inconclusive:
      no persistent journal" if `journalctl -b -1` had nothing; Group 3
      prevents that next time).

### Instance upgrade + snapshot restore (Group 2)

- [ ] Snapshot `pre-4gb-upgrade-2026-10-11` exists and is *Available*.
- [ ] New instance is on the 4 GB plan; `free -m` shows ≈ 3.8 GiB total.
- [ ] New instance firewall is exactly `22`/`80`/`443` (IPv4 and IPv6).
- [ ] Automatic snapshots are enabled on the new instance.
- [ ] Static IP is attached to the new instance; `dig +short
      zainstreat.com` still returns the same static IP.
- [ ] Restored data is intact: live `/en/menu` shows the real menu with
      photos (MinIO), and `/admin` lists orders placed before 2026-10-11
      (Postgres).
- [ ] Old 2 GB instance is **stopped**, not deleted.

### Host hardening (Group 3)

- [ ] `swapon --show` lists `/swapfile` at 2G **after a reboot**.
- [ ] `cat /proc/sys/vm/swappiness` → `10`.
- [ ] `sudo journalctl --list-boots` shows ≥ 2 boots, and `sudo
      journalctl -k -b -1 | head` returns the previous boot's kernel log.

### Repo changes (Groups 4–7), local

- [ ] `docker compose -f docker-compose.yml -f docker-compose.prod.yml
      config` succeeds, and in its output: every one of `postgres`,
      `minio`, `api`, `web`, `caddy`, `dozzle` has `mem_limit`,
      `memswap_limit` (equal to `mem_limit`), and `cpus` matching
      requirement.md Decision 4; `web.volumes` is **only**
      `web_cache:/app/.next/cache` (no `./apps/web` bind mount, no
      `web_node_modules`); top-level `volumes` includes `web_cache`.
- [ ] `docker build -t zt-web-test apps/web` and `docker build -t
      zt-api-test apps/api` both succeed.
- [ ] `docker run --rm zt-web-test whoami` → `node`; `docker run --rm
      --entrypoint whoami zt-api-test` → `app`.
- [ ] `docker run --rm zt-web-test stat -c '%U' /app/.next/cache` →
      `node`.
- [ ] `grep -n 'formats' apps/web/next.config.ts` shows
      `["image/webp"]` only (no `avif`); `minimumCacheTTL: 2592000` is
      present.
- [ ] `grep -n 'image prune' .github/workflows/deploy.yml` → exactly one
      `docker image prune -f` line, **without** `-a` or `--volumes`.
- [ ] `/validate` passes (lint, build, live dev smoke test). Dev compose
      is unchanged, so dev must behave exactly as before.

### Repo changes (Groups 4–7), after deploy to production

- [ ] GitHub Actions `Deploy` run for the merge is green, including the
      prune step.
- [ ] `docker compose ... ps`: all six services `Up`, `api` healthy.
- [ ] `docker stats --no-stream`: the `MEM USAGE / LIMIT` column shows
      the Decision 4 limits, not the host total.
- [ ] `docker compose ... exec web whoami` → `node`; `... exec api
      whoami` → `app`.
- [ ] `docker compose ... exec api create_admin --help` (or a dry
      invocation) still runs as the non-root user.
- [ ] Image optimizer writes to the volume: request a menu image, e.g.
      `curl -sI "https://zainstreat.com/_next/image?url=<encoded
      menu photo url>&w=640&q=75" -H 'Accept: image/avif,image/webp'`
      → `200`, `content-type: image/webp` (**not** `image/avif`). Then
      `docker compose ... exec web ls .next/cache/images | head` is
      non-empty.
- [ ] Cache survives a redeploy: `docker compose ... up -d --force-recreate
      web`, repeat the same request, and `x-nextjs-cache: HIT` on the
      first try.
- [ ] Containment check: `docker compose ... exec web node -e
      "const a=[];setInterval(()=>a.push(Buffer.alloc(5e7,1)),100)"`
      gets killed (exit 137) within seconds, `sudo journalctl -k | tail`
      shows the OOM kill scoped to that container's cgroup, and
      throughout `curl -s https://api.zainstreat.com/health` and SSH stay
      responsive. Afterwards `web` is `Up`: either the server process
      survived, or the kernel picked it instead and `restart:
      unless-stopped` brought it back within seconds. Either outcome
      passes; a hung host does not.

### Monitoring (Group 8)

- [ ] Lightsail notification email contact is verified.
- [ ] Four alarms exist on the new instance (CPU ≥ 70%, burst capacity
      ≤ 20%, instance status check, system status check), each with email
      notification on.
- [ ] Use **Test alarm** on at least one alarm → the email arrives.
- [ ] External uptime monitors exist for `https://zainstreat.com/en` and
      `https://api.zainstreat.com/health`, both currently *Up*.
- [ ] Alert path works end to end: `docker compose ... stop api` for
      ~10 minutes → the API monitor sends a *Down* email → `start api` →
      *Up* email. (Pick a quiet time.)

### Docs (Group 9)

- [ ] `docs/deployment.md` says 4 GB, documents swap, the persistent
      journal, alarms, and the uptime monitor, and has the "When the
      site is down" and "Resizing the instance" sections.
- [ ] `docs/deployment.md`'s Backups section records the 2026-10-11
      snapshot restore as tested.

### SES (Group 10)

- [ ] `zainstreat.com` shows *Verified* in SES Identities
      (`eu-central-1`).
- [ ] SES account is out of the sandbox (production access granted).
- [ ] Server `.env` has real `EMAIL_FROM_ADDRESS` /
      `EMAIL_BUSINESS_NOTIFY_ADDRESS` (no Phase 17 placeholders);
      `docker compose ... logs api | grep -i "email disabled"` returns
      nothing after the `api` recreate.

### README §43 Definition of Done, live domain (Group 11.1)

Against `https://zainstreat.com`, not localhost, on a phone **and** a
desktop browser:

- [ ] All public pages load (`/`, `/about`, `/services`, `/menu`,
      `/contact`, `/terms`, `/food-regulations`) in both `/en` and `/nl`.
- [ ] Menu browsing + category filtering work.
- [ ] Cart: add, remove, change quantity.
- [ ] A real test order submits, is persisted (visible in `/admin`), and
      the customer confirmation email **arrives**.
- [ ] Admin: log in, view orders, update an order's status.
- [ ] Admin: add, edit, archive a menu item; toggle availability; manage
      categories. An archived/unavailable item can't be ordered.
- [ ] The test order still shows its original item name and price after
      editing that item's price.
- [ ] Catering and contact enquiries submit, and the business
      notification email **arrives**.
- [ ] WhatsApp CTAs open `wa.me` with the prefilled message.
- [ ] Form validation errors show on Contact and Order.
- [ ] `/admin` redirects to login when signed out; `/api/admin/*` →
      `401` when signed out.
- [ ] `/sitemap.xml` and `/robots.txt` are served; page `<title>`/
      `og:image` are present.
- [ ] Menu images are served as WebP through `/_next/image`.
- [ ] Clean up the test order, enquiries, and menu item afterwards.

### After 7 days (Group 11.2–11.3)

- [ ] `sudo journalctl -k --since "7 days ago" | grep -i "killed
      process"` → empty, or every hit has led to a raised limit in
      `docker-compose.prod.yml`.
- [ ] No uptime-monitor *Down* alerts in the 7 days (or each one
      explained).
- [ ] Old 2 GB instance deleted; `pre-4gb-upgrade-2026-10-11` snapshot
      kept.

## Definition of done

Production runs on a 4 GB Lightsail instance restored from a snapshot of
the old box, which proves for the first time that the backup mechanism
actually restores. Every container has a memory and CPU ceiling, so one
runaway service gets OOM-killed and restarted instead of hanging the whole
host. A 2 GB swap file absorbs short spikes, and a persistent journal keeps
the evidence across a reboot. The `next/image` optimizer encodes WebP only,
caches for 30 days, and keeps that cache across deploys, and deploys no
longer pile up old images on disk. An outage is now detected off-box, by
Lightsail alarms and an external uptime monitor that both email the owner,
and `docs/deployment.md` has a step-by-step "When the site is down"
runbook. Both Dockerfiles run as non-root users. Email sends from a
verified domain with SES production access and real addresses, and the
full README §43 Definition-of-Done checklist passes against the live
domain. The box then runs a week without an OOM kill or uptime alert
before the old instance is deleted.
