# Phase 1 — Scaffolding: Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

- [ ] `cp .env.example .env` works with no manual edits required for a
      first run.
- [ ] `docker compose up` starts all four services (`web`, `api`,
      `postgres`, `minio`) with no manual intervention.
- [ ] `docker compose ps` shows `postgres`, `minio`, and `api` as
      `healthy` (not just `running`).
- [ ] `curl -i http://localhost:8080/health` returns HTTP `200` and body
      `{"status":"ok"}`.
- [ ] Visiting `http://localhost:3000` (or `curl`) returns the hello page,
      with the business name ("Zain's Treat n More") visible and the Pico
      CSS stylesheet rendering real layout/typography (not unstyled HTML).
- [ ] MinIO console reachable at `http://localhost:9001` and logs in with
      `MINIO_ROOT_USER` / `MINIO_ROOT_PASSWORD` from `.env`.
- [ ] `apps/web/pnpm-lock.yaml` and `apps/api/Cargo.lock` are committed.
- [ ] After `docker compose up` has run at least once, `git status` on the
      repo is clean except for intentional source changes — `.gitignore`
      correctly excludes `node_modules/`, `.next/`, `target/`, and `.env`.
- [ ] No Phase 2+ work is present on this branch: no DB migrations, no
      `sqlx`/`argon2`/`tower-sessions`/`lettre` dependencies, no menu/order
      routes, no brand theming beyond the minimal Pico CSS stylesheet check.
- [ ] `web` and `api` remain decoupled — the hello page makes no network
      call to the API, and the API has no CORS middleware.
- [ ] `docker compose down` cleanly stops all services.

## Definition of done

Phase 1 is complete when every box above is checked and the branch can be
merged into `master` without breaking anything Phase 2 will build on top
of (a running Postgres instance and a running API process inside the same
Compose network).
