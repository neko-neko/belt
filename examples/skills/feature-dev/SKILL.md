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

## Belt-Agent Loop

```
belt-agent init pipeline.yml [--smoke=true ...] → loop:
  1. belt-agent next           → phase info (JSON)
  2. Dispatch (see below)      → do the work
  3. belt-agent verify         → run gate checks
  4. belt-agent regate         → run regate targets (if any)
  5. belt-agent step [--confirm] → advance to next phase
```

## Phase Dispatch Rules

### Work phases (config.skill present)

Invoke the skill specified in `config.skill`. Pass other config keys
(`codex`, `iterations`, `swarm`, `ui`) as options to the skill invocation.

Example: if `belt-agent next` returns:
```json
{
  "phase": { "config": { "skill": "/code-review", "codex": "args.codex", "iterations": "args.iterations" } }
}
```
Then invoke `/code-review` with `--codex` and `--iterations` flags as indicated by
the resolved arg values in the `args` field of the response.

### Audit phases (config.audit == "required")

1. Read `references/done-criteria/{config.criteria}.md`
2. Dispatch a `phase-auditor` subagent following `references/audit-protocol.md`
3. Write `verdict.json` to the phase's `output_dir`
4. Run `belt-agent verify` (the `has_output: true` gate checks the file exists)
5. If verdict is PASS: `belt-agent step --confirm`
6. If verdict is FAIL: apply fix per `references/fix-dispatch-strategy.md`, then re-audit

### Integrate phase (audit: lite)

No separate audit phase. The orchestrator directly evaluates the `validate` criteria
and runs `belt-agent step --confirm` after user chooses integration method.

## Evidence Plan

Generated after `design-audit` completes. Re-evaluated after `plan-review-audit`
if design hash changed. Details: `references/evidence-plan-protocol.md`.
Collection requirements are injected into executor prompts for `execute` and later phases.

## Red Flags — NEVER DO

- Skip the `design` phase
- Auto-answer brainstorming design questions
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Pass an audit phase with only `belt-agent verify --passed` (auditor dispatch is mandatory)
- Proceed past a FAIL verdict without fix + re-audit or user intervention
