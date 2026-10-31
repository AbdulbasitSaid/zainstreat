# Phase 21 — Pipeline Refactor: Staging, No-SSH Ops, `cms.` Admin: Validation

Checked on 2026-10-31 unless noted. Where the implementation deviated
from `plan.md`, see its "Implementation notes" section.

## Pass/fail checklist

### AWS (Group 1)

- [x] IAM lists the OIDC provider `token.actions.githubusercontent.com`.
- [x] The trust policy of `zainstreat-github-actions` limits `sub` to
      this repo. It uses GitHub's **immutable** subject,
      `repo:AbdulbasitSaid@16627656/zainstreat@1400619975:*`, because this
      repo's OIDC customization has `use_immutable_subject: true`. The
      `repo:AbdulbasitSaid/zainstreat:*` form originally planned was
      rejected by STS.
- [x] Workflows assume the role: `build.yml`'s `staging-check` and
      `staging-up.yml`'s `provision` both succeed. No `AWS_ACCESS_KEY_ID`
      secret exists in the repo.
- [x] Deleting or re-firewalling an instance without `env=staging` is
      denied. Proven with the IAM policy simulator rather than a throwaway
      instance:
      - `DeleteInstance` and `PutInstancePublicPorts`: `allowed` with
        `env=staging`, `implicitDeny` without it or with `env=production`
      - `StopInstance` and `DeleteStaticIp`: `implicitDeny`
- [x] The backup bucket has Block Public Access on (all four flags), SSE,
      and a 30-day expiry rule.

### GitHub (Group 2)

- [x] `gh api` shows `zainstreat-api` and `zainstreat-web` linked to
      `AbdulbasitSaid/zainstreat`.
- [x] The `gh api` check reports `private` for `zainstreat-api`,
      `zainstreat-web` and `zainstreat-web-staging`. The staging package
      was created public by its first push and was then made private.
- [x] Logged out, `docker manifest inspect` on each package is denied.
- [x] The Environments `staging`, `production` and `production-backup`
      exist.
- [x] `production` has a required reviewer, and `production` and
      `production-backup` only accept deployments from `master`.
- [x] Every name in `.env.example` that the stack needs exists in
      `production` and `staging`. Exceptions, both deliberate:
      - `SES_*` and `EMAIL_*` aren't set on either Environment, because
        the production server never had them (SES go-live is still the
        Phase 18 item) and staging must not send email. Email is a
        logged no-op on both.
- [x] The `production` values match the old server `.env`. The first
      Promote's parity report listed only `ACME_EMAIL`.

### Bootstrap (Group 3)

- [x] A fresh staging box became ready and was deployed with no manual
      SSH. `_deploy`'s wait step saw `.bootstrap-done`, and the `deploy`
      user logged in.
- [ ] On the staging box: `swapon --show` shows a 2G `/swapfile`, and
      `sysctl vm.swappiness` gives 10. *(Not inspected; needs SSH to
      staging.)*
- [ ] `journalctl --disk-usage` reports the persistent journal.
      *(Not inspected.)*
- [x] `grep -iE 'password|token|secret' infra/bootstrap.sh` matches
      nothing sensitive.
- [ ] A deliberately broken bootstrap leaves `.bootstrap-failed`, and the
      next deploy fails within seconds and prints the log tail. *(Not
      run.)*

### Deploy plumbing (Groups 4–5)

- [x] `grep -n 'latest' docker-compose.prod.yml` shows no `zainstreat-*`
      image tags.
- [x] `API_IMAGE=a WEB_IMAGE=b ACME_EMAIL=x docker compose … config`
      succeeds, and without them it fails with "… is required".
- [x] `caddy validate` passes with `NOINDEX` both `true` and `false`. The
      header is on 4 sites with `true`, and on `cms.` only with `false`.
- [x] A PR runs only `ci.yml` (api tests, web lint and build). Checked on
      PR #32.
- [x] A merge to `master` with staging down builds `zainstreat-api:<sha>`
      and `zainstreat-web:<sha>`, and doesn't deploy. Checked on
      `7d00381`, after the trust fix.
- [ ] A merge to `master` with staging up also builds
      `zainstreat-web-staging:<sha>` and deploys staging green. *(Next
      merge while staging is up.)*
- [ ] The rendered `.env` on the box is mode `600` and owned by `deploy`.
      *(`_deploy` sets 600; not inspected on the box.)*
