---
name: feature-dev
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Belt pipeline for quality-gated development. 9 phases driven by belt-agent.

## Pipeline Overview

```
design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate
```

`monkey-test` and `dogfood` run only when `--e2e` is set.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in spec-review and code-review |

## Phase-Specific Invocation Rules

### Phase 1: design

- **INVOKE 1**: Read `./references/brainstorming-supplement.md` into context.
- **INVOKE 2**: Skill tool `/brainstorming`.
- The supplement injects parallel exploration (belt-agents:code-explorer / belt-agents:code-architect /
  belt-agents:impact-analyzer), implicit-rules extraction, required design sections, and
  worktree creation order.

### Phase 2: test-scenarios

- **INVOKE**: Skill tool `/test-scenarios:test-scenarios` with `e2e` passed through from args.
- Produces `test-strategy.md` always; produces `scenarios.yml` when e2e.

### Phase 3: spec-review

- **INVOKE**: Skill tool `/spec-review:spec-review` with `codex` passed through from args.
- Targets `test-strategy.md`. If `scenarios.yml` exists (`args.e2e`), include
  it in the review scope.
- grill-me dialogue for `requirements` / `design-judgment` findings; direct
  selection triage for the remaining observations.
- regate: `test-scenarios`; fix loop capped at `max_retries: 3`.

### Phase 4: plan

- **INVOKE 1**: Read `./references/writing-plans-supplement.md`.
- **INVOKE 2**: Skill tool `/writing-plans`.
- The supplement enforces path override and Must-Verify / scenarios cross-referencing.

### Phase 5: execute

- **INVOKE**: Skill tool `/subagent-driven-development`.
- Orchestrator must reconstruct plan tasks into self-contained implementation
  specs before dispatching `belt-agents:feature-implementer` subagents. Do not forward
  broad research verbatim.

### Phase 6: code-review

- **INVOKE**: Skill tool `/code-review:code-review` with `codex` passed through.
- On fix commits, Phase 5 validate is re-verified per belt regate semantics.
  `max_retries: 3` limits the review-fix loop.

### Phase 7: monkey-test (when e2e)

- **INVOKE 1**: Read `./references/monkey-test-supplement.md`.
- **INVOKE 2**: Skill tool `/monkey-test:monkey-test`.

### Phase 8: dogfood (when e2e)

- **INVOKE 1**: Read `./references/dogfood-supplement.md`.
- **INVOKE 2**: Skill tool `/dogfood`.
- The supplement injects prior-phase artifacts as exploration hints.

### Phase 9: integrate

- **INVOKE 1**: Read `./references/worktrunk-supplement.md`.
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`) and
  execute accordingly via `/worktrunk`.

## Narrative Notes

以下 6 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **design** / **plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

各 note は 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) と minimal frontmatter (`phase`, `run_id`) を含む。

規約詳細: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` 自体は user 判断（Claude Code runtime 制約で自動化不可）。重い phase 完了直後（例: design / execute / code-review 後）に context が膨れた場合の選択肢として narrative を活用できる。

## Red Flags

- **Never skip the Phase 1 supplement load**: parallel exploration and the
  required design sections depend on it.
- **Never bypass the Phase 9 A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through
  `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are
  phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave narrative note 4 sections blank**: gate は file_exists のみで空 section も通過するが、下流 consume で context 復元不能になる。最低限 `(none)` placeholder を置き、heading は必ず保持。

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — Phase 1 overrides
- `./references/writing-plans-supplement.md` — Phase 4 overrides
- `./references/monkey-test-supplement.md` — Phase 7 context injection
- `./references/dogfood-supplement.md` — Phase 8 overrides and context injection
- `./references/worktrunk-supplement.md` — Phase 9 A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with bug-fix)
