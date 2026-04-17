---
name: evidence-catalog
description: Evidence catalog. Defines all evidence types and applicability conditions that the Audit Agent references when generating an Evidence Plan.
---

# Evidence Catalog

Catalog referenced by the Audit Agent during Evidence Plan generation. Each evidence item's applicability is based on observable facts (verifiable with glob/grep).

## Activity Types

| Activity | Description |
|----------|------------|
| implementation | A step that performs code implementation |
| investigation | A step that investigates root causes or tests hypotheses |
| smoke-test | A step that confirms runtime behavior |
| review-fix | A step that addresses review findings |
| test-fix | A step that adds or modifies tests |
| doc-maintenance | A step that audits or updates documentation |
| integration | A step that integrates code |

## Evidence Layers

- **Claimed (Layer 1)**: Files the Executor collects and stores — "this is what happened" records.
- **Verified (Layer 2)**: Independent checks performed by the Audit Agent — "does it really hold" verification.

## Universal (common to all projects)

Universal evidence is `condition: always` and is assumed collectable across all projects. When collection is impossible (for example, no test framework configured for E-TEST or no build system for E-BUILD), the Audit Agent treats that item as a blocker FAIL.

### E-TEST: Test execution log
- **applies_to**: [implementation, investigation, review-fix, test-fix]
- **condition**: always
- **claimed**: `artifacts/test-results/phase-{N}-test.log`
- **verified**: Re-run the test command independently and compare results.
- **required_capabilities**: [bash]
- **collection**: Redirect the test command's stdout/stderr to the file.

### E-BUILD: Build log
- **applies_to**: [implementation]
- **condition**: always
- **claimed**: `artifacts/build/phase-{N}-build.log`
- **verified**: Re-run the build command independently and confirm exit code 0.
- **required_capabilities**: [bash]
- **collection**: Redirect the build command's stdout/stderr to the file.

### E-LINT: Lint / type-check log
- **applies_to**: [implementation, review-fix]
- **condition**: always
- **claimed**: `artifacts/lint/phase-{N}-lint.log`
- **verified**: Re-run the linter independently and compare results.
- **required_capabilities**: [bash]
- **collection**: Redirect the linter's stdout/stderr to the file.

### E-REVIEW: Review results
- **applies_to**: [review-fix, test-fix]
- **condition**: always
- **claimed**: `artifacts/reviews/phase-{N}-review.json`
- **verified**: N/A (re-running a review is impractical).
- **required_capabilities**: []
- **collection**: Aggregate review agent output into JSON and save.

### E-DIFF: git diff snapshot
- **applies_to**: [implementation, investigation, review-fix, test-fix]
- **condition**: always
- **claimed**: `artifacts/diff/phase-{N}.diff`
- **verified**: Re-obtain `git diff` independently and confirm it matches.
- **required_capabilities**: [bash]
- **collection**: `git diff > artifacts/diff/phase-{N}.diff`

### E-TRACE: Traceability matrix
- **applies_to**: [implementation, investigation]
- **condition**: always
- **claimed**: `artifacts/traceability/phase-{N}-trace.md`
- **verified**: Independently verify that spec requirements map to implementation files.
- **required_capabilities**: [bash]
- **collection**: Generate a mapping table between the spec's requirement list and implementation files.

## Conditional (enabled based on project characteristics)

### E-SCREENSHOT: Screen screenshots
- **applies_to**: [smoke-test]
- **condition**:
  - require_all:
    - `glob("**/*.{html,jsx,tsx,vue,svelte}")` returns 1 or more matches
    - The spec mentions any of "screen", "page", "UI", or "component"
- **claimed**: `artifacts/smoke-test/screenshots/{screen}_{state}.png`
- **verified**: Access the same URL in a browser and confirm the page renders.
- **required_capabilities**: [browser-automation]
- **variants**: [desktop, mobile]
- **if_unavailable**: skip_with_warning
- **collection**: Capture screenshots using a browser-automation tool.

### E-SCREENSHOT-MOBILE: Mobile screenshots
- **applies_to**: [smoke-test]
- **condition**:
  - require_all:
    - E-SCREENSHOT is enabled
    - The spec mentions "responsive" or "mobile"
- **claimed**: `artifacts/smoke-test/screenshots/{screen}_{state}_mobile.png`
- **verified**: Access with a mobile viewport (≤428px wide) and confirm rendering.
- **required_capabilities**: [browser-automation]
- **if_unavailable**: skip_with_warning
- **collection**: Capture screenshots using a mobile viewport.

### E-API-LOG: API response log
- **applies_to**: [implementation, investigation, smoke-test]
- **condition**:
  - `grep -r "router\|app\.\(get\|post\|put\|delete\)\|@app\.route\|@router" **/*.{ts,js,py,go,rb}` returns 1 or more matches
