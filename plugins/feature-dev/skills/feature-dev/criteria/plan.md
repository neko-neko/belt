---
name: plan-done-criteria
audit: lite
phase: plan
---

# Phase 3 (plan) Done Criteria

- **PLAN-01**: `docs/features/<topic>/plan.md` exists and is committed.
- **PLAN-02**: `plan.md` contains the required header:
  `Goal`, `Architecture`, `Tech Stack`, and at least one `Task N` section.
- **PLAN-03**: Every `Task` follows the TDD shape (failing test → minimal
  implementation → passing test → commit) with explicit code and commands
  per step.
- **PLAN-04**: Every item in `design.md`'s `Must-Verify Checklist` is cited
  by at least one task (cite ID, e.g. `MV-01`).
- **PLAN-05**: No placeholder language remains (no `TBD`, `TODO`,
  `add appropriate error handling`, `similar to Task N`, or unresolved
  types/functions).
- **PLAN-06**: When `args.e2e` is true, every `scenarios.yml` `id` is
  cited by at least one task.
- **PLAN-07**: Every input parameter surfaced in `test-strategy.md` has
  Given/When/Then coverage for: normal, boundary, abnormal, state-transition.

- **PLAN-08**: Narrative note at `.belt/runs/<run_id>/notes/phase-plan.md` exists,
  contains frontmatter (`phase: plan`, `run_id: <run_id>`), and all 4 required
  sections. Decisions records task decomposition rationale and granularity choices.
  Directives records constraints for execute phase (e.g. commit granularity rules,
  test-first enforcement). See `plugins/belt-agents/references/narrative-convention.md`.
