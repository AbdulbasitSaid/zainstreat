---
name: plan-phase
description: Plan the next (or a revisited) zainstreat roadmap phase — research, resolve open decisions with the user, then write only its git branch and specs/ files (requirement.md, plan.md, validation.md, plus roadmap.md/tech-stack.md updates). Never implements code. Not for off-roadmap housekeeping.
---

# Plan Phase

Formalizes the planning workflow already used for every phase in this
repo (scaffolding, AWS deployment, Dutch/English i18n): research, resolve
open product/architecture decisions with the user, then write the
branch + spec files for a roadmap phase. **This skill never writes or
edits implementation code.** Its output is a git branch and files under
`specs/` — nothing else.

## 0. Scope check — roadmap phase, or housekeeping?

This skill is for **numbered roadmap phases** only. If the request is
clearly a roadmap feature (new functionality toward the MVP), proceed.

If it's unclear whether the request belongs on the roadmap or is
off-roadmap housekeeping (tooling, config, dev-workflow changes — e.g. the
TypeScript-strictness fix or the validation hook/skill work earlier in
this project's history, neither of which got a spec folder), **ask the
user** which it is.

If it's housekeeping: **stop here**. Say so explicitly — this repo's
convention keeps housekeeping off the roadmap and out of `specs/`. Do not
invent a non-phase spec format or write anything.

## 1. Read current state

Read, in order:
- `specs/mission.md` — business goals/personas/non-negotiable rules this
  phase must respect.
- `specs/roadmap.md` — the full phase list, to find the current highest
  phase number and where this phase fits (appended at the end, or
  inserted mid-sequence with everything after it renumbered — as happened
  when the i18n phase was inserted ahead of Data Model).
- `specs/tech-stack.md` — already-pinned tech decisions, so the phase
  doesn't re-litigate or contradict them.
- The most recent phase folder(s) under `specs/` (e.g. whichever is
  newest) — read all three files as the live structural template. Don't
  rely on a hardcoded format below without checking it still matches;
  conventions may have evolved.

## 2. Research (read-only)

Grep/read whatever source is relevant to ground the plan in what actually
exists — don't assume. For a small, well-scoped phase, do this directly.
For broader or unfamiliar territory, launch an Explore subagent instead of
guessing. Either way: **read-only**. No edits yet.

## 3. Design the approach

Work out the concrete implementation approach, including any new
library/technology choice the phase would need to pin in
`specs/tech-stack.md`. For a non-trivial phase, a Plan subagent can help
validate the approach — give it full context from step 1-2 rather than a
bare restatement of the request.

## 4. Ask the user about open decisions

Use `AskUserQuestion` for every genuine product or architecture fork the
research surfaced (e.g. the i18n phase's URL strategy, default locale, and
admin-scope questions). Don't silently guess and don't finalize
`requirement.md` around an assumption the user should actually make. Skip
this step only if there's truly nothing to decide.

## 5. Determine the phase number and name

Next sequential number after the current highest phase in
`specs/roadmap.md`, unless the user wants it inserted earlier (then
renumber every phase after it — leave the independent "Beyond MVP"
section's numbering alone, it's a separate sequence). Pick a short
kebab-case `<name>` for it. Decide this once, here — the exact same
`YYYY-MM-DD-phase-N-<name>` string is reused verbatim for both the branch
(step 6) and the spec folder (step 9). They must match exactly; never
derive two slightly different slugs for the same phase.

## 6. Create the branch

`git checkout -b YYYY-MM-DD-phase-N-<name>` off the current branch, using
the exact name decided in step 5 (check `git status` first — stash or
flag anything uncommitted per standard git safety practice before
switching). No code files are touched or committed.

## 7. Update specs/roadmap.md

Insert the new phase's entry. If inserted mid-sequence, renumber every
subsequent phase's heading and any cross-references to them elsewhere in
the file.

## 8. Update specs/tech-stack.md (only if needed)

Only if this phase pins a new technology/library decision. Match the
existing "choice + why" section format exactly, and add a row to the
Summary Table.

## 9. Write the three phase files

`specs/YYYY-MM-DD-phase-N-<name>/` — the identical name used for the
branch in step 6:

- **`requirement.md`**: `## Source` (quote the roadmap bullet verbatim, or
  state it's a new phase with no prior source), `## Context` (why now,
  what already exists), `## Decisions` (numbered, each with a brief
  rationale — including anything resolved via step 4's questions),
  `## Out of scope` (explicitly deferred items, with phase references),
  `## Open risks flagged during planning`.
- **`plan.md`**: `## Group 0`, `## Group 1`, ... — numbered,
  dependency-ordered task groups. Concrete file paths, exact package
  names/versions, and code/config snippets wherever that removes ambiguity
  for whoever implements it later.
- **`validation.md`**: `## Pass/fail checklist` — concrete, testable
  checkboxes (commands to run, routes to curl, files that should or
  shouldn't exist) — plus a closing `## Definition of done` paragraph.

## 10. Hard boundary

Never create or edit anything outside `specs/`, other than the one
`git checkout -b` in step 6 — no `apps/web/**`, `apps/api/**`,
`docker-compose*.yml`, `.github/**`, `Caddyfile`, etc. Never run
build/install/dependency commands. A read-only lookup (checking a
package's current published version before pinning it, for example) is
fine — installing it is not. If asked to also implement after planning,
say that's a separate step and stop; don't start writing code.

## 11. Summary

End with: the branch name, which files were written or updated, and a
one-line reminder that implementation is a separate next step (e.g. the
`run` skill or manual implementation, then `/validate` once code exists).
