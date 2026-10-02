---
name: validate
description: Run the full validation sweep for zainstreat (lint, build, and a live HTTP smoke test of the running app) and report a clear pass/fail summary. Use after finishing a chunk of implementation work, and before checking off a spec-kit phase's validation.md checklist.
---

# Validate

A `.ts`/`.tsx` edit in `apps/web` already gets auto-linted by a
project hook (`.claude/hooks/lint-changed-file.sh`). This skill is the
heavier, on-demand check: a full lint + build sweep plus a real
HTTP smoke test of the running app, not just "it compiles."

## Steps

1. **Detect scope.** Run `git status` and `git diff master --stat` (or the
   relevant base branch) to see whether `apps/web/**`, `apps/api/**`, or
   both changed. Only run the matching steps below — skip whichever app
   didn't change.

2. **Web checks** (if `apps/web` changed), from inside `apps/web`:
   - `pnpm lint` — full-project sweep (not just the one file the hook
     already checked).
   - `pnpm build`.
   - Stop and report immediately if either fails; don't proceed to the
     live smoke test against a build that doesn't compile.

3. **API checks** (if `apps/api` changed), from inside `apps/api`:
   - `cargo check`, and `cargo clippy` if it's available.
   - `cargo build`.

4. **Live smoke test.** Start whatever changed:
   - Prefer `docker compose up -d` from the repo root when both
     `web`/`api`/`postgres`/`minio` need to be up together (matches the
     Phase 1 dev convention in `docs/local-development.md`).
   - For a web-only change, `pnpm start` (after the build above) is
     enough and faster.

   Then verify, with `curl`:
   - API: `GET /health` → `200` with the expected body.
   - Web: `GET /` → redirects to a locale (`/en` or `/nl`, per the
     Phase 3 i18n setup). Then `GET /en` and `GET /nl` directly → `200`,
     with locale-correct page content and the correct `<html lang>`.
   - **Also check the current phase's own `specs/<date>-phase-N-*/validation.md`**
     for anything else to verify mechanically (additional routes, expected
     content, status codes). Don't hardcode phase-specific checks into this
     skill beyond the Phase 3 baseline above — pull them from that file so
     this skill doesn't go stale as the roadmap progresses past Phase 3.

5. **Tear down** whatever was started in step 4 (`docker compose down`, or
   kill any `pnpm start`/`cargo run` process left running) before
   finishing, even if a check failed.

6. **Report.** A clear pass/fail line per check performed. Explicitly call
   out anything in the relevant `validation.md` that this skill could
   *not* automate (e.g. "toggle visually switches locale in a browser" or
   anything requiring human visual judgment) as **manual — not checked
   here**, rather than silently omitting it.
