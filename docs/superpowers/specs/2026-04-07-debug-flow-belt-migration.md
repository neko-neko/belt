# debug-flow Belt Pipeline Migration

**Linear**: BELT-20 (child)
**Status**: Draft
**Date**: 2026-04-07

## Summary

Port the debug-flow skill to `examples/skills/debug-flow/` as a belt pipeline.
This is a dogfooding exercise that demonstrates belt's compositional capabilities
with a debugging-focused workflow.

## Background

debug-flow is a quality-gated debugging orchestrator with 8 phases
(RCA → Fix Plan → Fix Plan Review → Execute → Smoke Test → Code Review →
Test Review → Integrate). The current implementation is a monolithic 579-line
SKILL.md that LLMs must hold entirely in context.

feature-dev was successfully migrated to a belt pipeline, achieving 89% context
reduction (35.7KB → 4KB). debug-flow shares significant structural overlap with
feature-dev's Phase 5-10 (Execute → Integrate) and can reuse existing
sub-pipelines (code-review, test-review, implementation-review, smoke-test).

## Design Decisions

### D1: RCA as a single pipeline phase

RCA contains complex substeps (symptom structuring → parallel exploration →
hypothesis formation → reproduction test → RCA Report → worktree → commit),
but these are inherently non-deterministic and depend on LLM exploration.
belt's value is in deterministic control. The gate (`file_exists`) and audit
(done-criteria) verify the output quality.

Precedent: feature-dev's design phase handles brainstorming as a single phase.

### D2: Independent done-criteria (no sharing with feature-dev)

Each example in `examples/skills/` is self-contained. Symlinks or shared
directories add complexity for readers. Duplication cost is low; independence
value is high.

### D3: No --linear flag

The examples focus on belt's own features (phases, gates, regate, when,
sub-pipelines). Linear sync is an external service concern handled by SKILL.md
dispatch rules if needed at the real skill level.

### D4: Structured references (feature-dev parity)

Same reference directory pattern as feature-dev: `references/done-criteria/`,
`audit-protocol.md`, `evidence-plan-protocol.md`, `fix-dispatch-strategy.md`.
This demonstrates the pattern's replicability across different pipeline types.

## Directory Structure

```
examples/skills/debug-flow/
├── belt.toml
├── pipeline.yml
├── SKILL.md
└── references/
    ├── done-criteria/
    │   ├── rca.md
    │   ├── fix-plan.md
    │   ├── fix-plan-review.md
    │   ├── execute.md
    │   ├── smoke-test.md
    │   ├── code-review.md
    │   └── test-review.md
    ├── audit-protocol.md
    ├── evidence-plan-protocol.md
    └── fix-dispatch-strategy.md
```

Total: 13 files.

## Pipeline Structure

### Args

```yaml
args:
  e2e:        { type: bool, default: false }     # Enable test-review phase
  smoke:      { type: bool, default: false }     # Enable smoke-test phase
  codex:      { type: bool, default: false }     # Passthrough to sub-pipelines
  ui:         { type: bool, default: false }     # Passthrough to sub-pipelines
  iterations: { type: number, default: 3 }       # N-way voting count
  swarm:      { type: bool, default: false }     # Agent team mode
```

No `doc` arg (unlike feature-dev). Debugging targets existing code;
doc-audit is omitted from the pipeline.

### Phase Map (15 phases)

| # | ID | Type | Description | Gate | Conditional |
|---|---|---|---|---|---|
| 1 | rca | work | Root cause investigation | `file_exists: docs/plans/*-rca-report.md` | — |
| 2 | rca-audit | audit | Audit RCA report | `has_output: true` | — |
| 3 | fix-plan | work | Create fix plan from RCA | `file_exists: docs/plans/*-fix-plan.md` | — |
| 4 | fix-plan-audit | audit | Audit fix plan | `has_output: true` | — |
| 5 | fix-plan-review | sub-pipeline | implementation-review | (from sub-pipeline) | — |
| 6 | fix-plan-review-audit | audit | Audit review completion | `has_output: true` | — |
| 7 | execute | work | TDD implementation | `cmd: make test` | — |
| 8 | execute-audit | audit | Audit implementation | `has_output: true` | — |
| 9 | smoke-test | work | Browser smoke test | `file_exists: smoke-test-report.md` | `when: args.smoke` |
| 10 | smoke-test-audit | audit | Audit smoke test | `has_output: true` | `when: args.smoke` |
| 11 | code-review | sub-pipeline | code-review | (from sub-pipeline) | — |
| 12 | code-review-audit | audit | Audit + regate | `has_output: true` | — |
| 13 | test-review | sub-pipeline | test-review | (from sub-pipeline) | `when: args.e2e` |
| 14 | test-review-audit | audit | Audit + regate | `has_output: true` | `when: args.e2e` |
| 15 | integrate | lite-audit | Merge/PR/branch | — | — |