- [x] The `_deploy` logs contain no secret values. Values reach the
      renderer only via `env:` and are masked, and the parity check
      prints key names only.
- [ ] `~deploy/.docker/config.json` has no `ghcr.io` auth after a deploy.
      *(The logout trap runs; not inspected on the box.)*
- [ ] A deploy whose `web` fails to start fails at the health check and
      prints logs. *(Not run.)*
- [x] **Env parity, enforced.** Tested locally against fixture files
      with `infra/env-parity.sh`:
      - a changed `POSTGRES_PASSWORD` fails, listing
        `postgres.POSTGRES_PASSWORD` and `api.DATABASE_URL` with no values
      - it passes once both are allow-listed
      - `$$`-escaped and single-quoted forms of the same value count as
        equal
      - an unreadable `.env` aborts instead of reporting every key
- [ ] An Environment value containing `'` fails rendering, naming the
      key. *(Code path exists; not run.)*
- [x] The backup key line is installed and usable: the production and
      staging backups succeeded over the backup key.
- [x] `PULL_TOKEN` falls back correctly: with `GHCR_PULL_TOKEN` unset,
      the job token pulled private images on both production and staging.

### Staging lifecycle (Group 7)

- [x] `staging-up` from nothing reaches green health checks with no
      manual SSH: `https://51-102-179-66.sslip.io`.
- [x] `/en` serves the 28 seeded menu items, with a Let's Encrypt (YE2)
      certificate.
- [x] The staging admin logs in at `https://cms.<ip-dashed>.sslip.io`
      and opens `/orders`, and `https://logs.…` returns 401 without auth.
- [x] `X-Robots-Tag: noindex, nofollow` is sent on staging.
- [x] Staging can't email the business: it has no SES credentials, so
      every send is a logged no-op.
- [x] Re-running `staging-up` while it's up redeploys in place: same
      instance and IP, parity reports "no changes".
- [x] `staging-up` on a running instance keeps the certificate (same
      serial before and after).
- [ ] `staging-down` without the confirm phrase aborts. *(Not run;
      staging kept up for client review.)*
- [ ] With `STAGING_DNS_SUFFIX=nip.io`, `staging-up` serves valid TLS.
      *(Not run.)*
- [ ] `staging-down` with the phrase deletes the instance. *(Not run;
      staging kept up for client review.)*

### Promote / rollback (Group 6)

- [x] `promote` with a SHA that has no images fails in `verify` before
      the approval prompt ("…:0000… doesn't exist").
