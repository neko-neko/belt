# Report Template

Write the smoke test report to `smoke-test-report.md` in the working directory.
This file is not committed to git.

## Template

```markdown
# Smoke Test Report

**Date:** YYYY-MM-DD HH:MM
**Diff Base:** <branch or HEAD~1>
**Server:** <command> (port: <N>)
**Status:** PASS / FAIL / PAUSE

## Ad-hoc Smoke Test

| Scenario | Perspective | Result | Screenshot |
|----------|------------|--------|------------|
| <name> | <perspective> | PASS/FAIL | smoke-<name>.png |

### Evidence Log

#### Check: <scenario name>
- **Action:** <what was done (browser navigation, click, input, etc.)>
- **Observed:** <what was seen (page state, console output, network response)>
- **Result:** PASS / FAIL

## VRT Diff Check

<PASS / SKIP / DIFF_DETECTED (with details)>

## E2E + Flaky Detection

### Test Results

| Test | Run 1 | Run 2 | Verdict |
|------|-------|-------|---------|
| <test name> | PASS/FAIL | PASS/FAIL | stable / implementation failure / flaky |

### Flaky Tests

| Test | Suspected Cause | Suggested Fix |
|------|----------------|---------------|
| <test> | timing / external dependency / nondeterministic data / DOM state | <suggestion> |

### Implementation Failures

| Test | Error | Suggested Fix |
|------|-------|---------------|
| <test> | <error message> | <suggestion> |
```

## Status Determination

| Condition | Status |
|-----------|--------|
| All steps PASS (flaky tolerated) | PASS |
| adhoc-test scenario fails after 2 retries | FAIL |
| Adversarial probe not executed | FAIL |
| E2E implementation failure (both runs FAIL) | FAIL |
| Server could not start | PAUSE |
| Only flaky tests detected (rest PASS) | PASS |
