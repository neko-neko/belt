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

# Run regate checks for target phases
belt-agent regate
belt-agent regate --run <run_id>

# Advance to next phase
belt-agent step
belt-agent step --confirm
belt-agent step --run <run_id>

# Inspect full run state (enriched view)
belt-agent status
belt-agent status --run <run_id>
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → execute → verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Decision Rules

| Situation | Action |
|-----------|--------|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` (verify clears regate state). |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself, then `step --confirm`. |

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|----------|--------|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML, and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "attempt": 1, "outputs": ["report.json"] },
    { "id": "review", "status": "current", "verify_passed": false, "attempt": 2, "outputs": [] }
  ]
}
```

Use `status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.

| Key | Type | Meaning |
|-----|------|---------|
| `config.skill` | `string` | Skill to invoke for this phase. |

Unknown config keys MAY be ignored. Pipeline-specific skills MAY add custom keys freely.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase, you MUST NOT run
`belt-agent step --confirm` without verifying each criterion.

validate is a list of criteria that belt returns for LLM judgment.
belt cannot know whether these criteria were actually evaluated.
The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>
