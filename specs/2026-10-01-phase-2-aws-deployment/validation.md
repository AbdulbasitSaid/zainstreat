# Phase 2 — AWS Deployment Infrastructure & CI/CD: Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

- [ ] Pushing/merging to `master` triggers `.github/workflows/deploy.yml`
      automatically, with no manual step — the Actions run shows a
      build-and-push job followed by a deploy job, both green.
- [ ] Both `ghcr.io/<owner>/zainstreat-web` and `...zainstreat-api`
      packages appear in the GitHub org/user's Packages tab after the
      first successful run, each tagged `latest` and with the triggering
      commit SHA.
- [ ] The GHCR packages are confirmed **private** (not publicly
      pullable without authentication).
- [ ] Visiting `https://<domain>` from a machine outside the office
      network returns the Phase 1 hello page over a valid HTTPS
      connection (browser shows a trusted certificate, no warnings).
- [ ] `curl -i https://api.<domain>/health` returns HTTP `200` and body
      `{"status":"ok"}` over valid HTTPS.
- [ ] `docker compose logs caddy` on the server shows successful
      certificate issuance for both hostnames, no ACME errors.
- [ ] Plain `http://<domain>` and `http://api.<domain>` redirect to
      HTTPS (Caddy's default behavior) rather than serving over HTTP.
- [ ] Attempting to reach `postgres` (`5432`), `minio` (`9000`/`9001`), or
      `api` (`8080`) directly against the instance's static IP (not
      through Caddy) **fails to connect** — confirms only `80`/`443` are
      publicly reachable.
- [ ] Lightsail firewall console shows exactly `22`, `80`, `443` open —
      nothing else.
- [ ] Lightsail console shows the automatic snapshot add-on enabled on
      the instance, with at least one snapshot present.
- [ ] The `deploy` system user on the server has no `sudo` access and is
      a member of the `docker` group only — confirm it cannot run
      anything beyond Docker commands.
- [ ] The `deploy` user's SSH key is distinct from the admin key used for
      manual access; `DEPLOY_SSH_KEY` in GitHub matches `deploy`'s
      `authorized_keys` entry, not the admin account's.
- [ ] No deploy secrets (`DEPLOY_SSH_KEY`, the server's GHCR pull PAT,
      `.env` contents) appear anywhere in Actions logs (check the run's
      log output doesn't echo them) or in the repository's git history.
- [ ] `docker compose -f docker-compose.yml -f docker-compose.prod.yml
      ps` on the server shows `web`, `api`, `postgres`, `minio`, `caddy`
      all running/healthy, with `web`/`api` running from pulled
      `ghcr.io/...` images (not built locally — `docker compose config`
      shows no `build:` key for either).
- [ ] `docs/deployment.md` exists and accurately describes the CI/CD flow
      plus the manual rollback runbook; following the rollback steps on a
      test basis (pulling a previous image tag) actually works.
- [ ] No Phase 3+ work is present on this branch: no DB migrations, no
      new application features beyond the Phase 1 scaffold being
      deployed.

## Definition of done

Phase 2 is complete when every box above is checked: a push to `master`
automatically builds and ships `web`/`api` to an AWS Lightsail instance in
`eu-central-1` via GitHub Actions and a private GHCR registry, the domain
serves it over trusted HTTPS, nothing except Caddy is publicly reachable,
the `deploy` user is least-privilege and distinct from the admin account,
daily snapshots are running, and the whole flow — plus a manual rollback
path — is documented and reproducible. Ready for Phase 3 (data model)
onward to ship incrementally to this same environment via the same
pipeline.
