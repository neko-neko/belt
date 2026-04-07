# Evidence Plan Protocol

## Overview

The Evidence Plan defines what evidence must be collected during pipeline execution
to support audit decisions. It is generated once and updated as the RCA evolves.

## Lifecycle

| Event | Action |
|-------|--------|
| `rca-audit` PASS | Generate Evidence Plan |
| `fix-plan-review-audit` PASS | Re-evaluate if RCA report hash changed since generation |
| `execute` and later phases | Inject collection requirements into executor prompts |

## Generation

After `rca-audit` passes, the orchestrator generates the Evidence Plan by analyzing:

1. The RCA Report's Root Cause, Impact Scope, and Fix Strategy sections
2. The done-criteria for all upcoming phases
3. Project characteristics (language, framework, UI presence, API presence)

The plan is written to the `rca-audit` output directory.

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
      "phases": ["fix-plan-review", "code-review", "test-review"],
      "collect": ["review findings JSON", "consensus findings count", "applied fixes"]
    },
    {
      "type": "smoke-test",
      "phases": ["smoke-test"],
      "collect": ["smoke-test-report.md", "screenshots", "flaky test list"]
    }
  ]
}
```

## Injection

When dispatching a work phase executor, include the relevant collection requirements:

> "In addition to the phase work, collect the following evidence and write to the output directory:
> {list from Evidence Plan for this phase's activity type}"

The auditor verifies that required evidence was actually collected.
