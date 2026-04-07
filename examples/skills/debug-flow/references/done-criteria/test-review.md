---
name: test-review
max_retries: 3
audit: required
---

## Criteria

### TEST-REVIEW-01: Review executed across all 3 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/test-review-review.json` or review log) and confirm execution records exist for all 3 perspectives (coverage, quality, design-alignment).
- **pass_condition**: Execution records exist for all 3 perspectives. Recorded perspective count is 3
- **fail_diagnosis_hint**: Identify the missing perspective and check the /test-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### TEST-REVIEW-02: All user-approved findings have been fixed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all user-approved findings from the review results
  2. Identify the target file and line for each finding
  3. Read the target file location using `Read` and verify the fix corresponding to each finding has been applied
  4. List findings where the fix has not been applied
- **pass_condition**: Step 4 list is empty (all user-approved findings have fixes applied)
- **fail_diagnosis_hint**: Identify unapplied findings and check the target file at the relevant line. Determine whether the fix was omitted or applied in a different form. Use `git log --oneline` to check for the existence of fix commits
- **depends_on_artifacts**: [artifacts/reviews/, tests/]

### TEST-REVIEW-03: All RCA Report test perspectives are covered by test code
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all test perspectives from the RCA Report's Reproduction Test section and the fix plan's test case section, organized by the 4 categories (happy path, error handling, edge cases, non-functional)
  2. Extract all test function/method names from the test code directories (tests/, __tests__/, spec/, etc.) using `Grep`
  3. For each test perspective, verify at least one corresponding test function exists
  4. Check whether any test functions exist that are not covered by any test perspective (reverse coverage check)
  5. List uncovered test perspectives
- **pass_condition**: Step 5 list is empty (all test perspectives have corresponding test code)
- **fail_diagnosis_hint**: Identify uncovered test perspectives and check for imbalances across the 4 categories (happy path/error handling/edge cases/non-functional). If test perspective titles and test function names follow different conventions, verify correspondence by content inspection
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md, tests/]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
