# zainstreat

A food ordering/catering site ("Zain's Treat n More") — Next.js 16 (App
Router) + React + TypeScript frontend (`apps/web`), Rust/Axum API
(`apps/api`), Postgres + MinIO, deployed to AWS Lightsail.

## Spec-kit workflow

This repo plans work in `specs/`: `mission.md`, `roadmap.md`, and
`tech-stack.md` are the standing decisions; each implementation phase gets
its own branch (`YYYY-MM-DD-phase-N-<name>`) and a matching
`specs/<same-name>/{requirement.md,plan.md,validation.md}` folder. Read
`specs/roadmap.md` for the current phase order before starting new work.

## TypeScript: no `any`, anywhere

`apps/web/tsconfig.json`'s `strict: true` rejects implicit `any`;
`apps/web/eslint.config.mjs` additionally sets
`@typescript-eslint/no-explicit-any` to `error`, rejecting explicit `any`
too (see `specs/tech-stack.md`'s Frontend section). Use a precise type,
`unknown` with narrowing, or a generic instead.

## Validation

- A project hook (`.claude/hooks/lint-changed-file.sh`, wired in
  `.claude/settings.json`) automatically runs ESLint on any `.ts`/`.tsx`
  file in `apps/web` right after it's edited, and surfaces failures
  immediately.
- Run the `/validate` skill (`.claude/skills/validate/SKILL.md`) after
  finishing a chunk of implementation work, or before checking off a
  phase's `specs/<phase>/validation.md` pass/fail checklist — it runs the
  full lint + build sweep and a live HTTP smoke test of the running app,
  not just a compile check.
