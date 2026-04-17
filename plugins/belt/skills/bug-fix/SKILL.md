---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix planning,
  code review, and regression verification. Use when a bug needs structured
  diagnosis and verified repair. --e2e adds browser-based regression tests;
  --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Belt pipeline for quality-gated debugging. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`fix-plan-review` / `execute` /
`code-review`) have no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| rca | `./references/rca-supplement.md` | RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, `rca-scenarios.yml` produce (when `--e2e`) |
| fix-plan | `./references/fix-plan-supplement.md` | RCA Fix Strategy → task traceability, Given/When/Then test cases, verifiable completion conditions, task granularity |
| monkey-test | `./references/monkey-test-supplement.md` | scenarios source = `docs/plans/*-rca-scenarios.yml`, first scenario verifies Reproduction Test now PASSes, glob collision resolution |
| dogfood | `./references/dogfood-supplement.md` | Impact Scope + Symmetry exploration, Root Cause re-emergence flag, CLI-only graceful degradation |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **fix-plan-review**: `/belt:spec-review` is reused for fix-plan
  review. The grill-me prompt under the `design-judgment` observation does
  not fire by default (design decisions are already settled in rca /
  fix-plan). If it fires, treat it as a signal that upstream phases need
  to be revisited.
- **execute**: orchestrator must reconstruct fix plan tasks into
  self-contained implementation specs before dispatching
  `belt-agent:feature-implementer` subagents. Do not forward RCA / Fix
  Plan excerpts verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `rca` / `fix-plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter.

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip rca**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip supplement loading when listed above**: without bug-fix specific overrides, behavior drifts.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never proceed without a failing reproduction test**: RCA blocker.
- **Never filter or omit review findings**: triage of `/belt:code-review` and `/belt:spec-review` output is the user's responsibility.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never hand-edit files under `docs/plans/<topic>-*`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md` only.
- **Never leave the narrative note's four sections blank**: gate is `file_exists` only; empty sections pass but break downstream consumers.

## References

- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` naming (SSOT)
- `./references/rca-supplement.md` — rca phase override
- `./references/fix-plan-supplement.md` — fix-plan phase override
- `./references/monkey-test-supplement.md` — monkey-test phase override
- `./references/dogfood-supplement.md` — dogfood phase override and CLI-only degradation
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
