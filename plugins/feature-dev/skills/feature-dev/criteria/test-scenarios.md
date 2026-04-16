---
name: test-scenarios-done-criteria
audit: lite
phase: test-scenarios
---

# Phase 2 (test-scenarios) Done Criteria

- **TEST-01**: `docs/features/<topic>/test-strategy.md` exists and is committed.
- **TEST-02**: `test-strategy.md` contains sections:
  - `Test Design Techniques` (ISTQB-based: equivalence partitioning,
    boundary-value analysis, decision tables, state transitions)
  - `Quality Characteristics` (ISO 25010-based: functional suitability,
    performance efficiency, compatibility, usability, reliability, security,
    maintainability, portability)
  - `Priority Matrix` mapping characteristics to criticality
- **TEST-03**: Every item in `design.md`'s `Must-Verify Checklist` has at
  least one corresponding entry in `test-strategy.md` (verified by ID cross-
  reference).
- **TEST-04**: When `args.e2e` is true:
  - `docs/features/<topic>/scenarios.yml` exists, committed.
  - Contains at least 3 scenarios.
  - Every scenario has: `id` (kebab-case), `category`, `severity`
    (`critical|high|medium|low`), `given`, `when`, `then`.
  - `preconditions` and `postconditions` are present when applicable.
- **TEST-05**: `test-strategy.md` includes at least one non-functional
  requirement (performance, security, or accessibility) with a concrete
  acceptance criterion.
