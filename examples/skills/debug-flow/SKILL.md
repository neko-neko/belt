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

| invoke variant | Orchestrator action |
|---|---|
| `skill:` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as arguments. |
| `pipeline:` | Initialise a nested `belt-agent` run on the referenced sub-pipeline (`invoke.pipeline`) with `invoke.with` as args. |

Phase validation is driven by `validate:` on each phase. When `validate` is
a file reference (e.g., `./criteria/rca.md`), read the file and judge
each criterion defined inside it before calling `belt-agent step --confirm`.
When `validate` is a list of inline strings, judge each string directly.
Both forms use the same `phase-auditor` subagent by convention; see
`../../references/audit-protocol.md`.

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