### Regate Topology

```
code-review-audit:
  regate: [execute, smoke-test]

test-review-audit:
  regate: [execute]
```

Compared to feature-dev: no `doc-audit` in code-review-audit's regate list
(debug-flow has no doc-audit phase).

### Differences from feature-dev

| Aspect | feature-dev | debug-flow |
|--------|-------------|------------|
| Phase 1 | design (brainstorming) | rca (parallel investigation) |
| Phase 2 | spec-review | — (not applicable) |
| Phase 3 | plan | fix-plan (RCA-driven) |
| Doc audit | conditional (--doc) | absent |
| Total phases | 19 | 15 |
| Args | 7 (e2e, smoke, doc, codex, ui, iterations, swarm) | 6 (no doc) |

## SKILL.md

~50 lines. Follows the SKILL.md Authoring Principle: only documents what
pipeline.yml and belt-agent SKILL.md cannot express.

### Dispatch Rules

| config pattern | Action |
|---|---|
| `config.skill: "/systematic-debugging"` | Parallel exploration: dispatch code-explorer, code-architect, impact-analyzer subagents. Synthesize findings into RCA Report. Create worktree. Write reproduction test (must FAIL). Commit RCA Report |
| `config.skill: "/writing-plans"` | Expand RCA Report's Fix Strategy into fix plan at `docs/plans/*-fix-plan.md` |
| `config.skill` (other) | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options |
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory. Follow that SKILL.md's dispatch rules. Runtime args come from top-level pipeline args |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch phase-auditor per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |

### Coordinator Discipline

Unique to debug-flow. The orchestrator owns root cause understanding and
must not delegate synthesis to subagents. Research → Synthesis →
Implementation → Verification flow.

### Evidence Plan

Generated after `rca-audit` completes (not `design-audit` as in feature-dev).
Re-evaluated after `fix-plan-review-audit` if RCA report hash changed.

### Red Flags

- Skip the `rca` phase or start fixing without root cause
- Proceed without a reproduction test
- Delegate root cause synthesis to subagents
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention

## Done Criteria

### rca.md (new)

| ID | Description | Severity |
|---|---|---|
| RCA-01 | RCA Report file exists with 5 required sections (Symptom, Investigation Record, Root Cause, Reproduction Test, Fix Strategy) | blocker |
| RCA-02 | Investigation Record has substantive content in 4 subsections (Code Flow Trace, Architecture Context, Impact Scope, Symmetry Check) | blocker |
| RCA-03 | Impact Scope file paths exist in the codebase | blocker |
| RCA-04 | At least 1 excluded hypothesis recorded with hypothesis, verification method, and rejection reason | blocker |
| RCA-05 | Reproduction test exists and its result is FAIL | blocker |
| RCA-06 | Root Cause contains specific file path, line number, and mechanism | quality |
| RCA-07 | RCA Report is committed to git | blocker |
| RCA-08 | Symmetry Check evaluates asymmetry risk across 4 dimensions | blocker |

Glob pattern: `docs/plans/*-rca-report.md`.
The original debug-flow uses `docs/debug/` but this example uses `docs/plans/`
to align with feature-dev conventions. The path is a pipeline-level concern,
not a belt constraint.

### fix-plan.md (adapted from plan.md)

| ID | Description | Severity | Change from plan.md |
|---|---|---|---|
| FIX-PLAN-01 | Fix plan document file exists | blocker | glob: `*-fix-plan.md` |
| FIX-PLAN-02 | Traceability from **RCA Report Fix Strategy** to tasks | blocker | design requirements → RCA Fix Strategy |
| FIX-PLAN-03 | Task granularity is sub-agent executable | quality | same |
| FIX-PLAN-04 | Task dependencies are explicit and consistent (no cycles) | blocker | same |
| FIX-PLAN-05 | Test cases are specified in Given/When/Then format | blocker | same |
| FIX-PLAN-06 | Fix plan document is committed to git | blocker | same |

### fix-plan-review.md (adapted from plan-review.md)

| ID | Description | Severity | Change from plan-review.md |
|---|---|---|---|
| FIX-PLAN-REVIEW-01 | Review executed across all 3 perspectives | quality | same |
| FIX-PLAN-REVIEW-02 | All consensus findings are resolved | quality | same |
| FIX-PLAN-REVIEW-03 | Fix plan and **RCA Report** are consistent | blocker | design document → RCA Report |
| FIX-PLAN-REVIEW-04 | Each task's completion condition is verifiably specified | blocker | same |

