---
name: feature-dev
description: >-
  Quality-gated development orchestrator. Drives a 10-phase pipeline
  (design → review → plan → review → execute → doc-audit → smoke-test →
  code-review → test-review → integrate) via belt-agent CLI.
  Conditional phases: --e2e (test-review), --smoke (smoke-test), --doc (doc-audit).
  Passthrough flags: --codex, --ui, --iterations N, --swarm.
user-invocable: true
argument-hint: "[--e2e] [--smoke] [--doc] [--codex] [--ui] [--iterations N] [--swarm]"
---

# Feature Dev Orchestrator

Quality-gated development pipeline driven by belt-agent.
belt handles phase transitions, gates, regate, and conditional skipping.
The orchestrator dispatches skills per phase and auditor agents per audit.

## Dispatch Rules

| invoke variant | Orchestrator action |
|---|---|
| `skill:` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as arguments. |
| `pipeline:` | Initialise a nested `belt-agent` run on the referenced sub-pipeline (`invoke.pipeline`) with `invoke.with` as args. |

Phase validation is driven by `validate:` on each phase. When `validate` is
a file reference (e.g., `./criteria/design.md`), read the file and judge
each criterion defined inside it before calling `belt-agent step --confirm`.
When `validate` is a list of inline strings, judge each string directly.
Both forms use the same `phase-auditor` subagent by convention; see
`../../references/audit-protocol.md`.

## Evidence Plan

Generated after `design-audit` completes. Re-evaluated after `plan-review-audit`
if design hash changed. Details: `references/evidence-plan-protocol.md`.

## Red Flags — NEVER DO

- Skip the `design` phase
- Auto-answer brainstorming design questions
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention
