---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving belt-agent CLI — a deterministic state machine for pipeline execution.

## Commands

```bash
# Start a new run
belt-agent init <pipeline.yml>
belt-agent init <pipeline.yml> --arg smoke=true --arg team=BELT

# Get current phase info (or completion signal)
belt-agent next
belt-agent next --run <run_id>

# Run gate checks for current phase
belt-agent verify
belt-agent verify --run <run_id>

# Advance to next phase
belt-agent step
belt-agent step --confirm          # required when phase has validate criteria
belt-agent step --run <run_id>

# Inspect full run state (enriched view)
belt-agent status
belt-agent status --run <run_id>
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → execute phase → verify (if gates) → step → next → ... → completed
```

1. `init` starts a run and returns the first active phase.
2. `next` returns the current phase info. If `completed: true`, the pipeline is done.
3. Execute the phase work (see `config.skill`).
4. If the phase has `gate`, run `verify`. On FAIL, fix and re-verify.
5. `step` to advance. If `advanced: false`, check the `reason` field.
6. Repeat from `next`.

## Decision Rules

| Situation | Action |
|-----------|--------|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. |
| `verify` FAIL with `regate` phases | Fix **those regate phases' work** (not the current phase). Re-run `verify`. |
| Phase has `validate` criteria | Verify each criterion yourself, then `step --confirm`. |
| `step` returns `advanced: false` | Read `reason`. Typically `confirmation_required` — verify criteria and retry with `--confirm`. |
| Need pipeline state overview | Run `status`. Use for context recovery, progress checks. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML, and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "attempt": 1, "outputs": ["report.json"] },
    { "id": "review", "status": "current", "verify_passed": false, "attempt": 2, "outputs": [] },
    { "id": "test", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] },
    { "id": "deploy", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] }
  ]
}
```

Use `status` to understand pipeline state after context recovery or before resuming work.
When pipeline completes, `status` is `"completed"` and `current_phase` is `null`.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Only the following key has a defined meaning in this protocol:

| Key | Type | Meaning |
|-----|------|---------|
| `config.skill` | `string` | Skill to invoke for this phase. |

### Rules

- Unknown config keys MAY be ignored (forward compatibility).
- Pipeline-specific skills MAY add custom keys freely (belt does not interpret them).
- Dispatch implementation (which agents to launch, how to execute) is the
  pipeline-specific skill's responsibility.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase, you MUST NOT run
`belt-agent step --confirm` without verifying each criterion.

validate is a list of criteria that belt returns for LLM judgment.
belt cannot know whether these criteria were actually evaluated.
The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>