- **claimed**: `artifacts/api/phase-{N}-api.log`
- **verified**: Send an HTTP request to the endpoint and confirm the response.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Hit the endpoint with curl or httpie and save the result to the log.

### E-MIGRATION: DB migration log
- **applies_to**: [implementation]
- **condition**:
  - `glob("**/migrations/**/*")` or `glob("**/migrate/**/*")` returns 1 or more matches
- **claimed**: `artifacts/migration/phase-{N}-migration.log`
- **verified**: Connect to the DB and confirm that the migration's target tables/columns exist.
- **required_capabilities**: [database-access]
- **if_unavailable**: manual_fallback
- **collection**: Save the migration command's output to the log.

### E-PERF: Performance metrics
- **applies_to**: [smoke-test]
- **condition**:
  - The spec mentions any of "performance", "latency", or "throughput"
- **claimed**: `artifacts/perf/phase-{N}-perf.log`
- **verified**: Re-run the load test independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Save the load-testing tool's output to the log.

### E-CONSOLE: Browser console log
- **applies_to**: [smoke-test]
- **condition**:
  - E-SCREENSHOT is enabled
- **claimed**: `artifacts/smoke-test/console.log`
- **verified**: Capture console output during page access and confirm no error-level entries.
- **required_capabilities**: [browser-automation]
- **if_unavailable**: skip_with_warning
- **collection**: Capture console logs using a browser-automation tool.

### E-DEFERRED-IMPACT: Deferred impact findings — actual harm verification
- **applies_to**: [review-fix]
- **condition**:
  - require_all:
    - The review result (`artifacts/reviews/phase-{N}-review.json`) contains 1 or more findings with category: code-impact and user_decision: deferred
- **claimed**: `artifacts/reviews/phase-{N}-deferred-impact-verification.md`
- **verified**: Actually exercise the consumer named by each deferred finding and confirm no inconsistency.
- **required_capabilities**: [bash, browser-automation]
- **if_unavailable**: manual_fallback
- **collection**: For each deferred impact finding, record (1) a summary of the finding, (2) the result of obtaining the same metric from the paired consumer, and (3) the consistency verdict (match / mismatch).

## Doc Maintenance (specific to doc-audit)

### E-DOC-REPORT: doc-audit integrated report
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-report.json`
- **verified**: Structural validation of the report JSON (required fields: categories, findings, summary).
- **required_capabilities**: [bash]
- **collection**: Save the doc-audit skill's integrated report output as JSON.

### E-DOC-DIFF: Documentation change diff
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/diff/phase-{N}-doc.diff`
- **verified**: Re-obtain `git diff` independently and confirm that .md file changes are included.
- **required_capabilities**: [bash]
- **collection**: `git diff -- '*.md' > artifacts/diff/phase-{N}-doc.diff`

### E-DOC-SCRIPT: doc-audit.sh execution log
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-script-output.json`
- **verified**: Re-run doc-audit.sh independently and compare results.
- **required_capabilities**: [bash]
- **collection**: `doc-audit.sh --full --json > artifacts/doc-audit/phase-{N}-script-output.json`

### E-DOC-CHECK: doc-check execution log
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-doc-check.log`
- **verified**: Confirm the doc-check exit code (0 = no impact, or user-approved).
- **required_capabilities**: [bash]
- **collection**: Redirect doc-check's stdout/stderr to the file.

### E-DOC-EXPLORATION: Exploration agent results
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-exploration.json`
- **verified**: N/A (re-running agent output is impractical).
- **required_capabilities**: []
- **collection**: Aggregate the integrated output of exploration agents (code-explorer, code-architect, impact-analyzer) as JSON.

### E-DOC-FINDINGS: Finding handling records
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-findings.json`
- **verified**: Confirm each finding's status is a valid value (fixed, skipped, deleted, linked, updated) and that skipped entries carry a user-approval record.
- **required_capabilities**: [bash]
- **collection**: Save each finding's processing result (status, user_decision) as a JSON array.

### E-DOC-VERIFY: Documentation consistency verification result
- **applies_to**: [doc-maintenance]
- **condition**: always
- **claimed**: `artifacts/doc-audit/phase-{N}-verify.json`
- **verified**: Re-run doc-audit.sh and confirm broken_deps=0 and dead_links=0.
- **required_capabilities**: [bash]
- **collection**: Save the post-fix doc-audit.sh re-run result as JSON.

## if_unavailable Policies

- **skip_with_warning**: Exclude the evidence and warn the user. Does not affect the verdict.
- **manual_fallback**: Ask the user to collect it manually. PAUSE and wait for the user to provide the evidence.
