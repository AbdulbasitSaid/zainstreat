# Phase 21 — Pipeline Refactor: Staging, No-SSH Ops, `cms.` Admin: Requirements

## Source

New phase, not in the original roadmap. Requested on 2026-10-31:

> - we need a staging server and staging deploy.
> - reduce the need to SSH into the Lightsail server as much as possible
> - admin should be on a subdomain rather than /admin. It should be
>   cms.zain... in production.
> - for staging don't set a custom domain.
> - I should be able to shut down the staging when not in use.

Inserted as Phase 21 ahead of the remaining client-call phases (now
22–29). It makes Phase 29 (move to the client's AWS account) mostly a
matter of running workflows.

## Context

### Today's pipeline

`.github/workflows/deploy.yml` runs on every push to `master`:

1. **`test-api`:** `cargo test` against Postgres and MinIO.
2. **`build-and-push`:** builds `zainstreat-{api,web}` and pushes them to
   GHCR as `:latest` and `:<sha>`. The web image gets `DOMAIN` as a build
   argument (Phase 19).
3. **`deploy`:** scp's `docker-compose.yml`, `docker-compose.prod.yml` and
   `Caddyfile` to `/opt/zainstreat`, then over SSH runs
   `compose pull && up -d`, restarts Caddy and prunes images.

Gaps in that flow:
- There is no staging, approval gate, post-deploy health check or
  concurrency guard.
- `docker-compose.prod.yml` hard-codes
  `ghcr.io/abdulbasitsaid/zainstreat-*:latest`, so production always runs
  whatever was pushed last.

### Manual SSH that still exists

From `docs/deployment.md` and the Phase 2/18/19 plans:
- **Instance setup:** dnf update, Docker, swap, persistent journald,
  the `deploy` user, a one-time GHCR `docker login` with a PAT, and a git
  clone of the repo into `/opt/zainstreat`. CI overwrites the compose
  files there, so the clone drifts from git.
- **`/opt/zainstreat/.env`:** written and edited by hand for every secret
  or log-level change.
- **`create_admin`:** run by hand. It only reads the password from an
  interactive `rpassword` prompt (`apps/api/src/bin/create_admin.rs`).
- **Rollback:** pull a `:<sha>` image by hand, or re-run an old Actions
  run, which rebuilds it.
- **Restore** from `backups/*.sql`. Those dumps only live on the box;
  Lightsail snapshots are the only off-box copy.

### The admin today

The admin lives at `apps/web/app/admin/**`, a separate root layout, and
is served at `/admin` on the public host.

- **Path matching:** `proxy.ts` (next-intl) skips it with the matcher
  `/((?!api|admin|_next|_vercel|.*\\..*).*)`.
- **API calls:** the browser calls same-origin route handlers under
  `app/api/admin/*`, which forward the cookie to `http://api:8080`.
- **Session cookie:** tower-sessions `id`, `SameSite=Lax`, **no Domain
  attribute**. It is host-only, so it follows whichever host served the
  login.
- **CORS:** the API has no CORS layer, and none is needed.
- **Hard-coded paths:** about 30 places in components and pages, for
  example:
  - `components/admin-nav.tsx`
  - `admin-login-form.tsx`
  - `(protected)/layout.tsx`, whose guard does `redirect("/admin/login")`
  - every `(protected)/**` page

### Lightsail billing

Lightsail bills a **stopped** instance at the full rate. "Shut down when
not in use" therefore means deleting the staging instance, not stopping
it.

## Decisions

1. **Staging is an ephemeral Lightsail instance.**
   - "Staging up" creates `zainstreat-staging` (2 GB plan, Amazon Linux
     2023, `eu-central-1`) from a launch script, deploys and reseeds it.
   - "Staging down" deletes it.
   - It costs about the hourly price while up and nothing while down.
     Data does not survive a down/up cycle; that's accepted.
   - **Rejected:**
     - **EC2 stop/start:** a second AWS compute service to maintain.
     - **A second compose project on the production box:** RAM
       contention after the 2026-10-11 OOM outage.
     - **A persistent Lightsail instance that is only stopped:** still
       billed in full.
2. **Production and staging use the same machinery.** One bootstrap
   script, one compose stack, one `Caddyfile` and one reusable deploy
   workflow serve both, parameterized by a GitHub Environment.
   - Production stays long-lived: static IP `63.185.172.26`, Lightsail
     snapshots, and never created or deleted by automation.
   - The bootstrap script is still the documented way to build a new
     production box (Phase 29, disaster recovery).
3. **Staging has no custom domain. It uses
   `DOMAIN=<ip-with-dashes>.sslip.io`.**
   - sslip.io resolves any subdomain to the embedded IP, so `api.`,
     `cms.` and `logs.` work unchanged.
   - Caddy gets real Let's Encrypt certificates, and there are no DNS
     records to create or delete.
4. **Releases.**
   - A merge to `master` builds images tagged `:<sha>`. When staging is
     up, it also deploys there automatically.
   - **Production deploys only through a manual `promote` workflow** that
     deploys an already-built SHA. It is gated by a required reviewer on
     the `production` GitHub Environment.
   - **Rollback is `promote` with an older SHA.** No `:latest` tag is
     deployed anywhere any more.
   - This replaces tech-stack's "true continuous deployment — no manual
     approval gate".
5. **The web image is built per environment.** `DOMAIN` stays baked in at
   build time (Phase 19 Decision 1; "allow any hostname" was rejected
   there and still is).
   - The API image is built once and promoted as-is.
   - The production web image is built at merge time as
     `zainstreat-web:<sha>`.
   - The staging web image (`zainstreat-web-staging:<sha>`) is built for
     the current staging IP.
   - Promote never rebuilds. It deploys the prebuilt production image for
     that SHA.
6. **The server `.env` is generated, never hand-edited.**
   - Each deploy renders it from the target GitHub Environment's secrets
     and variables, then uploads it with mode `600`.
   - `.env.example` becomes the documented list of those secrets and
     variables.
   - Compose takes full image references, `API_IMAGE` and `WEB_IMAGE`,
     from `.env`.
7. **Images are pulled with a short-lived token.** Each deploy runs
   `docker login ghcr.io` with the job's `GITHUB_TOKEN`
   (`packages: read`), pulls, then runs `docker logout`. The long-lived
   read-only PAT on the production box is retired.
8. **AWS is reached through GitHub OIDC.** Workflows assume one IAM role,
   `zainstreat-github-actions`, restricted to this repo.
   - Lightsail: only the instance actions staging needs. Deleting an
     instance and changing its firewall ports are limited to resources
     tagged `env=staging`.
   - S3: only the backup bucket.
   - No AWS access keys are stored in GitHub.
9. **`infra/bootstrap.sh` is the Lightsail launch script (user-data).** It
   reproduces the Phase 2/18 manual setup and is idempotent:
   - dnf update; Docker and the Compose plugin
   - 2 GB swap with `vm.swappiness=10`; persistent journald
   - a sudo-less `deploy` user in the `docker` group, with its
     `authorized_keys` taken from the Environment's deploy public key
   - `/opt/zainstreat` owned by `deploy`, with no git clone
   - a marker file, `/opt/zainstreat/.bootstrap-done`

   It contains no secrets. Only the public key is injected.
10. **Ops tasks become workflows.**
    - **`create-admin`:** `create_admin` gains a `--password-stdin` flag.
      The password comes from an Environment secret, never from a
      workflow input, because inputs are visible in the run UI. This
      reverses the "never through CI" rule in `docs/deployment.md`: a
      masked secret piped over stdin is no weaker than typing it over
      SSH, and it removes a routine SSH session.
    - **`backup`:** nightly for production, and on demand. It streams a
      `pg_dump -Fc` and a MinIO data archive over SSH to the runner, which
      uploads them to a private S3 bucket. The box never holds AWS
      credentials.
    - **`restore`:** restores a chosen backup into an Environment.
    - **`seed-production`:** moves onto the new SSH/Environment pattern,
      with the same behavior.
11. **The admin moves to `cms.${DOMAIN}`.** That is `cms.zainstreat.com`
    in production, `cms.<ip>.sslip.io` on staging and `cms.localhost:3000`
    in dev.
    - **Same Next.js app.** `app/admin/**` and `app/api/admin/**` stay
      where they are.
    - **Host-aware `proxy.ts`:**
      - On a `cms.` host it rewrites `/x` → `/admin/x`; `/api/*` is
        untouched.
      - On the public host, `/admin*` gets a 308 redirect to the same
        path on `cms.`.
      - On the public host, `/api/admin/*` returns 404.
    - **Links:** the ~30 hard-coded `/admin/...` paths lose the prefix.
    - **Caddy:** a `cms.{$DOMAIN}` block, which always sends
      `X-Robots-Tag: noindex, nofollow`.
    - **No API, cookie or CORS change.** The host-only session cookie now
      belongs to `cms.`, so admins log in once more after cutover.
12. **Staging is kept out of search and away from the business.**
    - Caddy adds `X-Robots-Tag: noindex, nofollow` to every staging site
      (driven by an env var).
    - `EMAIL_BUSINESS_NOTIFY_ADDRESS` in the `staging` Environment is the
      developer's address.
13. **Deploy safety.**
    - `concurrency: deploy-<env>` with no cancel-in-progress.
    - After `up -d`, the deploy fails unless these return success within
      about 3 minutes: `https://api.$DOMAIN/health`, `https://$DOMAIN/en`
      and `https://cms.$DOMAIN/login`.
    - There is no automatic rollback; a human runs `promote` with the
      previous SHA.

14. **The deploy checks env parity before replacing `.env`** (Risk 1).
    - `_deploy` renders `.env.next` on the box next to the live `.env`.
    - A script on the box runs `docker compose config` with each file,
      hashes every resolved value per service, and prints **only the
      names** of keys that differ. Values never leave the box.
    - **On production,** the deploy fails on any difference unless the
      dispatch input `allow_env_changes` names that key. On staging it
      only reports them, so automatic staging deploys aren't blocked.
      Only after the check passes does `mv .env.next .env` run.
    - `API_IMAGE`/`WEB_IMAGE` change on every deploy and are excluded.
      A fresh box with no `.env` skips the check.
    - The renderer single-quotes every value, so `$` in the Dozzle bcrypt
      hash is literal. It rejects values containing `'` or a newline.
    - Because resolved values are compared, a `$$` vs `'$'` style
      difference that resolves to the same string passes. The first
      production promote is provably a no-op for credentials.
15. **The backup key can only stream backups** (Risk 2). It is a
    dedicated SSH key whose `authorized_keys` line is
    `restrict,command="/opt/zainstreat/bin/backup-stream" <key>`.
    - `infra/backup-stream.sh` accepts only `db` (`pg_dump -Fc`) or
      `minio` (tar of `/data`) from `$SSH_ORIGINAL_COMMAND`, and refuses
      anything else.
    - `_deploy` installs the script and maintains that line idempotently.
    - `production-backup` holds this key, never the deploy key, so the
      nightly job without a reviewer can't deploy or run commands.
      Restore uses the deploy key behind the `production` reviewer gate.
    - The script reads the database name and user from the `postgres`
      container's own environment, so it never parses `.env` on the
      host.
16. **The staging DNS suffix is a variable** (Risk 3).
    `STAGING_DNS_SUFFIX` (default `sslip.io`, fallback `nip.io`, both
    using the dashed-IP format) is used everywhere a staging domain is
    derived.
17. **Caddy has an ACME account email** (Risk 4). A global
    `email {$ACME_EMAIL}` enables Caddy's automatic ZeroSSL fallback when
    Let's Encrypt rate-limits. `staging-up` on a running instance
    redeploys in place and reuses its certificates. The run summary
    prints the certificate issuer.
18. **Bootstrap failures are visible** (Risk 5).
    - `bootstrap.sh` logs to `/var/log/zainstreat-bootstrap.log`, which
      is readable by `deploy`.
    - An `ERR` trap writes `/opt/zainstreat/.bootstrap-failed`.
    - The deploy's wait loop stops on `.bootstrap-done` (continue),
      `.bootstrap-failed` or a 10-minute timeout. On either failure it
      prints the log tail.
19. **Phase 20 merges first; Phase 21 rebases onto it** (Risk 6). A plan
    group lists the reconciliation:
    - renumber Phase 20's references to old Phases 21–24
    - resolve the `roadmap.md` conflict
    - carry Phase 20's env vars (`PUBLIC_SITE_URL`, `BANK_*`,
      `BUSINESS_KVK`) into the Environments
    - strip `/admin` from the admin pages Phase 20 touched
