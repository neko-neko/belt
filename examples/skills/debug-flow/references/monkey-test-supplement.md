# Monkey Test Supplement (Phase 6 override for `/monkey-test`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 6 (INVOKE 1 = Read this file; INVOKE 2 = `/monkey-test`). Only runs when `args.e2e=true`.

## Scenarios source

`/monkey-test` defaults to `docs/features/*/scenarios.yml` (feature-dev). In debug-flow, override to:

```
docs/plans/*-rca-scenarios.yml
```

## Glob collision handling

If multiple files match the `docs/plans/*-rca-scenarios.yml` glob (concurrent runs / multiple bugs same day), select the **most recently modified** (mtime DESC).

## First scenario requirement

The first scenario in `rca-scenarios.yml` MUST correspond to the RCA Reproduction Test (from Phase 1). After fix, this scenario is expected to PASS (previously FAIL per RCA-05). `criteria/monkey-test.md` MONKEY-TEST-03 verifies this transition.

## Regression scenarios

Supplement subsequent scenarios cover:
- Symmetry pair validation (per RCA-08): if the RCA identified paired paths, add scenarios exercising them
- Impact Scope regression: scenarios exercising adjacent functionality that shares code paths with the fix

## Output paths

Produce:

```
docs/plans/YYYY-MM-DD-<topic>-monkey-test-report.md
docs/plans/YYYY-MM-DD-<topic>-monkey-test-results.json
```

Report structure: scenario list, per-scenario result (PASS/FAIL/SKIP with rationale), summary count, flaky detection notes (optional).
