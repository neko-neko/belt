---
name: dogfood
max_retries: 3
audit: lite
---

## Criteria

### DOGFOOD-01: Dogfood report exists (directory form) and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/dogfood-report/report.md")`
  2. Run `git status --porcelain -- docs/features/*/dogfood-report/` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes under the report directory
- **fail_diagnosis_hint**: `/dogfood` default output is `./dogfood-output/`; the dogfood supplement must override it to `docs/features/<topic>/dogfood-report/`. Confirm the supplement was loaded
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-02: Must-Verify Checklist is verified (feature runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `docs/features/<topic>/design.md` does not exist (bug runs), PASS (vacuously satisfied)
  2. Otherwise verify every item in design.md's `Must-Verify Checklist` has a status (`PASS`, `FAIL`, `N/A`) in the report's `Must-Verify Checklist Verification` section
- **pass_condition**: No design.md, OR zero checklist items without a recorded status
- **fail_diagnosis_hint**: List unverified items and explore each; N/A requires a one-line justification
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/dogfood-report/report.md]

### DOGFOOD-03: Root Cause mechanism does not re-emerge (bug runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `docs/features/<topic>/rca-report.md` does not exist (feature runs), PASS (vacuously satisfied)
  2. Otherwise read the RCA `## Root Cause` mechanism and confirm the report contains an explicit re-verification statement (e.g. "after the fix, the <mechanism> condition no longer triggers"), and that exploration found no re-manifestation in adjacent code paths
- **pass_condition**: Non-bug run, OR (re-verification statement present AND zero re-emergence findings)
- **fail_diagnosis_hint**: Fix is incomplete or asymmetric. Re-examine the RCA Symmetry Check output and Fix Strategy
- **depends_on_artifacts**: [docs/features/*/rca-report.md, docs/features/*/dogfood-report/report.md]

### DOGFOOD-04: Every monkey-test FAIL is addressed in the report
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read all FAIL entries from `monkey-test-results.json`
  2. Verify each is addressed in the report's `Known Issues Re-encountered` section (still broken / now passing)
- **pass_condition**: Zero FAIL entries missing from the section
- **fail_diagnosis_hint**: Retry each missing FAIL by hand and record the observation
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json, docs/features/*/dogfood-report/report.md]

### DOGFOOD-05: Evidence exists (screenshots/videos OR CLI-only rationale)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Check `docs/features/<topic>/dogfood-report/screenshots/` or `.../videos/` for at least one evidence file
  2. If the change scope contains zero UI files (CLI / API / backend-only), accept a rationale paragraph in report.md explaining the CLI-only exploration scope instead
- **pass_condition**: >= 1 evidence file, OR a CLI-only rationale paragraph is present
- **fail_diagnosis_hint**: For UI-touching changes `/dogfood` emits screenshots by default; for CLI-only changes ensure the rationale paragraph was added per the supplement
- **depends_on_artifacts**: [docs/features/*/dogfood-report/]

### DOGFOOD-06: New issues documented, or an explicit all-clear
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Verify the report either documents newly found issues (severity, reproduction steps, evidence), OR explicitly states "No critical or high issues found" with a rationale paragraph
- **pass_condition**: One of the two forms is present
- **fail_diagnosis_hint**: An empty findings section without an explicit all-clear reads as unfinished exploration — add one or the other
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-07: Summary counts are consistent with detail sections
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Compare the report's `Summary` counts (new issues by severity, known issues re-encountered, checklist coverage) against the corresponding detail sections
- **pass_condition**: Zero count mismatches
- **fail_diagnosis_hint**: Recount from the detail sections and correct the Summary
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-08: Narrative note captures exploratory results
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `dogfood_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: dogfood` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Observations records exploration coverage (feature runs: beyond-script findings; bug runs: Symmetry-Pair probe results)
  6. Verify Concerns flags unresolved risks and Directives carries forward regression guards for integrate
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: If Observations is thin, re-derive from the report's detail sections. If Concerns is empty, explicitly affirm that no regression signals surfaced. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [dogfood_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
