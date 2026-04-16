---
name: spec-review-done-criteria
audit: lite
phase: spec-review
---

# Phase 3 (spec-review) Done Criteria

- **SREV-01**: The required sections of `docs/features/<topic>/test-strategy.md`
  (`Test Design Techniques` / `Quality Characteristics` / `Priority Matrix`)
  remain intact after spec-review (structural parity with TEST-02).
- **SREV-02**: Triage of spec-review findings is complete
  (both the grill-me group and the selection group are processed, with no unhandled findings left).
- **SREV-03**: Only user-approved findings are reflected in `test-strategy.md` / `scenarios.yml`
  (grill-me: `accept` or `accept_current` only; selection: only entries the user picked by number).
- **SREV-04**: Unapproved findings (grill-me `reject` and selection entries the user did not pick)
  leave no diff traces in the deliverable files.
- **SREV-05**: When `args.e2e` is true, `docs/features/<topic>/scenarios.yml` is also
  in scope for spec-review (scenarios are referenced in the findings).
- **SREV-06**: Whenever `test-strategy.md` or `scenarios.yml` was modified,
  a corresponding commit exists (no unstaged changes remain).