20. **The API gets `PUBLIC_ADMIN_URL`** (Risk 7). It is
    `https://cms.${DOMAIN}` in prod compose and
    `http://cms.localhost:3000` in dev. Phase 20's owner emails link to
    `{admin_url}/orders/{id}` instead of `{site_url}/admin/orders/{id}`.
    The `/admin` → `cms.` 308 keeps path and query, so emails and
    bookmarks sent before the cutover still land on the right page.
21. **GHCR job-token pulls are checked before they're relied on**
    (Risk 8).
    - **Linking is confirmed:** on 2026-10-31 `gh api` showed both
      packages linked to `AbdulbasitSaid/zainstreat`.
    - **The images are made private:** the same check found both
      packages **public**, despite `tech-stack.md`'s "private GHCR". You
      chose private: they hold compiled application code, and only this
      repo's deploys need them. The same applies to the new
      `zainstreat-web-staging` package.
    - The first staging deploy proves job-token pulls of the private
      images end to end.
    - `GHCR_PULL_TOKEN` stays as the documented fallback.
22. **`promote` refuses to deploy production before the `cms` DNS
    record exists** (Risk 9). The pre-approval `verify` job checks that
    `dig +short cms.$DOMAIN` equals `DEPLOY_HOST`. Without that record,
    Caddy's failed ACME attempts burn rate limit and the health check
    fails.


