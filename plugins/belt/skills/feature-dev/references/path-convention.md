---
name: path-convention
description: >-
  Single source of truth for the docs/features/<YYYY-MM-DD-topic>/ directory
  naming and file layout used by all feature-dev phases.
---

# Path Convention for feature-dev Artifacts

All feature-dev artifact files live under `docs/features/<YYYY-MM-DD-topic>/`.
Every supplement references this document for naming rules.

## Directory Name

`docs/features/<YYYY-MM-DD-topic>/`

- `<YYYY-MM-DD>`: the date Phase 1 (design) is first invoked, in UTC (ISO 8601).
- `<topic>`: a kebab-case slug (lowercase letters, digits, hyphens; no spaces,
  no underscores). Chosen interactively with the user during Phase 1.

Examples:
- `docs/features/2026-04-14-user-authentication/`
- `docs/features/2026-05-01-payment-refactor/`

## Topic Slug Rules

- Only `[a-z0-9-]`, no leading/trailing hyphens, no consecutive hyphens.
- Minimum 3 characters, maximum 48 characters.
- Must not collide with an existing directory under `docs/features/`.
- Must be stable for the duration of the feature (do not rename mid-flight).

If a collision is detected, Phase 1 supplement appends `-N` (e.g. `-2`) until
unique.

## Worktree Branch Correspondence

The worktree branch name created in Phase 1 must match:

```
feature/<YYYY-MM-DD-topic>
```

Example: directory `docs/features/2026-04-14-user-authentication/` maps to
branch `feature/2026-04-14-user-authentication`.

## File Layout per Feature

| File | Phase | Producer | When |
|------|-------|----------|------|
| `design.md` | 1 | /brainstorming (+ brainstorming-supplement) | always |
| `test-strategy.md` | 2 | /test-scenarios | always |
| `scenarios.yml` | 2 | /test-scenarios | when `args.e2e` |
| `plan.md` | 3 | /writing-plans (+ writing-plans-supplement) | always |
| `monkey-test-report.md` | 6 | /monkey-test | when `args.e2e` |
| `monkey-test-results.json` | 6 | /monkey-test | when `args.e2e` |
| `dogfood-report/report.md` | 7 | /dogfood (+ dogfood-supplement) | when `args.e2e` |
| `dogfood-report/screenshots/*` | 7 | /dogfood | when `args.e2e` |
| `dogfood-report/videos/*` | 7 | /dogfood | when `args.e2e` |

Phase 4 (execute) and Phase 5 (code-review) write to git history and
`belt://current/review/findings.json` (resolve via `belt-agent status` or
`belt-agent locate belt://current/review/findings.json`), not under
`docs/features/`.

Phase 8 (integrate) consumes from `docs/features/<topic>/` but does not write
there.

## Editing Rules

- Phases generate these files; do not hand-edit.
- Hand-edits break belt's phase-start mtime filter (BELT-32 DD-1) used for
  artifact glob resolution.
- If a correction is needed, re-run the owning phase (verify → regate → step).
