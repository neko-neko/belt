---
name: feature-dev
description: >-
  Quality-gated development pipeline (8 phases). Design → test scenarios → plan →
  execute → code review → monkey test (E2E scripted) → dogfood (E2E exploratory) →
  integrate. Web UI testing phases are conditional on --e2e.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Belt pipeline for quality-gated development. 8 phases driven by belt-agent.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in code-review |

## Phase-Specific Invocation Rules

### Phase 1: design

- **INVOKE 1**: Read `./references/brainstorming-supplement.md` into context.
- **INVOKE 2**: Skill tool `/brainstorming`.
- The supplement injects parallel exploration (code-explorer / code-architect /
  impact-analyzer), implicit-rules extraction, required design sections, and
  worktree creation order.

### Phase 2: test-scenarios

- **INVOKE**: Skill tool `/test-scenarios` with `e2e` passed through from args.
- Produces `test-strategy.md` always; produces `scenarios.yml` when e2e.

### Phase 3: plan

- **INVOKE 1**: Read `./references/writing-plans-supplement.md`.
- **INVOKE 2**: Skill tool `/writing-plans`.
- The supplement enforces path override and Must-Verify / scenarios cross-referencing.

### Phase 4: execute

- **INVOKE**: Skill tool `/subagent-driven-development`.
- Orchestrator must reconstruct plan tasks into self-contained implementation
  specs before dispatching `feature-implementer` subagents. Do not forward
  broad research verbatim.

### Phase 5: code-review

- **INVOKE**: Skill tool `/code-review` with `codex` passed through.
- On fix commits, Phase 4 validate is re-verified per belt regate semantics.
  `max_retries: 3` limits the review-fix loop.

### Phase 6: monkey-test (when e2e)

- **INVOKE 1**: Read `./references/monkey-test-supplement.md`.
- **INVOKE 2**: Skill tool `/monkey-test`.

### Phase 7: dogfood (when e2e)

- **INVOKE 1**: Read `./references/dogfood-supplement.md`.
- **INVOKE 2**: Skill tool `/dogfood`.
- The supplement injects prior-phase artifacts as exploration hints.

### Phase 8: integrate

- **INVOKE 1**: Read `./references/worktrunk-supplement.md`.
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`) and
  execute accordingly via `/worktrunk`.

## Red Flags

- **Never skip the Phase 1 supplement load**: parallel exploration and the
  required design sections depend on it.
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through
  `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are
  phase-produced; manual edits break belt's phase-start mtime filter.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — Phase 1 overrides
- `./references/writing-plans-supplement.md` — Phase 3 overrides
- `./references/monkey-test-supplement.md` — Phase 6 context injection
- `./references/dogfood-supplement.md` — Phase 7 overrides and context injection
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