## Out of scope

- Moving to the client's AWS account. That is Phase 29, which this phase
  enables.
- Closing port 22. CI still deploys over SSH.
  - SSM is not natively supported on Lightsail. Registering the box as a
    hybrid managed instance is a heavier change and is deferred.
  - Lightsail's browser SSH stays the emergency fallback.
- Building one web image for every environment, i.e. a runtime `DOMAIN`.
  Phase 19 rejected the open-optimizer route; revisit only if per-env
  builds get slow.
- Preview environments per pull request.
- Terraform or CDK for the production instance, static IP, alarms or DNS.
  These stay console-managed, as in `tech-stack.md`.
- Staging data persistence. Reseeding on every "up" is accepted
  (Decision 1). Restoring a production backup into staging is possible
  with `restore`, but it is not automatic.

## Open risks flagged during planning

Risks 1, 2, 5, 6, 8 and 9 are closed by Decisions 14–22 (see the
Resolution below). What remains is accepted:

1. **sslip.io is a third-party DNS service.** If it is down, staging is
   unreachable until `STAGING_DNS_SUFFIX` is switched to `nip.io`.
   Production doesn't depend on it.
2. **Certificates on a recycled IP.** If Lightsail hands staging an IP
   whose sslip.io hostnames hit Let's Encrypt's duplicate-certificate
   limit (5 per week), Caddy falls back to ZeroSSL. Staging still gets
   valid HTTPS, just from a different issuer.
