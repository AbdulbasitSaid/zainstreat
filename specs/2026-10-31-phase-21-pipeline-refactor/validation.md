# Phase 21 — Pipeline Refactor: Staging, No-SSH Ops, `cms.` Admin: Validation

## Pass/fail checklist

### AWS (Group 1)

- [ ] IAM lists the OIDC provider `token.actions.githubusercontent.com`.
- [ ] The trust policy of `zainstreat-github-actions` limits `sub` to
      `repo:AbdulbasitSaid/zainstreat:*`.
- [ ] A workflow step running `aws sts get-caller-identity` prints the
      assumed-role ARN. No `AWS_ACCESS_KEY_ID` secret exists in the repo.
- [ ] Assuming the role, `delete-instance` on a throwaway instance
      **without** the `env=staging` tag fails with `AccessDenied` (never
      test this against production).
- [ ] The backup bucket has Block Public Access on and a 30-day expiry
      rule.

### GitHub (Group 2)

- [x] `gh api` shows `zainstreat-api` and `zainstreat-web` linked to
      `AbdulbasitSaid/zainstreat` (checked 2026-10-31).
- [ ] The same `gh api` check reports `private` for `zainstreat-api`,
      `zainstreat-web` and, after the first staging build,
      `zainstreat-web-staging`.
- [ ] Logged out, `docker pull ghcr.io/abdulbasitsaid/zainstreat-api:<sha>`
      fails with `denied`/`unauthorized`.
- [ ] The Environments `staging`, `production` and `production-backup`
      exist.
- [ ] `production` has a required reviewer, and `production` and
      `production-backup` only accept deployments from `master`.
- [ ] Every name in `.env.example` exists in `production` and `staging`,
      as a secret or variable per the plan table.
- [ ] The `production` values match the old server `.env` (checked
      before cutover).

### Bootstrap (Group 3)

- [ ] On a fresh staging box, `/opt/zainstreat/.bootstrap-done` exists
      and `id deploy` shows the `docker` group without `wheel`.
- [ ] On the same box: `swapon --show` shows a 2G `/swapfile`, and
      `sysctl vm.swappiness` gives 10.
- [ ] `journalctl --disk-usage` reports the persistent journal, and
      `docker compose version` matches production's major version.
- [ ] `grep -iE 'password|token|secret' infra/bootstrap.sh` matches
      nothing sensitive.
- [ ] A deliberately broken bootstrap (for example `false` before the
      marker) leaves `.bootstrap-failed`. The next deploy fails at "Wait
      for bootstrap" within seconds and prints the log tail. It must not
      wait out the 10 minutes.

### Deploy plumbing (Groups 4–5)

- [ ] `grep -n 'latest' docker-compose.prod.yml` shows no `zainstreat-*`
      image tags.
- [ ] `API_IMAGE=x WEB_IMAGE=y docker compose -f docker-compose.yml -f docker-compose.prod.yml config`
      succeeds, and without them it fails with "API_IMAGE is required".
- [ ] `docker run --rm -v $PWD/Caddyfile:/etc/caddy/Caddyfile caddy:2-alpine caddy validate --config /etc/caddy/Caddyfile`
      passes, with `DOMAIN=example.com` and with `NOINDEX` both `true`
      and `false`.
- [ ] A PR runs `ci.yml` (api tests, web lint and build) and nothing
      else.
- [ ] A merge to `master` with staging down builds `zainstreat-api:<sha>`
      and `zainstreat-web:<sha>`, and does not deploy.
- [ ] A merge to `master` with staging up also builds
      `zainstreat-web-staging:<sha>` and deploys staging green.
- [ ] The rendered `.env` on the box is mode `600` and owned by `deploy`.
- [ ] The `_deploy` logs contain no secret values; GitHub masks them as
      `***`.
- [ ] `~deploy/.docker/config.json` on the box has no `ghcr.io` auth
      after a deploy (logout ran).
- [ ] A deploy whose `web` fails to start fails the workflow at the
      health-check step within about 3 minutes and prints logs.
