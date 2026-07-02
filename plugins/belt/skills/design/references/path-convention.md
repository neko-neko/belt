---
name: path-convention
description: >-
  Single source of truth for the docs/features/<YYYY-MM-DD-topic>/ directory
  naming and file layout used by all belt stage pipelines (design / diagnose /
  build / verify) and their composed entry points (feature-dev / bug-fix).
---

# Path Convention for belt Stage Artifacts

All belt run artifact files — feature runs and bug runs alike — live under
`docs/features/<YYYY-MM-DD-topic>/`. Every supplement references this
document for naming rules. The former bug-fix convention
(`docs/plans/YYYY-MM-DD-<topic>-*` flat files) is retired.

## Directory Name

`docs/features/<YYYY-MM-DD-topic>/`

- `<YYYY-MM-DD>`: the date the run's first phase (design for feature runs,
  rca for bug runs) is first invoked, in UTC (ISO 8601).
- `<topic>`: a kebab-case slug (lowercase letters, digits, hyphens; no spaces,
  no underscores). Chosen interactively with the user during the first phase.

Examples:
- `docs/features/2026-04-14-user-authentication/`
- `docs/features/2026-05-01-payment-refactor/`

## Topic Slug Rules

- Only `[a-z0-9-]`, no leading/trailing hyphens, no consecutive hyphens.
- Minimum 3 characters, maximum 48 characters.
- Must not collide with an existing directory under `docs/features/`.
- Must be stable for the duration of the run (do not rename mid-flight).

If a collision is detected, the first phase's supplement appends `-N`
(e.g. `-2`) until unique.

## Worktree Branch Correspondence

The worktree branch created in the first phase must match:

- Feature runs: `feature/<YYYY-MM-DD-topic>`
- Bug runs: `bugfix/<YYYY-MM-DD-topic>`

Example: directory `docs/features/2026-04-14-user-authentication/` maps to
branch `feature/2026-04-14-user-authentication`.

## File Layout per Topic

| File | Producing phase | Producer | When |
|------|-----------------|----------|------|
| `design.md` | design | /brainstorming (+ brainstorming-supplement) | feature runs |
| `test-strategy.md` | test-scenarios | /belt:test-scenarios | feature runs |
| `scenarios.yml` | test-scenarios | /belt:test-scenarios | feature runs, when `args.e2e` |
| `plan.md` | plan | /writing-plans (+ writing-plans-supplement) | feature runs |
| `rca-report.md` | rca | /systematic-debugging (+ rca-supplement) | bug runs |
| `rca-scenarios.yml` | rca | /systematic-debugging (+ rca-supplement) | bug runs, when `args.e2e` |
| `fix-plan.md` | fix-plan | /writing-plans (+ fix-plan-supplement) | bug runs |
| `monkey-test-report.md` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `monkey-test-results.json` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `monkey-test-screenshots/*` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `dogfood-report/report.md` | dogfood | /dogfood (+ dogfood-supplement) | when `args.e2e` |
| `dogfood-report/screenshots/*` | dogfood | /dogfood | when `args.e2e` |
| `dogfood-report/videos/*` | dogfood | /dogfood | when `args.e2e` |

The execute and code-review phases write to git history and
`belt://current/review/findings.json` (resolve via `belt-agent status` or
`belt-agent locate belt://current/review/findings.json`), not under
`docs/features/`.

The integrate phase consumes from `docs/features/<topic>/` but does not
write there.

## Glob Resolution

belt-agent resolves `docs/features/*/<name>` glob patterns with the
phase-start mtime filter; on ambiguity (multiple matching topics), the most
recently modified file wins (mtime DESC).

## Editing Rules

- Phases generate these files; do not hand-edit.
- Hand-edits break belt's phase-start mtime filter (BELT-32 DD-1) used for
  artifact glob resolution.
- If a correction is needed, re-run the owning phase (verify -> regate -> step).
