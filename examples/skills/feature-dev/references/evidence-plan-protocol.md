# Evidence Plan Protocol

## Overview

The Evidence Plan defines what evidence must be collected during pipeline execution
to support audit decisions. It is generated once and updated as the design evolves.

## Lifecycle

| Event | Action |
|-------|--------|
| `design-audit` PASS | Generate Evidence Plan |
| `plan-review-audit` PASS | Re-evaluate if design doc hash changed since generation |
| `execute` and later phases | Inject collection requirements into executor prompts |

## Generation

After `design-audit` passes, the orchestrator generates the Evidence Plan by analyzing:

1. The design document's requirements and test perspectives
2. The done-criteria for all upcoming phases
3. Project characteristics (language, framework, UI presence, API presence)

The plan is written to the `design-audit` output directory.

## Structure

```json
{
  "project_type": "rust-cli | web-frontend | api-backend | ...",
  "has_ui": false,
  "has_api": false,
  "activities": [
    {
      "type": "implementation",
      "phases": ["execute"],
      "collect": ["build output", "test results", "lint results", "coverage report"]
    },
    {
      "type": "review",
      "phases": ["spec-review", "plan-review", "code-review", "test-review"],
      "collect": ["review findings JSON", "consensus findings count", "applied fixes"]
    },
    {
      "type": "smoke-test",
      "phases": ["smoke-test"],
      "collect": ["smoke-test-report.md", "screenshots", "flaky test list"]
    },
    {
      "type": "doc-maintenance",
      "phases": ["doc-audit"],
      "collect": ["doc-audit report", "broken deps count", "stale signals"]
    }
  ]
}
```

## Injection

When dispatching a work phase executor, include the relevant collection requirements:

> "In addition to the phase work, collect the following evidence and write to the output directory:
> {list from Evidence Plan for this phase's activity type}"

The auditor verifies that required evidence was actually collected.