### execute.md (adapted from feature-dev execute.md)

| ID | Description | Severity | Change from feature-dev |
|---|---|---|---|
| EXECUTE-01 | Code changes exist for every task | blocker | same |
| EXECUTE-02 | Build/compilation succeeds | blocker | same |
| EXECUTE-03 | No lint or type-check errors | blocker | same |
| EXECUTE-04 | Full test suite passes | blocker | same |
| EXECUTE-05 | Test code exists for every planned test case | blocker | same |
| EXECUTE-06 | Implementation respects component boundaries | quality | references RCA Report Impact Scope |
| EXECUTE-07 | End-to-end traceability from **RCA → fix plan → implementation** | blocker | 3-tier: RCA → fix-plan → code |
| EXECUTE-08 | Newly added tests are not tautological | blocker | same |
| EXECUTE-09 | Test cases cover both requirement coverage and impact scope | blocker | references RCA Report |

### smoke-test.md (same as feature-dev)

Identical content. 4 criteria: SMOKE-TEST-01 through SMOKE-TEST-04.

### code-review.md (same as feature-dev)

Identical content. 4 criteria: CODE-REVIEW-01 through CODE-REVIEW-04.

### test-review.md (adapted from feature-dev test-review.md)

| ID | Description | Severity | Change from feature-dev |
|---|---|---|---|
| TEST-REVIEW-01 | Review executed across all 3 perspectives | quality | same |
| TEST-REVIEW-02 | All user-approved findings have been fixed | blocker | same |
| TEST-REVIEW-03 | All **RCA Report** test perspectives are covered by test code | blocker | design doc → RCA Report |

## References

### audit-protocol.md

Identical to feature-dev. Defines phase-auditor dispatch, Audit Context template,
verdict JSON format, PASS/FAIL rules, and failure handling.

### evidence-plan-protocol.md

Adapted from feature-dev:

| Event | feature-dev | debug-flow |
|-------|-------------|------------|
| Generation trigger | `design-audit` PASS | `rca-audit` PASS |
| Re-evaluation trigger | `plan-review-audit` PASS if design hash changed | `fix-plan-review-audit` PASS if RCA report hash changed |
| Source document | design doc | RCA Report |
| Activities | implementation, review, smoke-test, doc-maintenance | implementation, review, smoke-test (no doc-maintenance) |

### fix-dispatch-strategy.md

Adapted dispatch table:

| Work Phase | Fix Executor | Strategy |
|------------|-------------|----------|
| rca | Orchestrator | Re-scan codebase, re-run exploration agents for missing info |
| fix-plan | Orchestrator | Edit fix plan doc directly based on audit findings |
| fix-plan-review | Orchestrator | Edit fix plan doc directly based on audit findings |
| execute | `feature-implementer` subagent | Decompose fix instructions into TDD tasks |
| smoke-test | `feature-implementer` subagent | Bug fixes to implementation code |
| code-review | `feature-implementer` subagent | Apply review finding fixes |
| test-review | `feature-implementer` subagent | Apply test code fixes |

## What Changes

| File | Action |
|------|--------|
| `examples/skills/debug-flow/belt.toml` | New |
| `examples/skills/debug-flow/pipeline.yml` | New |
| `examples/skills/debug-flow/SKILL.md` | New |
| `examples/skills/debug-flow/references/done-criteria/*.md` (7 files) | New |
| `examples/skills/debug-flow/references/audit-protocol.md` | New |
| `examples/skills/debug-flow/references/evidence-plan-protocol.md` | New |
| `examples/skills/debug-flow/references/fix-dispatch-strategy.md` | New |

Total: 13 new files.

## What Does NOT Change

| File | Reason |
|------|--------|
| `examples/skills/feature-dev/*` | Independent example, no coupling |
| `examples/skills/code-review/*` | Consumed via `uses:`, no modification needed |
| `examples/skills/test-review/*` | Consumed via `uses:`, no modification needed |
| `examples/skills/implementation-review/*` | Consumed via `uses:`, no modification needed |
| `examples/skills/smoke-test/*` | Consumed via `uses:`, no modification needed |
| `crates/*` | No belt-core changes required |

## Verification

1. `belt lint examples/skills/debug-flow/pipeline.yml` passes
2. Sub-pipeline references (`../code-review/pipeline.yml`, etc.) resolve correctly
3. `when` conditions, `regate` targets, and `max_retries` are valid
4. Done-criteria files are referenced consistently from pipeline.yml config
