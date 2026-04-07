---
name: debug-flow
description: >-
  Quality-gated debugging orchestrator. Drives an 8-phase pipeline
  (rca → fix-plan → fix-plan-review → execute → smoke-test →
  code-review → test-review → integrate) via belt-agent CLI.
  Conditional phases: --e2e (test-review), --smoke (smoke-test).
  Passthrough flags: --codex, --ui, --iterations N, --swarm.
user-invocable: true
argument-hint: "[--e2e] [--smoke] [--codex] [--ui] [--iterations N] [--swarm]"
---

# Debug Flow Orchestrator

Quality-gated debugging pipeline driven by belt-agent.
belt handles phase transitions, gates, regate, and conditional skipping.
The orchestrator investigates root cause, plans fixes, and drives
implementation through quality gates.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `config.skill: "/systematic-debugging"` | Parallel exploration: dispatch code-explorer, code-architect, impact-analyzer subagents. Synthesize findings into RCA Report. Create worktree. Write reproduction test (must FAIL). Commit RCA Report |
| `config.skill: "/writing-plans"` | Expand RCA Report's Fix Strategy into fix plan at `docs/plans/*-fix-plan.md` |
| `config.skill` (other) | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options (resolved from `args`) |
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory (resolved from `uses:` path in pipeline.yml). Follow that SKILL.md's dispatch rules for the current sub-phase. Runtime args (`codex`, `iterations`, `swarm`, `ui`) come from top-level pipeline args |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch `phase-auditor` subagent per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |

## Coordinator Discipline

The orchestrator owns understanding of root cause and fix strategy.
Do NOT delegate synthesis to subagents.

- Research → Synthesis → Implementation → Verification
- After receiving exploration results, reconstruct root cause yourself
- Parallel exploration for read-only investigation; serialize write operations

## Evidence Plan

Generated after `rca-audit` completes. Re-evaluated after `fix-plan-review-audit`
if RCA report hash changed. Details: `references/evidence-plan-protocol.md`.

## Red Flags — NEVER DO

- Skip the `rca` phase or start fixing without root cause
- Proceed without a reproduction test
- Delegate root cause synthesis to subagents
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention
