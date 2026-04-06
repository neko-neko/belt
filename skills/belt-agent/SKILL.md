---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Generic protocol for driving the belt-agent CLI. Defines how LLM agents interact
with belt's deterministic state machine — the command loop, response interpretation,
and safety constraints.

## Protocol Loop

```
belt-agent init <pipeline.yml> [--arg key=value ...]
  |
loop {
  phase = belt-agent next [--run <id>]

  if phase.completed:
    break                        # pipeline complete

  execute(phase)                 # LLM executes the phase (see config)

  if phase.gate is not empty:
    result = belt-agent verify [--run <id>]

    while result.verdict == "FAIL":
      fix(result)                # LLM fixes failing gates
      result = belt-agent verify

  if phase.confirm or phase.validate is not empty:
    belt-agent step --confirm [--run <id>]
  else:
    belt-agent step [--run <id>]
}
```

### Rules

- `next` returns all phase information: description, config, artifacts, gate, validate,
  confirm, regate, max_retries, attempt, and args.
- Phases with gates require a `verify` PASS before `step`.
- Phases without gates (confirm-only, etc.) skip `verify` and go directly to `step`.
- `step` success is determined by the `advanced` field in the JSON response.
- `status` can be called at any time to inspect the full run state.
- `--run <id>` is optional on all commands; omit it to use the latest run.

## Command Response Handling

### init

Starts a new run. Returns the first active phase (after `when:` evaluation).

```json
{
  "run_id": "019d6195-...",
  "pipeline": "feature-dev",
  "phase": {
    "id": "design/explore",
    "description": "...",
    "config": {},
    "artifacts": [".belt/exploration/*.json"],
    "output_dir": ".belt/runs/<run_id>/design_explore"
  },
  "gate": [{ "file_exists": ".belt/exploration/*.json" }],
  "validate": [],
  "confirm": false,
  "max_retries": 0,
  "attempt": 0,
  "args": {}
}
```

### next

Returns current phase information, or signals pipeline completion.

| Response | Action |
|----------|--------|
| `completed: true` | Exit loop. Pipeline complete. |
| `phase` present | Read phase info, begin execution. |

### verify

Runs gate checks and returns a verdict.

```json
{
  "run_id": "...",
  "phase": "design/explore",
  "verdict": "PASS",
  "checks": [
    {
      "check_type": "file_exists",
      "passed": true,
      "detail": "matched 3 files",
      "duration_ms": null
    }
  ],
  "attempt": 1,
  "max_retries": 0
}
```

| Response | Action |
|----------|--------|
| `verdict: "PASS"` | Gate passed. Proceed to `step`. |
| `verdict: "FAIL"` | Read `checks`, fix failing gates, re-run `verify`. |

### step

Advances to the next phase.

| Response | Action |
|----------|--------|
| `advanced: true`, `to` present | Transition succeeded. Call `next`. |
| `advanced: true`, `completed: true` | Pipeline complete. |
| `advanced: false`, `reason: "confirmation_required"` | `--confirm` needed. Verify validate criteria first, then retry with `--confirm`. |

### status

Returns the full run state. Can be called at any time.

```json
{
  "run_id": "...",
  "pipeline": "feature-dev",
  "pipeline_file": "examples/feature-dev/pipeline.yml",
  "current_phase": "design/synthesize",
  "completed_phases": ["design/explore"],
  "skipped_phases": [],
  "phase_attempts": { "design/explore": 1 },
  "args": {},
  "created_at": "...",
  "updated_at": "..."
}
```

### Regate

When `next` returns a `regate` field containing phase IDs, `verify` re-checks gates
for those phases in addition to the current phase.

- If a regate target's gate fails, fix **that phase's work** (not the current phase).
- Repeat verify -> fix until all regate targets and the current phase pass.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase, you MUST NOT run
`belt-agent step --confirm` without verifying each criterion.

validate is a list of criteria that belt returns for LLM judgment.
belt cannot know whether these criteria were actually evaluated.
The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>

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
