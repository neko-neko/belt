---
name: fix-plan-review
max_retries: 3
audit: required
---

## Criteria

### FIX-PLAN-REVIEW-01: Review artifact (findings.json) exists
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Locate the review artifact file at `.belt/runs/*/review/findings.json`
  2. Verify the file exists
  3. Parse as JSON and confirm a `findings` array field is present
- **pass_condition**: File exists AND parses as valid JSON AND contains a `findings` array
- **fail_diagnosis_hint**: `/implementation-review` invocation interrupted or artifact path drift. Re-invoke the skill from the fix-plan-review phase
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json]

### FIX-PLAN-REVIEW-02: Fix plan and RCA Report are consistent
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Cross-reference the RCA Report's Fix Strategy list with the fix plan document's task list
  2. Verify component names, file paths, and data types used in the fix plan document match the definitions in the RCA Report
  3. Verify that task completion conditions in the fix plan document do not deviate from Fix Strategy items
  4. Verify interfaces defined in the RCA Report (function signatures, API endpoints, etc.) are correctly referenced in the fix plan document
- **pass_condition**: Zero mismatches in component names / paths / types, zero deviations, zero reference inconsistencies
- **fail_diagnosis_hint**: Compare inconsistent entries side-by-side. If a review fix updated only one document, trace cause via `git log --oneline -- docs/plans/`
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md]

### FIX-PLAN-REVIEW-03: No unresolved blocker findings in review artifact
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Parse `.belt/runs/*/review/findings.json`
  2. Filter findings where `severity == "blocker"`
  3. For each blocker finding, verify either (a) a resolution comment / fix commit is referenced in the fix plan, OR (b) the finding has been explicitly rejected by user triage
- **pass_condition**: Zero unresolved blocker findings
- **fail_diagnosis_hint**: User triage (accept/reject for each finding) is incomplete, or fix commits have not landed. Re-run the `/implementation-review` fix phase with accepted blocker findings
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json, docs/plans/*-fix-plan.md]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output. Record
quality/warning-level findings even for criteria that PASS. Observations
accumulate in the pipeline's audit trail.
