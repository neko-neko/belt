---
name: smoke-test
max_retries: 3
audit: required
---

## Criteria

### SMOKE-TEST-01: All smoke test steps pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Read the smoke test result files (logs/reports under `artifacts/smoke-test/`) and check the PASS/FAIL status of each step. If result files do not exist, re-execute the smoke tests.
- **pass_condition**: All steps have PASS status. Zero FAIL steps
- **fail_diagnosis_hint**: Check the name and error message of each FAIL step. Distinguish between browser operation failures (selector mismatch, timeout) and application errors (HTTP 5xx, exceptions). If screenshots are available in `artifacts/smoke-test/screenshots/`, review the screen state
- **depends_on_artifacts**: [artifacts/smoke-test/]

### SMOKE-TEST-02: Flaky tests are undetected or reported
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the smoke test result logs and detect cases where the same step produced different results on re-execution (FAIL on first run then PASS on second, or vice versa). If detected, verify they are recorded in the flaky test report list.
- **pass_condition**: Zero flaky detections, or all detected flaky cases are recorded in the report list
- **fail_diagnosis_hint**: Identify the flaky steps and investigate whether the cause is timing-dependent (setTimeout, animation waits), external service-dependent (API response delays), or test data-dependent (random data)
- **depends_on_artifacts**: [artifacts/smoke-test/]

### SMOKE-TEST-03: Test scenarios reflect project characteristics
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the project characteristics from the Evidence Plan (project_type, UI presence, API presence, etc.)
  2. Read the list of smoke test scenarios
  3. Determine whether required scenario categories exist for each project characteristic:
     - web-frontend: At least one scenario each for navigation, form interactions, and responsive display
     - API: At least one scenario each for successful responses (2xx) and error responses (4xx/5xx)
     - DB: At least one scenario each for data write and data read operations
  4. Enumerate the main user flows from the RCA Report's Impact Scope and verify at least one scenario exists for each flow
- **pass_condition**: Step 3: all required categories have at least one scenario. Step 4: all user flows have a corresponding scenario. Zero missing categories or unmatched flows
- **fail_diagnosis_hint**: Identify missing categories and add smoke test scenarios for them. For unmatched user flows, reference the RCA Report to create scenarios. If the Evidence Plan's project characteristics do not match reality, consider updating the Evidence Plan
- **depends_on_artifacts**: [artifacts/smoke-test/, docs/plans/*-rca-report.md]

### SMOKE-TEST-04: Smoke test execution evidence is valid
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Mechanically verify the following 3 points:
  1. `smoke-test-report.md` exists in the working directory and conforms to the required format (Step 2 table contains "Scenario", "Perspective", "Result", and "Screenshot" columns)
  2. At least one `smoke-*.png` file exists
  3. The "Screenshot" column in the Step 2 table of the report references existing `smoke-*.png` files
- **pass_condition**: All 3 points are satisfied
- **fail_diagnosis_hint**:
  - Report missing: The smoke-test skill was not properly executed. Check whether existing test suites (rspec, jest, etc.) were run as a substitute. If so, that is an invalid execution and the smoke-test skill must be re-run correctly
  - Format non-compliant: Regenerate the report
  - Screenshots missing: The browser-use CLI may not have been executed. If it is an environment issue, report to the user with a PAUSE status
- **depends_on_artifacts**: [smoke-test-report.md, smoke-*.png]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
