---
name: monkey-test-supplement
description: >-
  feature-dev Phase 6 only. Read BEFORE invoking /monkey-test to inject
  design/plan/test-strategy as interpretation hints and to fix output paths.
---

# Monkey-Test Supplement for feature-dev

Read BEFORE invoking `/monkey-test` in Phase 6. This phase only runs when
`args.e2e` is true. Path convention reference: `./path-convention.md`.

## Primary Input

- `docs/features/<topic>/scenarios.yml` — the scripted scenarios (required)

## Hint Inputs (read these into context)

- `docs/features/<topic>/design.md`
  - Use to resolve ambiguity in scenarios' natural-language Given/When/Then
    (e.g., "valid email" → use the exact validation rule from design).
  - Use Impact Analysis to predict likely regressions.
- `docs/features/<topic>/test-strategy.md`
  - Use `category`/`severity` from each scenario's matching strategy entry
    to set the failure severity in results.
- `docs/features/<topic>/plan.md`
  - Use to decide `SKIP` verdicts: if a scenario targets a feature whose
    implementing task is marked incomplete in the plan, SKIP the scenario
    and note the reason.

## Output Paths

- `docs/features/<topic>/monkey-test-report.md` — human-readable
- `docs/features/<topic>/monkey-test-results.json` — machine-readable

## Behavior

1. Parse `scenarios.yml`; collect `id`, `given`, `when`, `then`, `severity`.
2. For each scenario:
   a. Determine SKIP if an associated plan task is incomplete.
   b. Launch agent-browser (restore auth-state if present).
   c. Interpret `given` → navigate/setup; `when` → actions; `then` →
      assertions. Resolve ambiguity via `design.md`.
   d. Capture a screenshot at each step (save under
      `docs/features/<topic>/monkey-test-screenshots/` — create if missing).
   e. Record result.
3. After all scenarios, write both outputs.

## results.json Schema

```json
{
  "scenarios": [
    {
      "id": "string",
      "status": "PASS | FAIL | SKIP",
      "severity": "critical | high | medium | low",
      "duration_ms": 1234,
      "error": "string (only when FAIL)",
      "skip_reason": "string (only when SKIP)",
      "screenshots": ["docs/features/<topic>/monkey-test-screenshots/<id>-step1.png", "..."]
    }
  ],
  "summary": {
    "total": 10,
    "passed": 8,
    "failed": 1,
    "skipped": 1
  }
}
```

## Completion Criteria (for Phase 6 gate)

- Both output files exist and are committed.
- Every scenario id in `scenarios.yml` is present in `results.json.scenarios`.
- `results.json` validates against the schema above.
- Every FAIL with severity `critical` or `high` is surfaced in the primary
  section of `monkey-test-report.md`.
