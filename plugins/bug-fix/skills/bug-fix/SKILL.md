---
name: bug-fix
description: >-
  Quality-gated debugging pipeline (8 phases). rca → fix-plan → plan-review →
  execute → code-review → monkey-test (E2E scripted) → dogfood (E2E exploratory)
  → integrate. --e2e enables monkey-test and dogfood.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Belt pipeline for quality-gated debugging. 8 phases driven by belt-agent.

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
- Note: spec-review を fix-plan レビューに流用する。`design-judgment` 観点の
  grill-me は原則発動しない (設計判断は rca / fix-plan で決定済みのため)。
  発動した場合は上流 (rca / fix-plan) の見直しサインとして扱う。

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

以下 6 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **rca** / **fix-plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

各 note は 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) と minimal frontmatter (`phase`, `run_id`) を含む。

規約詳細: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` 自体は user 判断（Claude Code runtime 制約で自動化不可）。重い phase 完了直後（例: rca / execute 後）に context が膨れた場合の選択肢として narrative を活用できる。

## Red Flags

- **Never skip Phase 1 (rca)**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip Phase 1 / 2 / 6 / 7 / 8 の supplement load**: bug-fix 固有 override が inject されず drift 発生.
- **Never delegate root cause synthesis to subagents**: parallel exploration results は orchestrator が再構築.
- **Never proceed without a failing reproduction test**: RCA-05 blocker.
- **Never filter or omit review findings**: `/code-review:code-review`, `/spec-review:spec-review` の triage は user 責務.
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR は user 決定.
- **Never hand-edit files under `docs/plans/<topic>-*`**: phase-produced; manual edits break belt の phase-start mtime filter.
- **Never modify the consumed global skills**: override は `references/*-supplement.md` 経由のみ.
- **Never leave narrative note 4 sections blank**: gate は file_exists のみで空 section も通過するが、下流 consume で context 復元不能になる。最低限 `(none)` placeholder を置き、heading は必ず保持。

## References

- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` 命名 SSOT
- `./references/rca-supplement.md` — Phase 1 override
- `./references/fix-plan-supplement.md` — Phase 2 override
- `./references/monkey-test-supplement.md` — Phase 6 override
- `./references/dogfood-supplement.md` — Phase 7 override and CLI-only degradation
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
