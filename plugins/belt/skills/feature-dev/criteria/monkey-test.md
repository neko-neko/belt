---
name: monkey-test-done-criteria
audit: lite
phase: monkey-test
---

# Phase 6 (monkey-test) Done Criteria

(Only evaluated when `args.e2e` is true; Phase is skipped otherwise.)

- **MONKEY-01**: `docs/features/<topic>/monkey-test-report.md` exists and is
  committed.
- **MONKEY-02**: `docs/features/<topic>/monkey-test-results.json` exists and
  validates against the schema in
  `references/monkey-test-supplement.md`.
- **MONKEY-03**: Every scenario `id` from `scenarios.yml` has a matching
  entry in `results.json.scenarios` with status `PASS`, `FAIL`, or `SKIP`.
- **MONKEY-04**: Every FAIL whose severity is `critical` or `high` is
  described in detail in `monkey-test-report.md`'s primary section with
  expected-vs-actual and at least one screenshot.
- **MONKEY-05**: `SKIP` entries include a non-empty `skip_reason` referencing
  the `plan.md` task that is incomplete.

- **MONKEY-06**: Narrative note for the `monkey_test_notes` artifact exists
  (locate its resolved_path via `belt-agent status`) with required frontmatter
  and 4 sections. Observations records scenario replay results (pass/fail per
  scenario). Concerns flags scenarios that revealed unexpected behavior worth
  dogfood follow-up. Directives carries forward regression hotspots. See
  `plugins/belt-agent/references/narrative-convention.md`.
