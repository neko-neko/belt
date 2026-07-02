---
name: monkey-test-supplement
description: >-
  verify stage only. Read BEFORE invoking /belt:monkey-test to resolve the
  scenarios source, inject prior-phase artifacts as interpretation hints, and
  fix output paths.
---

# Monkey-Test Supplement for the verify stage

Read BEFORE invoking `/belt:monkey-test`. Path convention reference:
`plugins/belt/skills/design/references/path-convention.md`.

## Scenarios Source

- Feature runs: `docs/features/<topic>/scenarios.yml`
- Bug runs: `docs/features/<topic>/rca-scenarios.yml`

Resolve whichever exists for the current topic. On glob collision (multiple
topics), select the most recently modified (mtime DESC). If neither exists,
monkey-test cannot run — pause and report per the verify SKILL.md Entry Check.

## Hint Inputs (read those that exist into context)

- `docs/features/<topic>/design.md`
  - Resolve ambiguity in scenarios' natural-language Given/When/Then
    (e.g., "valid email" -> the exact validation rule from design).
  - Use Impact Analysis to predict likely regressions.
- `docs/features/<topic>/test-strategy.md`
  - Use `category`/`severity` from each scenario's matching strategy entry
    to set the failure severity in results.
- `docs/features/<topic>/plan.md` (or `fix-plan.md`)
  - Use to decide `SKIP` verdicts: if a scenario targets a feature whose
    implementing task is incomplete, SKIP with a reason.
- `docs/features/<topic>/rca-report.md` (bug runs)
  - The first scenario in `rca-scenarios.yml` corresponds to the RCA
    Reproduction Test. After the fix it is expected to PASS (it FAILed
    pre-fix). `criteria/monkey-test.md` MONKEY-TEST-04 verifies this
    transition. Subsequent scenarios cover Symmetry pairs and Impact Scope
    regressions from the RCA report.

## Output Paths

- `docs/features/<topic>/monkey-test-report.md` — human-readable
- `docs/features/<topic>/monkey-test-results.json` — machine-readable
- `docs/features/<topic>/monkey-test-screenshots/` — step screenshots

## Behavior

1. Parse the scenarios source; collect `id`, `given`, `when`, `then`, `severity`.
2. For each scenario:
   a. Determine SKIP if an associated plan task is incomplete.
   b. Launch agent-browser (restore auth-state if present).
   c. Interpret `given` -> navigate/setup; `when` -> actions; `then` ->
      assertions. Resolve ambiguity via `design.md` / `rca-report.md`.
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

## Completion Criteria (for the monkey-test gate)

- Both output files exist and are committed.
- Every scenario id in the source file is present in `results.json.scenarios`.
- `results.json` validates against the schema above.
- Every FAIL with severity `critical` or `high` is surfaced in the primary
  section of `monkey-test-report.md`.
- Bug runs: the first (reproduction) scenario PASSes.
