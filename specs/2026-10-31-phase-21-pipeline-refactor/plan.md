# Phase 21 — Pipeline Refactor: Staging, No-SSH Ops, `cms.` Admin: Implementation Plan

**Two kinds of group:**
- **Ops groups** (1, 2, 12) happen in the AWS console or CLI, GitHub
  settings, and the DNS provider.
- **Repo groups** (3–11, 13) change the repo.

**Ordering:**
- **Phase 20 merges to `master` first.** This branch is rebased onto it,
  and Group 13 reconciles the overlap (Decision 19) before any repo group
  is implemented.
- Groups 1–2 must exist before any new workflow runs.
- Production cutover is Group 12. Until then the old `deploy.yml` keeps
  shipping production, and it is deleted in the same push that enables
  `promote` (Group 6).

Run `/validate` after Groups 10–11 locally, and `cargo test` after
Group 8.

## Group 0 — Branch

Branched `2026-10-31-phase-21-pipeline-refactor` off `master` (Phase 19
merged as `57a3a2a`).

## Group 1 — AWS: OIDC role and backup bucket (ops)

Depends on: nothing.

1.1. **Add the GitHub OIDC provider in IAM** (once per account):
     `token.actions.githubusercontent.com`, audience `sts.amazonaws.com`.

1.2. **Create the role `zainstreat-github-actions`.** Its trust policy
     allows only this repo:

```json
{
  "Effect": "Allow",
  "Principal": { "Federated": "arn:aws:iam::<ACCOUNT>:oidc-provider/token.actions.githubusercontent.com" },
  "Action": "sts:AssumeRoleWithWebIdentity",
  "Condition": {
    "StringEquals": { "token.actions.githubusercontent.com:aud": "sts.amazonaws.com" },
    "StringLike":   { "token.actions.githubusercontent.com:sub": "repo:AbdulbasitSaid/zainstreat:*" }
  }
}
```

1.3. **Give the role this permissions policy:**
     - **Lightsail, unrestricted:** `CreateInstances`, `GetInstance`,
       `GetInstances`, `GetInstanceState`, `GetBundles`, `TagResource`.
     - **Lightsail, staging only:** `DeleteInstance` and
       `PutInstancePublicPorts`, with the condition
       `"aws:ResourceTag/env": "staging"`, so the role can't change
       production's firewall or delete it.
     - **S3:** `PutObject`, `GetObject` and `ListBucket` on the backup
       bucket only.

1.4. **Create the S3 bucket** `zainstreat-backups-<ACCOUNT>` in
     `eu-central-1`:
     - Block Public Access on, SSE-S3, versioning off.
     - A lifecycle rule that expires objects after 30 days.

1.5. Record the role ARN and bucket name for Group 2.

## Group 2 — GitHub Environments, secrets and variables (ops)

Depends on: Group 1.

2.1. **Last planned admin SSH session.** It does three things:
     - **Capture the current production `.env`:**
       `sudo cat /opt/zainstreat/.env`. Copy each value into the
       `production` Environment as the value compose actually resolves.
       For example, a `$$` in the file is a single `$` in the secret,
       because the renderer single-quotes values (Decision 14). The env
       parity preflight then proves the copy is exact on the first
       promote.
     - **Pre-install the backup path,** so the cutover backup in 12.2 can
       run before the first new-style deploy:
       - copy `infra/backup-stream.sh` to `/opt/zainstreat/bin/backup-stream`
         (mode `755`, owner `deploy`)
       - append the `restrict,command=...` line from Group 4.4 step 5 to
         `~deploy/.ssh/authorized_keys`

       From then on, `_deploy` maintains both.
     - **Remove the old git checkout:** delete `/opt/zainstreat/.git`
       and other clone leftovers, keeping `.env`, the compose files,
       `Caddyfile` and `backups/`. This ends the drift between the box
       and git, and is done here while sudo is at hand.

2.2. **Create three Environments:**

| Environment | Protection | Used by |
|---|---|---|
| `staging` | none | build → staging deploy, staging-up/down, create-admin, restore |
| `production` | required reviewer (you), deployments from `master` only | promote, create-admin, restore, seed-production |
| `production-backup` | deployments from `master` only | backup (cron) |

