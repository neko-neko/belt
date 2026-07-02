---
name: evidence-catalog
description: >-
  Concrete evidence catalog for feature-dev pipeline. Conforms to
  plugins/belt-agent/references/evidence-schema.md. Evidence is picked
  by each phase's criteria/<phase>.md via uses_evidence (IoC).
---

# Evidence Catalog (feature-dev)

Concrete evidence items available to feature-dev pipeline phases. Each
phase's `criteria/<phase>.md` declares which evidence it `uses_evidence`.

Schema: [`plugins/belt-agent/references/evidence-schema.md`](../../../../belt-agent/references/evidence-schema.md)

## Evidence Layers

- **Claimed (Layer 1)**: Files the Executor collects and stores.
- **Verified (Layer 2)**: Independent checks performed by the Audit Agent.

## Universal Evidence

Universal evidence is `condition: always` and assumed collectable. When
collection is impossible (no test framework for E-TEST, no build system for
E-BUILD), the Audit Agent treats it as a blocker FAIL.

### E-TEST: Test execution log
- **condition**: always
- **claimed**: `artifacts/test-results/phase-{N}-test.log`
- **verified**: Re-run the test command independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the test command's stdout/stderr to the file.

### E-BUILD: Build log
- **condition**: always
- **claimed**: `artifacts/build/phase-{N}-build.log`
- **verified**: Re-run the build command independently and confirm exit code 0.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the build command's stdout/stderr to the file.

### E-LINT: Lint / type-check log
- **condition**: always
- **claimed**: `artifacts/lint/phase-{N}-lint.log`
- **verified**: Re-run the linter independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the linter's stdout/stderr to the file.

### E-REVIEW: Review results
- **condition**: always
- **claimed**: `belt://current/review/findings.json` (resolve via `belt-agent locate`)
- **verified**: N/A (re-running a review is impractical).
- **required_capabilities**: []
- **if_unavailable**: block
- **collection**: The code-review skill merges the per-observation finding files (findings-security / findings-test / findings-ai-antipattern / findings-cross-cutting, plus findings-codex when enabled) into the merged `findings` artifact.

### E-DIFF: git diff snapshot
- **condition**: always
- **claimed**: `artifacts/diff/phase-{N}.diff`
- **verified**: Re-obtain `git diff` independently and confirm it matches.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: `git diff > artifacts/diff/phase-{N}.diff`

### E-TRACE: Traceability matrix
- **condition**: always
- **claimed**: `artifacts/traceability/phase-{N}-trace.md`
- **verified**: Independently verify that spec requirements map to implementation files.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Generate a mapping table between the spec's requirement list and implementation files.

## Conditional Evidence

### E-SCREENSHOT: Screen screenshots
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
- **condition**:
  - `grep -r "router\|app\.\(get\|post\|put\|delete\)\|@app\.route\|@router" **/*.{ts,js,py,go,rb}` returns 1 or more matches
- **claimed**: `artifacts/api/phase-{N}-api.log`
- **verified**: Send an HTTP request to the endpoint and confirm the response.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Hit the endpoint with curl or httpie and save the result to the log.

### E-MIGRATION: DB migration log
- **condition**:
  - `glob("**/migrations/**/*")` or `glob("**/migrate/**/*")` returns 1 or more matches
- **claimed**: `artifacts/migration/phase-{N}-migration.log`
- **verified**: Connect to the DB and confirm that the migration's target tables/columns exist.
- **required_capabilities**: [database-access]
- **if_unavailable**: manual_fallback
- **collection**: Save the migration command's output to the log.

### E-PERF: Performance metrics
- **condition**:
  - The spec mentions any of "performance", "latency", or "throughput"
- **claimed**: `artifacts/perf/phase-{N}-perf.log`
- **verified**: Re-run the load test independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Save the load-testing tool's output to the log.

### E-CONSOLE: Browser console log
- **condition**:
  - E-SCREENSHOT is enabled
- **claimed**: `artifacts/smoke-test/console.log`
- **verified**: Capture console output during page access and confirm no error-level entries.
- **required_capabilities**: [browser-automation]
- **if_unavailable**: skip_with_warning
- **collection**: Capture console logs using a browser-automation tool.

### E-DEFERRED-IMPACT: Deferred impact findings — actual harm verification
- **condition**:
  - require_all:
    - The merged findings artifact (`belt://current/review/findings.json`) contains 1 or more findings with `observation: impact` whose triage disposition was deferral (recorded in the code-review narrative note)
- **claimed**: `belt://current/review/deferred-impact-verification.md`
- **verified**: Actually exercise the consumer named by each deferred finding and confirm no inconsistency.
- **required_capabilities**: [bash, browser-automation]
- **if_unavailable**: manual_fallback
- **collection**: For each deferred impact finding, record (1) a summary of the finding, (2) the result of obtaining the same metric from the paired consumer, and (3) the consistency verdict (match / mismatch).
