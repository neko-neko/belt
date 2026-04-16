# Audit Protocol

## Overview

Each audit phase dispatches a `phase-auditor` subagent to independently verify
the preceding work phase against its done-criteria. This protocol defines the
dispatch procedure, verdict format, and failure handling.

## Auditor Dispatch

When `belt-agent next` returns a phase with `config.audit == "required"`:

1. Read `references/done-criteria/{config.criteria}.md`
2. Compose the Audit Context (see template below)
3. Launch a `phase-auditor` subagent via the Agent tool
4. Validate the returned JSON (must have all required fields)
5. Write the verdict to `{output_dir}/verdict.json`

If the JSON is invalid, retry once. If still invalid, PAUSE.

## Audit Context Template

Inject the following into the phase-auditor prompt:

```
## Audit Context

### Phase
name: {criteria name from config}
attempt: {current attempt number, from belt-agent next response}

### Done Criteria
{full content of references/done-criteria/{criteria}.md}

### Artifacts to Verify
- primary: {artifacts from the work phase}
- dependencies: {artifacts from prior phases referenced by done-criteria}

### Cumulative Diagnosis (attempt 2+ only)
{previous verdict(s) and their fail details, so the auditor knows what was already tried}
```

## Verdict Format

The phase-auditor must return JSON in this structure:

```json
{
  "verdict": "PASS | FAIL",
  "criteria_results": [
    {
      "id": "CRITERIA-01",
      "passed": true,
      "severity": "blocker",
      "detail": "Primary artifact verified at expected path"
    }
  ],
  "summary": {
    "total": 7,
    "passed": 7,
    "failed": 0,
    "blocking_issues": [],
    "quality_warnings": ["CRITERIA-05: section could be more detailed"]
  },
  "observations": [
    {
      "type": "quality",
      "content": "Artifact is well-structured but some sections could be more detailed"
    }
  ],
  "escalation": null
}
```

### Required fields
- `verdict`: "PASS" or "FAIL"
- `criteria_results`: array with one entry per criterion
- `summary`: counts + blocking issues + quality warnings
- `observations`: array (may be empty, but field must exist)
- `escalation`: null or object with `reason` and `recommendation`

## Verdict Rules

- **PASS**: All `blocker` criteria pass. Quality warnings are reported but don't block.
- **FAIL**: At least one `blocker` criterion fails.
- **FAIL with escalation**: The auditor identifies a fundamental issue that retries cannot fix (e.g., design is fundamentally flawed). Set `escalation` to a non-null object. This triggers an immediate PAUSE regardless of remaining retries.

## Failure Handling

When verdict is FAIL (no escalation):
1. Extract `fix_instruction` from each failed criterion's detail
2. Apply the fix (see individual skill SKILL.md Fix sections for workflow-specific dispatch rules)
3. Re-run `belt-agent verify` to confirm output still exists
4. The orchestrator re-dispatches the auditor (attempt increments automatically via belt)

When `max_retries` (3) is exhausted:
1. Compile a cumulative diagnosis from all attempts
2. PAUSE and present the diagnosis to the user
3. User intervention resets the attempt counter

## PAUSE Recovery

After user intervenes and instructs to continue:
1. belt's `max_retries` counter has been exhausted — user must acknowledge
2. Apply any user-directed fixes
3. Re-run the audit from the beginning (the orchestrator manages this via belt-agent)