- [ ] **Env parity, enforced:** change `POSTGRES_PASSWORD` in the
      `staging` Environment, then run a staging deploy with
      `enforce_env_parity: true`. Check that:
      - it fails and prints `POSTGRES_PASSWORD` with no value
      - the live `.env` on the box is unchanged
      - re-running with `allow_env_changes: POSTGRES_PASSWORD` passes the
        check

      Then restore the secret.
- [ ] An Environment value containing `'` fails rendering, naming the
      key and not the value.
- [ ] After a deploy, `~deploy/.ssh/authorized_keys` has exactly one
      `zainstreat-backup` line starting with `restrict,command=`, and the
      deploy key line is unchanged. Running the deploy twice doesn't
      duplicate it.
- [ ] `PULL_TOKEN` falls back correctly: with `GHCR_PULL_TOKEN` unset,
      the job token pulls (Decision 21).

### Staging lifecycle (Group 7)

- [ ] `staging-up` from nothing reaches green health checks with **no
      manual SSH**. The summary lists the `https://<ip>.sslip.io` URLs.
- [ ] `https://<ip-dashed>.sslip.io/en` serves the seeded menu with valid
      TLS (`curl -sv` shows a Let's Encrypt or ZeroSSL issuer).
- [ ] `https://cms.<ip-dashed>.sslip.io/login` lets the staging admin
      log in, and `https://logs.<ip-dashed>.sslip.io` asks for Basic
      Auth.
- [ ] `curl -sI https://<ip-dashed>.sslip.io/en | grep -i x-robots-tag`
      gives `noindex, nofollow`.
- [ ] Placing a test order on staging emails only the developer's
      address.
- [ ] Re-running `staging-up` while it's up redeploys and doesn't create
      a second instance.
- [ ] `staging-down` without the confirm phrase aborts.
- [ ] With `STAGING_DNS_SUFFIX=nip.io`, `staging-up` serves
      `https://<ip-dashed>.nip.io` with valid TLS. Switch it back
      afterwards.
- [ ] `staging-up` on a running instance redeploys in place (same IP).
      The summary's certificate issuer is unchanged, and Caddy's log
      shows no new certificate order.
- [ ] `staging-down` with the phrase deletes the instance, and
      `aws lightsail get-instance --instance-name zainstreat-staging`
      returns NotFound.

### Promote / rollback (Group 6)

- [ ] `promote` with a SHA that has no images fails in `verify` before
      the approval prompt.
- [ ] Before the `cms` DNS record exists, `promote` fails in `verify`
      with a DNS-gate message, before the approval prompt.
- [ ] `promote` with a good SHA waits for reviewer approval, then
      deploys.
- [ ] After that deploy, `docker inspect` on the box shows
      `zainstreat-web:<sha>`.
- [ ] `promote` with the previous SHA (rollback) succeeds; promoting the
      newer SHA afterwards succeeds too.

### `create_admin` (Group 8)

- [ ] `printf 'pw\n' | create_admin --email a@b.c --password-stdin`
      creates or updates the user without prompting, and `/api/admin/login`
      with `pw` succeeds.
- [ ] Without the flag, the interactive prompt behaves as before, and an
      empty stdin password is rejected.
- [ ] `cargo test` passes.
- [ ] `create-admin` on staging works.
- [ ] `create-admin` on production refuses to run when `ADMIN_PASSWORD`
      is unset.

### Backup / restore (Group 9)

- [ ] A manual `backup` on production writes non-empty
      `production/<ts>/db.dump` and `production/<ts>/minio.tgz` to S3.
- [ ] The nightly cron run succeeds without an approval prompt.
- [ ] `restore` of that production backup **into staging** works:
      staging shows production's menu and photos, and admin orders match.
- [ ] `restore` without the matching confirm phrase aborts.
- [ ] Backup key is locked down:
      `ssh -i backup_key deploy@<host> 'cat /opt/zainstreat/.env'`,
      `... id` and `... -t` (interactive) are all refused or print
      "refused". `... db | head -c 5` shows `PGDMP`.
- [ ] The nightly cron run's logs show only `BACKUP_SSH_KEY` in use;
      `production-backup` has no `DEPLOY_SSH_KEY` secret.

### Admin on `cms.` (Group 10, local)

- [ ] `grep -rn '"/admin' apps/web/app apps/web/components` finds only
      `robots.ts`.
- [ ] `http://cms.localhost:3000/` shows the admin login when logged
      out, and the dashboard after login.
- [ ] On `cms.localhost:3000`, the following all work: nav highlighting,
      orders, order detail, menu create/edit with a photo upload,
      categories, enquiries, logout.
- [ ] `curl -sI http://localhost:3000/admin/orders` gives `308` with
      `location: http://cms.localhost:3000/orders`.
- [ ] `curl -s -o /dev/null -w '%{http_code}' -X POST http://localhost:3000/api/admin/login`
      gives `404`.
- [ ] `/en`, `/nl`, the menu, cart and checkout on `localhost:3000`
      behave as before. Public `/api/*` handlers (orders, enquiries)
      still work.
- [ ] `/validate` passes (lint, build, smoke test).

### Phase 20 reconciliation (Group 13)

- [ ] Phase 20 is merged to `master`, and this branch is rebased onto it
      with `specs/roadmap.md` showing Phase 20's "Planned in …"
      paragraph and Phases 21–29 in order.
- [ ] `grep -n "Phase 2[1-5]" specs/2026-10-25-phase-20-bank-transfer-payment/requirement.md`:
      every hit uses the new numbers (tracking 22, catering 23, French
      24, admin content 25).
- [ ] `grep -rn '/admin/' apps/api/src | grep -v '/api/admin'` gives 0
      hits.
- [ ] A local test order emails the owner a link to
      `http://cms.localhost:3000/orders/<id>`, and so does an "I've
      paid" claim.
- [ ] On staging, the owner email links to `https://cms.<ip-dashed>.sslip.io/orders/<id>`.
- [ ] The `BANK_*` values on the staging invoice read "STAGING – do not
      pay".

### Production cutover (Groups 11–12)

- [ ] `dig +short cms.zainstreat.com` gives `63.185.172.26`.
- [ ] The owner got the new address before the cutover, and the cutover
      ran at a quiet time with no open orders.
- [ ] The cutover `backup` (12.3) succeeded before the first `promote`,
      using the backup path 2.1 pre-installed.
- [ ] The first production `promote`'s parity report listed only
      non-credential keys (`ACME_EMAIL`, `COOKIE_SECURE`, Phase 20
      additions). No `POSTGRES_*`, `MINIO_*`, `SES_*` or `DOZZLE_*` key
      was allow-listed.
- [ ] An owner email sent before the cutover
      (`https://zainstreat.com/admin/orders/<id>`) lands on that order at
      `cms.zainstreat.com` after login.
- [ ] A backup taken right before cutover is in S3.
- [ ] `deploy.yml` is deleted.
- [ ] The first `promote` deploy is green, and the API connects to
      Postgres (orders are visible in admin).
- [ ] `https://cms.zainstreat.com/login` serves the admin with valid TLS
      and `X-Robots-Tag: noindex, nofollow`.
- [ ] `https://zainstreat.com/admin` 308s to `https://cms.zainstreat.com/`.
- [ ] The owner can log in at `cms.zainstreat.com` and see orders.
- [ ] The public site's response headers are unchanged: no
      `X-Robots-Tag` on `zainstreat.com`.
- [ ] `/opt/zainstreat` has no `.git`; the GHCR PAT is revoked.
- [ ] `docs/deployment.md` describes only workflows for routine tasks,
      and its "emergency-only SSH" section is the only place SSH appears
      as a procedure.
- [ ] Manual SSH sessions used after Group 2: **0**, apart from the one
      12.5 tidy-up session if 2.1's session couldn't do it.

## Definition of done

Merging to `master` builds SHA-tagged images and, when staging is up,
deploys them to an ephemeral Lightsail staging box on `<ip>.sslip.io` that
anyone on the team can create or delete from GitHub Actions without
touching a server. Production changes only when someone promotes an
already-built SHA and a reviewer approves it, and rolling back is the same
button with an older SHA. The server's `.env`, admin accounts, backups to
S3 and restores are all handled by workflows, so routine operations need
no SSH. The admin lives at `cms.zainstreat.com` (and `cms.<ip>.sslip.io`
on staging), old `/admin` links redirect there, and the public site is
unchanged.