2.3. **Repository variables:** `AWS_ROLE_ARN`, `AWS_REGION=eu-central-1`,
     `BACKUP_BUCKET`, `IMAGE_REPO=ghcr.io/abdulbasitsaid`,
     `PROD_DEPLOY_HOST=63.185.172.26` (for promote's pre-approval DNS gate),
     `STAGING_DNS_SUFFIX=sslip.io` (switch to `nip.io` if sslip.io is down;
     Decision 16). `DOMAIN`
     (`zainstreat.com`) stays a repository variable: `build.yml` bakes it
     into the production web image without entering the reviewer-gated
     Environment.

2.4. **Per-Environment secrets and variables.** The names match
     `.env.example` (Group 4.3).

| Name | Kind | production | staging |
|---|---|---|---|
| `DEPLOY_HOST` | var | `63.185.172.26` | — (looked up at run time) |
| `DEPLOY_SSH_KEY` | secret | existing deploy key | new keypair |
| `DEPLOY_SSH_PUBLIC_KEY` | var | — | public half (fed to bootstrap) |
| `DOMAIN` | repo var (unchanged) | `zainstreat.com` | — (derived from the IP) |
| `POSTGRES_USER` / `POSTGRES_DB` | var | from 2.1 | `zainstreat` |
| `POSTGRES_PASSWORD` | secret | from 2.1 | random |
| `MINIO_ROOT_USER` | var | from 2.1 | `zainstreat` |
| `MINIO_ROOT_PASSWORD` | secret | from 2.1 | random |
| `SES_SMTP_HOST` / `EMAIL_FROM_ADDRESS` | var | from 2.1 | same |
| `SES_SMTP_USERNAME` / `SES_SMTP_PASSWORD` | secret | from 2.1 | same |
| `EMAIL_BUSINESS_NOTIFY_ADDRESS` | var | from 2.1 | developer's address |
| `RUST_LOG` / `LOG_FORMAT` | var | from 2.1 | `debug` / `json` |
| `DOZZLE_BASIC_AUTH_USER` | var | from 2.1 | `admin` |
| `DOZZLE_BASIC_AUTH_HASH` | secret | from 2.1 | new hash |
| `NOINDEX` | var | — | `true` |
| `ACME_EMAIL` | var | developer's address | same |
| `BACKUP_SSH_PUBLIC_KEY` | var | public half of the backup key | same (separate keypair) |
| `BANK_ACCOUNT_NAME` / `BANK_IBAN` / `BANK_BIC` / `BUSINESS_KVK` (Phase 20) | var | the business's real details | `STAGING – do not pay`, a checksum-valid test IBAN (e.g. `NL91ABNA0417164300`), empty, empty |
| `GHCR_PULL_TOKEN` | secret | only if 4.5 fails | only if 4.5 fails |
| `ADMIN_EMAIL` / `ADMIN_NAME` | var | — | staging admin |
| `ADMIN_PASSWORD` | secret | set only while running `create-admin` | staging admin |

`PUBLIC_SITE_URL` and `PUBLIC_ADMIN_URL` are not set per Environment:
prod compose derives them from `DOMAIN` (`https://${DOMAIN}`,
`https://cms.${DOMAIN}`).

`production-backup` gets only `DEPLOY_HOST`, `BACKUP_SSH_KEY` (the
private half of the backup key; **not** `DEPLOY_SSH_KEY`), `POSTGRES_USER`
and `POSTGRES_DB`. The `staging` Environment gets its own `BACKUP_SSH_KEY`
too.

2.5. **GHCR packages are linked and private** (Decision 21).
     - **Linking was checked on 2026-10-31:** both packages report
       `repo=AbdulbasitSaid/zainstreat`. The same check showed both were
       **public**, contradicting `tech-stack.md`'s "private GHCR".
     - **Make both private:** github.com → your profile → Packages →
       `zainstreat-api` → Package settings → Danger Zone → Change
       visibility → Private. Repeat for `zainstreat-web`.
     - **Do it while `deploy.yml` is still live.** The prod box's stored
       PAT login keeps pulls working, so the old deploy isn't affected.
     - **Re-check:**

```bash
for p in zainstreat-api zainstreat-web; do
  gh api /users/AbdulbasitSaid/packages/container/$p --jq '"\(.name): \(.visibility), repo=\(.repository.full_name)"'
done
# Expect: private, repo=AbdulbasitSaid/zainstreat for both.
```

     - **The staging web image:** `zainstreat-web-staging` is created by
       the first staging build (Group 5/7). Run the same check on it
       afterwards. If it came out public, make it private too.
     - **If job-token pulls fail later** (4.5), confirm each package's
       settings → **Manage Actions access** lists `zainstreat` with Read.
       If it doesn't, create `GHCR_PULL_TOKEN` (`read:packages` only).

## Group 3 — `infra/bootstrap.sh` (repo)

Depends on: nothing.

3.1. **Create `infra/bootstrap.sh`.** It is a template: workflows replace
     `__DEPLOY_SSH_PUBLIC_KEY__` before passing it as `--user-data`.

```bash
#!/bin/bash
# Lightsail launch script (Amazon Linux 2023). Reproduces the Phase 2/18
# manual setup. Idempotent; contains no secrets.
set -euxo pipefail
# Decision 18: a readable log and a failure marker the deploy can see.
mkdir -p /opt/zainstreat
exec > >(tee -a /var/log/zainstreat-bootstrap.log) 2>&1
trap 'echo "bootstrap failed at line $LINENO" > /opt/zainstreat/.bootstrap-failed' ERR

dnf update -y
dnf install -y dnf-plugins-core
dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo
# AL2023 isn't Fedora: pin $releasever to a Fedora release Docker publishes for,
# or use the AL2023 `docker` package + the compose plugin binary — decide during
# implementation and match what Phase 2 installed (`docker compose version`).
dnf install -y docker
mkdir -p /usr/local/lib/docker/cli-plugins
curl -fsSL "https://github.com/docker/compose/releases/download/v2.40.0/docker-compose-linux-$(uname -m)" \
  -o /usr/local/lib/docker/cli-plugins/docker-compose
chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
systemctl enable --now docker

if [ ! -f /swapfile ]; then
  fallocate -l 2G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi
echo 'vm.swappiness=10' > /etc/sysctl.d/99-swappiness.conf && sysctl --system

mkdir -p /etc/systemd/journald.conf.d
printf '[Journal]\nStorage=persistent\nSystemMaxUse=200M\n' > /etc/systemd/journald.conf.d/10-persistent.conf
systemctl restart systemd-journald

id deploy >/dev/null 2>&1 || useradd -m -G docker deploy
install -d -m 700 -o deploy -g deploy /home/deploy/.ssh
echo '__DEPLOY_SSH_PUBLIC_KEY__' > /home/deploy/.ssh/authorized_keys
chown deploy:deploy /home/deploy/.ssh/authorized_keys && chmod 600 /home/deploy/.ssh/authorized_keys

install -d -m 750 -o deploy -g deploy /opt/zainstreat /opt/zainstreat/bin
chown deploy:deploy /var/log/zainstreat-bootstrap.log
rm -f /opt/zainstreat/.bootstrap-failed
touch /opt/zainstreat/.bootstrap-done && chown deploy:deploy /opt/zainstreat/.bootstrap-done
```

3.2. **Create `infra/backup-stream.sh`** (Decision 15). It is the forced
     command for the backup key, so it's the only thing that key can run:

```bash
#!/bin/bash
# Forced command for the backup SSH key (authorized_keys `command=`).
# Streams one backup to stdout; refuses everything else.
set -euo pipefail
cd /opt/zainstreat
C="docker compose -f docker-compose.yml -f docker-compose.prod.yml"
case "${SSH_ORIGINAL_COMMAND:-}" in
  # The postgres container already has POSTGRES_USER/DB in its env, so
  # the host never has to parse .env (the hand-written one isn't quoted).
  db)    exec $C exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" -Fc "$POSTGRES_DB"' ;;
  minio) exec $C exec -T minio tar -C /data -czf - . ;;
  *)     echo "refused: only 'db' or 'minio'" >&2; exit 1 ;;
esac
```

3.3. **Check the bootstrap on a throwaway instance first.** Run it on a Lightsail
     instance created by hand (or the first `staging-up`), then compare
     `docker compose version`, `free -m`, `swapon --show` and
     `journalctl --disk-usage` with production. Then break the script on
     purpose (for example `false` before the marker), create another
     instance, and confirm `.bootstrap-failed` appears and the log
     explains why.

## Group 4 — Compose image refs and the reusable deploy (repo)

Depends on: Groups 2, 3.

4.1. **`docker-compose.prod.yml`:**
     - Set `api.image: ${API_IMAGE:?API_IMAGE is required}` and
       `web.image: ${WEB_IMAGE:?WEB_IMAGE is required}`.
     - Remove the hard-coded `ghcr.io/abdulbasitsaid/...:latest`.

4.2. **`Caddyfile`:**
     - In the global options block, add `email {$ACME_EMAIL}`
       (Decision 17). This enables Caddy's ZeroSSL fallback when Let's
       Encrypt rate-limits.
     - Add a `cms.{$DOMAIN}` block → `reverse_proxy web:3000` with
       `header X-Robots-Tag "noindex, nofollow"`.
     - For staging, add `header X-Robots-Tag "noindex, nofollow"` to every
       site when `NOINDEX=true`. Use a snippet selected by
       `import noindex-{$NOINDEX:false}`, with two snippets,
       `(noindex-true)` (sets the header) and `(noindex-false)` (empty).
       Keep the existing JSON `log` blocks.

4.3. **`.env.example`:** list every name from Group 2.4 that the stack
     reads, plus `API_IMAGE`, `WEB_IMAGE`, `NOINDEX` and `ACME_EMAIL`. Add a header comment: *"On servers this
     file is generated by `.github/workflows/_deploy.yml`; never edit it
     there."*

4.4. **New `.github/workflows/_deploy.yml`** (`on: workflow_call`):
     - **Inputs:** `environment`, `host` (optional; empty = use the
       `DEPLOY_HOST` variable), `domain` (optional; empty = the `DOMAIN`
       variable), `api_image`, `web_image`, `allow_env_changes`
       (optional, comma-separated key names; Decision 14), and
       `enforce_env_parity` (boolean, default
       `${{ inputs.environment == 'production' }}`).
     - `environment: ${{ inputs.environment }}`.
     - `concurrency: { group: deploy-${{ inputs.environment }}, cancel-in-progress: false }`.
     - `permissions: { contents: read, packages: read }`.

     Steps:
     1. **Set up SSH.** Write `DEPLOY_SSH_KEY` to `~/.ssh/id_ed25519`, then
        `ssh-keyscan $HOST >> ~/.ssh/known_hosts`. Use plain OpenSSH/scp
        from here on; the `appleboy/*` actions are dropped.
     2. **Wait for bootstrap** (Decision 18). Poll every 10 s, for up
        to 10 minutes:
        - `.bootstrap-done` exists → continue.
        - `.bootstrap-failed` exists → fail.
        - Timeout → fail.

        On either failure, print the marker and
        `tail -n 100 /var/log/zainstreat-bootstrap.log` over SSH.
     3. **Render `.env.next`** (Decision 14).
        - Run `umask 077` and write one `KEY='value'` line per name in
          4.3 from `secrets.*`/`vars.*`. Values are passed in via `env:`,
          never interpolated into the script.
        - Then set `API_IMAGE`, `WEB_IMAGE` and `DOMAIN` from the inputs,
          and `COOKIE_SECURE='true'`.
        - Before writing anything, fail if any value contains `'` or a
          newline. Print the key name only.
     4. **Upload:** `scp docker-compose.yml docker-compose.prod.yml Caddyfile .env.next infra/backup-stream.sh deploy@$HOST:/opt/zainstreat/`.
        Then move `backup-stream.sh` into `bin/` with mode `755` and set
        `.env.next` to mode `600`.
     5. **Authorize the backup key** (Decision 15). Idempotently replace
        the line tagged `zainstreat-backup` in
        `~deploy/.ssh/authorized_keys` with:
        `restrict,command="/opt/zainstreat/bin/backup-stream" <BACKUP_SSH_PUBLIC_KEY> zainstreat-backup`.
        Leave every other line untouched.
     6. **Env parity preflight** (Decision 14). If `.env` exists on the
        box, run `infra/env-parity.sh` (new, shipped like
        `backup-stream.sh`) there:

```bash
# Prints only the KEY names whose resolved values differ; never values.
hashes() {
  docker compose --env-file "$1" -f docker-compose.yml -f docker-compose.prod.yml config --format json \
    | jq -r '.services | to_entries[] | .key as $s | (.value.environment // {}) | to_entries[]
             | "\($s).\(.key) \(.value | tostring)"' \
    | while read -r k v; do printf '%s %s\n' "$k" "$(printf '%s' "$v" | sha256sum | cut -c1-16)"; done | sort
}
diff <(hashes .env) <(hashes .env.next) | awk '/^[<>]/ {print $2}' | sed 's/^[^.]*\.//' | sort -u \
  | grep -vxE 'API_IMAGE|WEB_IMAGE' || true
```

        The `API_IMAGE`/`WEB_IMAGE` keys (which change every deploy,
        and reach `caddy` via `env_file`) are excluded.
        - **`enforce_env_parity` true** (production by default): fail if
          any reported key is not in `allow_env_changes`.
        - **False** (staging by default): report the keys and continue.
        - **No `.env` yet** (a fresh box): skip the check.

        Then run `mv .env.next .env`.
     7. **Deploy:**
        ```bash
        ssh deploy@$HOST bash -s <<EOF
        set -euo pipefail
        cd /opt/zainstreat
        echo "$PULL_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin   # GHCR_PULL_TOKEN if set, else GITHUB_TOKEN
        C="docker compose -f docker-compose.yml -f docker-compose.prod.yml"
        \$C pull && \$C up -d --remove-orphans && \$C restart caddy
        docker logout ghcr.io
        docker image prune -f
        EOF
        ```
     8. **Health check:** retry for up to 3 minutes until all three return
        2xx/3xx: `https://api.$DOMAIN/health`, `https://$DOMAIN/en` and
        `https://cms.$DOMAIN/login`. Otherwise fail, and print
        `docker compose ps` plus the last 100 log lines of `api` and
        `web`.
     9. **Record the deployment:** write the SHA to
        `$GITHUB_STEP_SUMMARY`. The `environment:` key already makes
        GitHub record the deployment.

4.5. **Confirm job-token pulls work** (Decision 21; pre-checked in 2.5).
     The first staging deploy must pull with `GITHUB_TOKEN`. If it gets
     `denied`, set `GHCR_PULL_TOKEN` in the Environment; step 7 already
     prefers it.

## Group 5 — `ci.yml` + `build.yml`; retire `deploy.yml` (repo)

Depends on: Group 4.

5.1. **`ci.yml`** (on `pull_request` and `push` to `master`):
     - Move the `test-api` job from `deploy.yml` unchanged.
     - Add `web-check`: `pnpm install --frozen-lockfile`, `pnpm lint`,
       `pnpm build` in `apps/web`.

5.2. **`build.yml`:**
     - **Trigger:** `workflow_run` of `ci.yml` (completed, success,
       branch `master`), using its head SHA.
     - **`build`:**
       - Build and push `zainstreat-api:<sha>`, plus
         `zainstreat-web:<sha>` with `DOMAIN=${{ vars.DOMAIN }}` (the
         repository variable). The job has no `environment:`, so the
         reviewer gate isn't triggered.
       - Add `cache-from/cache-to: type=gha`.
     - **`staging-check`:** assume the AWS role and run
       `aws lightsail get-instance --instance-name zainstreat-staging`.
       Output `up=true|false` and `ip`.
     - **`staging-web`** (if up): build and push
       `zainstreat-web-staging:<sha>` with
       `DOMAIN=${ip//./-}.${STAGING_DNS_SUFFIX}`.
     - **`deploy-staging`** (if up): `uses: ./.github/workflows/_deploy.yml`
       with `environment: staging`, `host: <ip>`, `domain: <ip-dashed>.$STAGING_DNS_SUFFIX`
       and both image refs.

5.3. **Keep `deploy.yml` unchanged until Group 12.** Production keeps
     deploying the old way until the cutover push. Tests run twice on
     `master` in the meantime, which is harmless.

## Group 6 — `promote.yml` / rollback (repo)

Depends on: Group 4.

6.1. **`workflow_dispatch`:**
     - **Input:** `sha` (required). The description says: *"Commit SHA to
       deploy. For rollback, use an earlier SHA from the Deployments
       page."*
     - **Job 1 `verify`** (no `environment:`, so it runs before
       approval):
       - Check that `docker manifest inspect` succeeds for
         `$IMAGE_REPO/zainstreat-api:<sha>` and
         `$IMAGE_REPO/zainstreat-web:<sha>`, and fail otherwise. Promote
         never builds.
       - **DNS gate** (Decision 22): fail unless
         `dig +short cms.$DOMAIN @1.1.1.1` contains `vars.PROD_DEPLOY_HOST`.
         That is a repository-variable mirror of the `production`
         Environment's `DEPLOY_HOST`, since `verify` can't read
         Environment variables.
     - **Job 2:** `uses: ./.github/workflows/_deploy.yml` with
       `environment: production` (this is where the reviewer approves)
       and both image refs.

6.2. **Cutover:** this file lands in the same push as the deletion of
     `deploy.yml` (Group 12.4).

## Group 7 — `staging-up.yml` / `staging-down.yml` (repo)

Depends on: Groups 3–5.

7.1. **`staging-up.yml`** (`workflow_dispatch`, input `sha`, default
     master HEAD). The `provision` job (assumes the role,
     `environment: staging`) does:
     1. **If `zainstreat-staging` already exists,** skip to step 4. Redeploying
        in place keeps `caddy_data` and its certificates (Decision 17).
        Otherwise create it:
        ```bash
        sed "s|__DEPLOY_SSH_PUBLIC_KEY__|$DEPLOY_SSH_PUBLIC_KEY|" infra/bootstrap.sh > user-data.sh
        aws lightsail create-instances --instance-names zainstreat-staging \
          --availability-zone eu-central-1a --blueprint-id amazon_linux_2023 \
          --bundle-id "$(aws lightsail get-bundles --query "bundles[?ramSizeInGb==\`2.0\` && isActive && contains(supportedPlatforms,'LINUX_UNIX')].bundleId | [0]" --output text)" \
          --user-data file://user-data.sh --tags key=env,value=staging
        ```
     2. Poll `get-instance-state` until it is `running`.
     3. **Open the ports:** `put-instance-public-ports` with 22, 80 and
        443 (TCP, `0.0.0.0/0` and `::/0`).
     4. Read the public IP and output `ip` and
        `domain=${ip//./-}.${STAGING_DNS_SUFFIX}` (Decision 16).

     The rest of the workflow:
     - **`build-web`:** build and push `zainstreat-web-staging:<sha>`
       with that domain.
     - **`deploy`:** `_deploy.yml` (`environment: staging`).
     - **`seed`:** pipe `apps/api/seed.sql` over SSH into
       `compose exec -T postgres psql -v ON_ERROR_STOP=1 ...`.
     - **`admin`:**
       `printf '%s\n' "$ADMIN_PASSWORD" | ssh ... compose exec -T api create_admin --email "$ADMIN_EMAIL" --name "$ADMIN_NAME" --password-stdin`
       (Group 8).
     - **Summary:** print the URLs `https://<domain>`,
       `https://cms.<domain>` and `https://logs.<domain>` to
       `$GITHUB_STEP_SUMMARY`, plus the certificate issuer:
       `echo | openssl s_client -connect <domain>:443 -servername <domain> 2>/dev/null | openssl x509 -noout -issuer`.
       It should show Let's Encrypt, or ZeroSSL if rate-limited.

7.2. **`staging-down.yml`** (`workflow_dispatch`, input `confirm` must
     equal `DELETE STAGING`):
     - The instance name is hard-coded to `zainstreat-staging`. Before
       deleting, check that its `env` tag is `staging`.
     - `aws lightsail delete-instance --instance-name zainstreat-staging --force-delete-add-ons`.
     - Poll `get-instance` until it returns NotFound.

7.3. **Optional, not built:** a cost guard, `staging-reaper.yml`. A
     nightly cron would post a warning in the run summary if staging has
     been up for more than 24 hours. Leave it out unless you ask for it.

## Group 8 — Non-interactive `create_admin` + `create-admin.yml` (repo)

Depends on: Group 4.

8.1. **`apps/api/src/bin/create_admin.rs`:** add a `--password-stdin`
     flag.
     - When the flag is present, read exactly one line from stdin and
       trim the trailing newline. Skip the confirm prompt and keep the
       empty-password check.
     - Without the flag, keep the existing `rpassword` prompts.
     - Update the usage string. Follow the existing `arg_value` style,
       adding a small `has_flag`.

8.2. **Test:** run the binary against the test DB with
     `--password-stdin`, then log in through `/api/admin/login`. Add it to
     `apps/api/tests/` if the existing harness can run a bin; otherwise
     check it manually and record that in validation.

8.3. **`create-admin.yml`** (`workflow_dispatch`):
     - **Inputs:** `environment` (`staging`|`production`), `email`,
       `name`.
     - The password comes from the Environment secret `ADMIN_PASSWORD`;
       the job fails if it is empty.
     - Runs the 7.1 `admin` command against `DEPLOY_HOST` (production) or
       the looked-up staging IP.
     - The run summary reminds you to delete the production
       `ADMIN_PASSWORD` secret afterwards.

## Group 9 — `backup.yml` / `restore.yml` (repo)

Depends on: Groups 1, 2, 4.

9.1. **`backup.yml`:**
     - **Triggers:** `schedule: cron "30 2 * * *"` (production), and
       `workflow_dispatch` with an `environment` input that defaults to
       production.
     - **Environment:** `production-backup` for production,
       `staging` for staging. Both use `BACKUP_SSH_KEY` only (Decision 15).
     - **Steps:** assume the role, then:
       ```bash
       TS=$(date -u +%Y%m%dT%H%M%SZ); P="s3://$BACKUP_BUCKET/$ENV/$TS"
       # The forced command ignores everything except the word db/minio.
       ssh -i backup_key deploy@$HOST db    > db.dump
       ssh -i backup_key deploy@$HOST minio > minio.tgz
       test -s db.dump && test -s minio.tgz
       aws s3 cp db.dump "$P/db.dump" && aws s3 cp minio.tgz "$P/minio.tgz"
       ```
     - Write the S3 prefix and file sizes to the run summary. Failed runs
       email the repo owner (GitHub's default).

9.2. **`restore.yml`** (`workflow_dispatch`):
     - **Inputs:** `environment`, `prefix` (for example
       `production/20261101T023000Z`), and `confirm`, which must equal
       `RESTORE <environment>`.
     - Before restoring into production, take a fresh backup (by calling
       9.1's steps).
     - Restore uses the target Environment's `DEPLOY_SSH_KEY`, behind the
       reviewer gate for production. The backup key can't write.
     - **Steps:**
       1. Download both files and stop `api` and `web`.
       2. Pipe `db.dump` into
          `compose exec -T postgres pg_restore -U ... -d ... --clean --if-exists`.
       3. Pipe `minio.tgz` into `compose exec -T minio sh -c 'rm -rf /data/* && tar -C /data -xzf -'`,
          then restart `minio`.
       4. Run `up -d` and the 4.4 health check.

## Group 10 — Admin on `cms.` (repo, apps/web)

Depends on: nothing. It can be built first and tested locally.

10.1. **`apps/web/proxy.ts`**, host-aware:

```ts
import createMiddleware from "next-intl/middleware";
import { NextResponse, type NextRequest } from "next/server";
import { routing } from "./i18n/routing";

const intlMiddleware = createMiddleware(routing);

// The admin is served from cms.<host> (Phase 21): the same app, rewritten
// onto app/admin/**. The public host redirects old /admin links there.
export default function proxy(request: NextRequest) {
  const host = request.headers.get("host") ?? "";
  const { pathname } = request.nextUrl;

  if (host.startsWith("cms.")) {
    if (pathname.startsWith("/api/")) return NextResponse.next();
    const url = request.nextUrl.clone();
    url.pathname = pathname === "/" ? "/admin" : `/admin${pathname}`;
    return NextResponse.rewrite(url);
  }

  if (pathname === "/admin" || pathname.startsWith("/admin/")) {
    const proto = request.headers.get("x-forwarded-proto") ?? request.nextUrl.protocol.replace(":", "");
    const target = `${proto}://cms.${host}${pathname.slice("/admin".length) || "/"}${request.nextUrl.search}`;
    return NextResponse.redirect(target, 308);
  }
  if (pathname.startsWith("/api/admin/")) {
    return new NextResponse(null, { status: 404 });
  }
  if (pathname.startsWith("/api/")) return NextResponse.next();

  return intlMiddleware(request);
}

export const config = {
  matcher: ["/((?!_next|_vercel|.*\\..*).*)"],
};
```

   - Confirm Caddy's `reverse_proxy` passes `Host` through unchanged (it
     does by default).
   - Confirm the Next 16 `proxy.ts` export shape matches the existing
     file's default-export convention.

10.2. **Drop the `/admin` prefix from every admin link and redirect.**
      - `/admin` → `/`, and `/admin/x` → `/x`.
      - Files: `components/admin-nav.tsx` (items and active check:
        `href === "/" ? pathname === "/" : pathname.startsWith(href)`),
        `admin-login-form.tsx`, `admin-logout-button.tsx`,
        `admin-category-form.tsx`, `admin-menu-item-form.tsx`,
        `admin-category-list.tsx`, `admin-menu-list.tsx`, and every page
        under `app/admin/(protected)/**`.
      - `(protected)/layout.tsx`: `redirect("/login")`.
      - `usePathname()` on the cms host returns the *browser* path (no
        `/admin`), which is why the active check compares against
        unprefixed hrefs. Verify this in the browser.
      - Find them all with `grep -rn '"/admin' apps/web/app apps/web/components`.
        Expect 0 remaining, except `robots.ts`.

10.3. **`apps/web/next.config.ts`:** add
      `allowedDevOrigins: ["cms.localhost"]` so dev HMR works on the cms
      host.

10.4. **`robots.ts`:** leave it unchanged (`/admin` stays disallowed on the
      public host). The cms host gets noindex from Caddy (4.2).

10.5. **Dev:** add a line to the README/`docs`: *admin in dev is
      `http://cms.localhost:3000`*. Browsers resolve `*.localhost`
      natively.

## Group 11 — Docs (repo)

Depends on: Groups 3–10.

11.1. **Rewrite `docs/deployment.md`** around workflows. Sections:
      - **Environments:** staging, production, and the
        GitHub-Environments table.
      - **Release flow:** merge → staging → promote; rollback by
        promoting an older SHA.
      - **Staging up/down.**
      - **Admin accounts:** `create-admin`.
      - **Backups and restore.**
      - **Building a new box:** bootstrap, then the first deploy, then
        restore. This is the Phase 29 path.
      - **Emergency-only SSH:** what still needs it (kernel logs after a
        hang, `docker stats`); Lightsail browser SSH as a fallback.
      - **Remove** the "never through CI" note and the manual `.env` and
        `docker login` steps.

11.2. Update the `/admin` smoke-check line to `cms.`.

## Group 12 — Production cutover (ops + one repo push)

Depends on: everything above, and staging validated end to end.

12.1. **Schedule the cutover** at a quiet time (no open orders, outside
      meal hours). The day before, send the owner a WhatsApp message with
      the new address, `https://cms.zainstreat.com`, and a note that
      they'll need to log in once more. Old `/admin` links and bookmarks
      keep working through the redirect (Decision 20).

12.2. **DNS:** add `cms` A → `63.185.172.26`. Then confirm
      `dig +short cms.zainstreat.com @1.1.1.1`. `promote` refuses to run
      until this resolves (Decision 22).

12.3. **Run `backup` manually** against production and confirm the S3
      objects exist. This works before the cutover because 2.1
      pre-installed the backup path.

12.4. **One push to `master`** that deletes `deploy.yml` (the
      `build.yml`/`promote.yml` path takes over). Then run `promote` with
      that SHA and approve it.
      - The env parity preflight must report no differing keys apart from
        the deliberately new ones. Both sides are resolved against the
        *new* compose files, so compose-derived values such as
        `PUBLIC_ADMIN_URL` match. Only keys new to `.env` itself show up,
        through `caddy`'s `env_file`.
      - Expect `ACME_EMAIL` and `COOKIE_SECURE`, plus any Phase 20
        `BANK_*`/`BUSINESS_KVK` keys the old file lacked. List exactly the
        reported non-credential keys in `allow_env_changes`.
      - If any credential key is reported, stop and fix the Environment
        secret. Never allow-list it.

12.5. **Verify** the site, `cms.zainstreat.com`, the
      `zainstreat.com/admin` redirect and admin login.

12.6. **Retire the PAT.** `_deploy` already runs `docker logout ghcr.io`
      on every deploy, so the stored PAT is gone from the box after
      12.4. Revoke it in GitHub. The git checkout was removed in 2.1.

12.7. **Confirm with the owner** that they're logged in at
      `cms.zainstreat.com` and can see today's orders.

## Group 13 — Reconcile with Phase 20 (repo, after rebase)

Depends on: Phase 20 merged to `master`, and this branch rebased onto
it (Decision 19). Do this before any other repo group.

13.1. **Renumber** Phase 20's references to the old phase numbers in
      `specs/2026-10-25-phase-20-bank-transfer-payment/requirement.md`:

| Line (on the Phase 20 branch) | Old | New |
|---|---|---|
| 101, 154, 162, 176 | Phase 21 (order tracking) | Phase 22 |
| 183 | Phase 22 (catering quotations) | Phase 23 |
| 142 | Phase 23 (French) | Phase 24 |
| 182 | Phase 24 (admin content) | Phase 25 |

      Also update its paragraph in `specs/roadmap.md` ("…that Phase 21
      extends into full tracking") → Phase 22.

13.2. **Resolve the `specs/roadmap.md` conflict.** Keep Phase 20's added
      "Planned in …" paragraph under Phase 20, and keep this branch's
      Phase 21 insertion and renumbering below it.

13.3. **Carry Phase 20's env vars into the generated `.env`.**
      - Add `BANK_ACCOUNT_NAME`, `BANK_IBAN`, `BANK_BIC` and
        `BUSINESS_KVK` to the Group 2.4 table and `.env.example`; they
        are already listed there.
      - `PUBLIC_SITE_URL` stays derived in prod compose
        (`https://${DOMAIN}`), as Phase 20 set it.

13.4. **Add `PUBLIC_ADMIN_URL` to the API** (Decision 20):
      - `docker-compose.yml` api env:
        `PUBLIC_ADMIN_URL: ${PUBLIC_ADMIN_URL:-http://cms.localhost:3000}`.
      - `docker-compose.prod.yml` api env:
        `PUBLIC_ADMIN_URL: https://cms.${DOMAIN}`.
      - Read it at startup, next to Phase 20's `PUBLIC_SITE_URL`, and
        follow its pattern.
      - `apps/api/src/routes/orders.rs`: change the two owner-email links
        (Phase 20 branch lines ~415 and ~532) from
        `{site_url}/admin/orders/{id}` to `{admin_url}/orders/{id}`.
      - Update any API test asserting those email bodies.
      - `grep -rn '/admin/' apps/api/src | grep -v '/api/admin'` → 0
        hits.

13.5. **Phase 20's admin UI changes.** After Group 10.2 strips the
      `/admin` prefix, re-run
      `grep -rn '"/admin' apps/web/app apps/web/components`. Phase 20
      touched `app/admin/(protected)/{page,orders/page,orders/[id]/page}.tsx`
      and added `components/admin-order-actions.tsx`. Expect only
      `robots.ts`, which Phase 20 extended with `"/*/order/"`; keep that
      entry.
