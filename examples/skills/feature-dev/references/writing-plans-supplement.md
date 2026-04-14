---
name: writing-plans-supplement
description: >-
  feature-dev Phase 3 only. Read BEFORE invoking superpowers:writing-plans to
  override input paths, output path, and test-case expansion rules.
---

# Writing-Plans Supplement for feature-dev

Read BEFORE invoking `/writing-plans` in Phase 3. Path convention reference:
`./path-convention.md`.

## Input Paths (read these)

- `docs/features/<topic>/design.md` — the feature design
- `docs/features/<topic>/test-strategy.md` — test perspectives + strategy
- `docs/features/<topic>/scenarios.yml` — when `args.e2e` is true

## Output Path Override

Write the plan to:

```
docs/features/<topic>/plan.md
```

This overrides writing-plans' default `docs/superpowers/plans/` location.

## Plan Content Requirements

Beyond the standard writing-plans output:

1. **Test-case integration**: Every task that implements a feature requirement
   MUST reference at least one entry from `test-strategy.md` and, when e2e is
   enabled, at least one `scenarios.yml` `id` that exercises the feature.

2. **Must-Verify Checklist mapping**: Every item in the design's
   Must-Verify Checklist MUST map to at least one task that verifies it
   (cite the item ID in the task).

3. **Given/When/Then expansion**: For every input parameter surfaced in
   `test-strategy.md`, include Given/When/Then tests covering the four
   categories (normal, boundary, abnormal, state-transition).

4. **No placeholders**: per the writing-plans standard — no TBD/TODO/"add
   appropriate error handling". Show actual code in every step.

## When args.e2e is true

- Each scenario in `scenarios.yml` must be referenced by at least one plan
  task (either implementation or verification).
- Plan tasks that produce UI-bearing code must include a step that verifies
  the UI can be reached by the corresponding `scenarios.yml` scenario's
  `given` preconditions.

## Completion Criteria (for Phase 3 gate)

- `docs/features/<topic>/plan.md` exists, committed in the worktree.
- All four plan-content requirements above are satisfied.
- Plan links every Must-Verify Checklist item to at least one task.
- When e2e: plan links every `scenarios.yml` `id` to at least one task.