3. **One re-login for admins at cutover.** The session cookie is
   host-only, so sessions from `zainstreat.com` don't carry over to
   `cms.`. The cutover is scheduled at a quiet time and the owner is told
   beforehand (plan Group 12).

### Resolution (risk review with the user, 2026-10-31)

| Risk raised in planning | Resolution |
|---|---|
| 1. The first generated `.env` must match the existing Postgres/MinIO credentials, and `$` escaping | Decision 14: env parity preflight on every deploy, single-quoted rendering |
| 2. The backup key had the same power as the deploy key | Decision 15: dedicated key with a forced command |
| 3. sslip.io outage | Decision 16: `STAGING_DNS_SUFFIX` variable. Residual risk 1 |
| 4. Let's Encrypt rate limits on sslip.io | Decision 17: ACME email + ZeroSSL fallback, in-place redeploys. Residual risk 2 |
| 5. Bootstrap asynchronous or failing silently | Decision 18: log + failure marker + bounded wait |
| 6. Phase 20 branch numbering and overlap | Decision 19: Phase 20 merges first; reconciliation group |
| 7. Cutover breaks admin links and sessions | Decision 20: `PUBLIC_ADMIN_URL`, path-preserving 308. Residual risk 3 |
| 8. GHCR pulls with the job token | Decision 21: linking verified 2026-10-31; packages found public and made private; fallback token documented |
| 9. (new) `cms.` certificate requested before DNS exists | Decision 22: DNS gate in `promote` |