- [ ] Before the `cms` DNS record exists, `promote` fails the DNS gate.
      *(Can't be shown now: the record existed before `promote` did.)*
- [x] `promote` with a good SHA (`7d00381`) waited for reviewer approval,
      then deployed. GitHub's latest `production` deployment is
      `7d00381`.
- [x] The deploy is serving that build. Health checks passed, and
      `cms.zainstreat.com` and the `/admin` redirect come from the new web
      image.
- [ ] `promote` with the previous SHA (rollback), then forward again.
      *(Needs a second Build-produced SHA. Do this on the next merge.)*

### `create_admin` (Group 8)

- [x] `printf 'pw\n' | create_admin --email … --password-stdin` creates
      the user without prompting, and `/api/admin/login` returns 200.
- [x] An empty stdin password is rejected ("password must not be
      empty"). The interactive path is unchanged.
- [x] `cargo test` passes (CI `test-api` on PR #32 and on `master`).
- [x] `create-admin` on staging works: `workflow-test@staging.local`
      logs in.
- [ ] `create-admin` on production refuses to run when `ADMIN_PASSWORD`
      is unset. *(Not run; the check is the workflow's first step.)*

### Backup / restore (Group 9)

- [x] A manual production `backup` wrote `production/20261031T121819Z/`
      to S3 (30.5 KiB `db.dump`, 10.3 MiB `minio.tgz`).
- [ ] The nightly cron run (02:30 UTC) succeeds without an approval
      prompt. *(First run due 2026-11-01.)*
- [x] `restore` works, tested **staging → staging** instead of
      production → staging, so customer data stays off staging:
      - an admin created after the backup was gone after the restore
        (login 401)
      - the original admin and all 28 menu items were back
- [x] `restore` without the matching confirm phrase aborts before
      touching anything.
- [ ] Backup key lockdown (`cat .env`, `id` or an interactive session
      refused). *(The forced command is installed; the private key exists
      only in GitHub, so it wasn't probed by hand.)*
- [x] `production-backup` has only `BACKUP_SSH_KEY`, with no
      `DEPLOY_SSH_KEY` secret.

### Admin on `cms.` (Group 10, local)

- [x] `grep -rn '"/admin' apps/web/app apps/web/components` finds only
      `robots.ts`.
- [x] `http://cms.localhost:3000/` redirects to `/login` when logged
      out, and shows the dashboard after login.
- [x] On `cms.localhost:3000`, logged in: the dashboard, `/orders`,
      `/menu`, `/menu/new`, `/categories` and `/enquiries` all return 200
      with the right nav item active and no `/admin` links.
- [ ] Clicking through a photo upload, cart and checkout in a browser.
      *(Manual; not checked by `/validate`.)*
- [x] `/admin/orders` gives `308` to `http://cms.localhost:3000/orders`.
- [x] Public `POST /api/admin/login` gives `404`.
- [x] `/en`, `/nl` (correct `<html lang>`) and `/en/menu` behave as
      before.
- [x] `/validate` passes: lint, build, `cargo check`/clippy/build, smoke
      test.

### Phase 20 reconciliation (Group 13)

Not started. Phase 21 merged to `master` **before** Phase 20, the
reverse of Decision 19, so this group now runs on the Phase 20 branch
when it's rebased. See `plan.md`'s implementation notes.

- [ ] Phase 20 rebased onto `master`; `specs/roadmap.md` shows Phase 20's
      paragraph and Phases 21–29 in order.
- [ ] Phase 20's `requirement.md` uses the new phase numbers.
- [ ] `grep -rn '/admin/' apps/api/src | grep -v '/api/admin'` gives 0
      hits (owner-email links use `PUBLIC_ADMIN_URL`).
- [ ] A local test order and an "I've paid" claim email the owner a
      `cms.` link.
- [ ] `BANK_*`/`BUSINESS_KVK` are rendered by `_deploy.yml` and set on
      both Environments; staging's invoice reads "STAGING – do not pay".

### Production cutover (Groups 11–12)

- [x] `dig +short cms.zainstreat.com` gives `63.185.172.26`.
- [ ] The owner was told about the new admin address beforehand.
      *(Yours to confirm.)*
- [x] Safety net before the first `promote`: Lightsail snapshots
      `zainstreat-pre-phase21` and `zainstreat-resize-4gb`. These replace
      the pre-installed-backup-path plan in 2.1 and 12.3.
- [x] The first production `promote`'s parity report listed only
      `ACME_EMAIL`. No credential key was allow-listed.
- [x] Old admin links keep working:
      `https://zainstreat.com/admin/orders/42?x=1` 308s to
      `https://cms.zainstreat.com/orders/42?x=1`.
- [x] A backup taken right after cutover is in S3
      (`production/20261031T121819Z/`).
- [x] `deploy.yml` is deleted.
- [x] The first `promote` deploy is green, and the API reads Postgres
      (28 menu items) and MinIO (uploaded photo 200).
- [x] `https://cms.zainstreat.com/login` serves the admin with valid TLS
      (Let's Encrypt, until 2027-01-29) and `X-Robots-Tag: noindex,
      nofollow`.
- [x] `https://zainstreat.com/admin` 308s to `https://cms.zainstreat.com/`.
- [ ] The owner can log in at `cms.zainstreat.com` and see orders.
      *(Yours to confirm.)*
- [x] The public site has no `X-Robots-Tag`.
- [ ] `/opt/zainstreat` has no `.git`; the old GHCR PAT and old deploy
      key are removed. *(Needs production SSH, which auto mode blocks for
      Claude: yours.)*
- [x] `docs/deployment.md` describes only workflows for routine tasks;
      SSH appears only under "Emergency-only SSH".
- [x] Manual SSH sessions after Group 2: **1**, the user-run
      `prod-cutover-prep.sh`. It used short-lived Lightsail access to
      import `.env` and rotate the deploy key.
- [x] Old repo-level `DEPLOY_HOST`/`DEPLOY_USER`/`DEPLOY_SSH_KEY`
      secrets are deleted.
- [ ] The old server's IPv6 AAAA record for `zainstreat.com` is removed
      at the DNS provider. *(Still points at the deleted instance: yours.)*

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
