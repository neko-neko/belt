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

Belt pipeline for quality-gated debugging. 8 phases driven by belt-agent.

## Pipeline Overview

```
rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate
```

`monkey-test` and `dogfood` run only when `--e2e` is set.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in review phases |

## Phase-Specific Invocation Rules

### Phase 1: rca

- **INVOKE 1**: Read `./references/rca-supplement.md` into context.
- **INVOKE 2**: Skill tool `/systematic-debugging`.
- The supplement enforces RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, and (when `--e2e`) `rca-scenarios.yml` produce.

### Phase 2: fix-plan

- **INVOKE 1**: Read `./references/fix-plan-supplement.md`.
- **INVOKE 2**: Skill tool `/writing-plans`.
- The supplement enforces RCA Fix Strategy → task traceability, Given/When/Then test cases, verifiable completion conditions, and task granularity.

### Phase 3: fix-plan-review

- **INVOKE**: Skill tool `/spec-review:spec-review` with `codex` passed through.
- No supplement required; the skill is self-contained.
- Note: spec-review is reused for fix-plan review. The grill-me prompt under
  the `design-judgment` observation does not fire by default (design decisions
  are already settled in rca / fix-plan). If it does fire, treat it as a signal
  that upstream phases (rca / fix-plan) need to be revisited.

### Phase 4: execute

- **INVOKE**: Skill tool `/subagent-driven-development`.
- Orchestrator must reconstruct fix plan tasks into self-contained implementation specs before dispatching `belt-agents:feature-implementer` subagents. Do not forward RCA / Fix Plan excerpts verbatim.

### Phase 5: code-review

- **INVOKE**: Skill tool `/code-review:code-review` with `codex` passed through.
- On fix commits, Phase 4 validate is re-verified per belt regate semantics. `max_retries: 3` limits the review-fix loop.

### Phase 6: monkey-test (when `--e2e`)

- **INVOKE 1**: Read `./references/monkey-test-supplement.md`.
- **INVOKE 2**: Skill tool `/monkey-test:monkey-test`.
- The supplement points scenarios source at `docs/plans/*-rca-scenarios.yml`, requires the first scenario to verify the RCA Reproduction Test now PASSes, and documents glob collision resolution.

### Phase 7: dogfood (when `--e2e`)

- **INVOKE 1**: Read `./references/dogfood-supplement.md`.
- **INVOKE 2**: Skill tool `/dogfood`.
- The supplement focuses exploration on fix Impact Scope + Symmetry pairs, flags Root Cause mechanism re-emergence, and provides CLI-only graceful degradation for UI-free fixes.

### Phase 8: integrate

- **INVOKE 1**: Read `./references/worktrunk-supplement.md`.
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`).
- **INVOKE 3**: Execute via `/worktrunk` per user's choice.

## Narrative Notes

The following six phases produce a narrative note so context can be restored after `/clear` (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **rca** / **fix-plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) and minimal frontmatter (`phase`, `run_id`).

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` itself is the user's call — Claude Code runtime constraints prevent automation. Use narrative notes as an option when context has grown large after a heavy phase (for example, right after rca or execute).

## Red Flags

- **Never skip Phase 1 (rca)**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip the supplement load in Phases 1 / 2 / 6 / 7 / 8**: without bug-fix specific overrides injected, behavior drifts.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never proceed without a failing reproduction test**: RCA-05 blocker.
- **Never filter or omit review findings**: triage of `/code-review:code-review` and `/spec-review:spec-review` output is the user's responsibility.
- **Never bypass the Phase 8 A/B choice**: the merge-vs-PR decision is always the user's.
- **Never hand-edit files under `docs/plans/<topic>-*`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md` only.
- **Never leave the narrative note's four sections blank**: the gate is `file_exists` only and empty sections still pass, but downstream consumers cannot restore context. Use at least `(none)` as a placeholder and always keep the heading.

## References

- `./references/path-convention.md` — SSOT for `docs/plans/YYYY-MM-DD-<topic>-*` naming
- `./references/rca-supplement.md` — Phase 1 override
- `./references/fix-plan-supplement.md` — Phase 2 override
- `./references/monkey-test-supplement.md` — Phase 6 override
- `./references/dogfood-supplement.md` — Phase 7 override and CLI-only degradation
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
