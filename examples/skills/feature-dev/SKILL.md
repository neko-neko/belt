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

| config pattern | Action |
|---|---|
| `config.skill` present | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options (resolved from `args`) |
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory (resolved from `uses:` path in pipeline.yml). Follow that SKILL.md's dispatch rules for the current sub-phase. Runtime args (`codex`, `iterations`, `swarm`, `ui`) come from top-level pipeline args |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch `phase-auditor` subagent per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |
| `config.audit: "lite"` | Orchestrator directly evaluates `validate` criteria. `step --confirm` after user chooses integration method |

## Evidence Plan

Generated after `design-audit` completes. Re-evaluated after `plan-review-audit`
if design hash changed. Details: `references/evidence-plan-protocol.md`.

## Red Flags — NEVER DO

- Skip the `design` phase
- Auto-answer brainstorming design questions
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention
