---
name: dogfood-done-criteria
audit: lite
phase: dogfood
---

# Phase 7 (dogfood) Done Criteria

(Only evaluated when `args.e2e` is true; Phase is skipped otherwise.)

- **DOGFOOD-01**: `docs/features/<topic>/dogfood-report/report.md` exists
  and is committed.
- **DOGFOOD-02**: Every item in `design.md`'s `Must-Verify Checklist` has
  a verification status (`PASS`, `FAIL`, `N/A`) in
  `dogfood-report/report.md` under
  `Must-Verify Checklist Verification`.
- **DOGFOOD-03**: Every `FAIL` scenario in `monkey-test-results.json` is
  addressed in the `Known Issues Re-encountered` section of the report.
- **DOGFOOD-04**: Either:
  - ≥ 5 new issues are documented with severity, reproduction steps, and
    evidence (screenshot/video path under `screenshots/` or `videos/`), OR
  - The report explicitly states "No critical or high issues found" with
    a rationale paragraph.
- **DOGFOOD-05**: The report's `Summary` section counts (new issues by
  severity, known issues re-encountered, must-verify coverage) are
  consistent with the detail sections.

- **DOGFOOD-06**: Narrative note for the `dogfood_notes` artifact exists
  (locate its resolved_path via `belt-agent status`) with required frontmatter
  and 4 sections. Observations records exploratory findings beyond scripted
  scenarios. Concerns flags unresolved risks for integrate phase. Directives
  carries forward any must-verify items discovered during exploration. See
  `plugins/belt-agent/references/narrative-convention.md`.
