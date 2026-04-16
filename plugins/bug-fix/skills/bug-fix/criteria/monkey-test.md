---
name: monkey-test
max_retries: 3
audit: required
---

## Criteria

### MONKEY-TEST-01: Monkey test report file exists
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-monkey-test-report.md")`
- **pass_condition**: At least one match
- **fail_diagnosis_hint**: `/monkey-test:monkey-test` invocation did not produce the report. Confirm Phase 6 supplement was loaded and scenarios source `docs/plans/*-rca-scenarios.yml` was resolvable
- **depends_on_artifacts**: [docs/plans/*-monkey-test-report.md]

### MONKEY-TEST-02: Monkey test results JSON exists
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-monkey-test-results.json")` and parse as valid JSON
- **pass_condition**: File exists AND JSON parses successfully
- **depends_on_artifacts**: [docs/plans/*-monkey-test-results.json]

### MONKEY-TEST-03: Reproduction scenario replay is PASS
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The first scenario in `rca-scenarios.yml` corresponds to the RCA Reproduction Test
  2. In `monkey-test-results.json`, confirm this scenario's result is PASS (previously FAIL per RCA-05)
- **pass_condition**: First scenario PASSes post-fix
- **fail_diagnosis_hint**: If PASS not achieved, the fix did not resolve the root cause. Re-examine Fix Strategy and `execute` phase output. If the scenario itself is malformed, correct the Given/When/Then in `rca-scenarios.yml` and re-run monkey-test
- **depends_on_artifacts**: [docs/plans/*-monkey-test-results.json, docs/plans/*-rca-scenarios.yml]

### MONKEY-TEST-04: All scenarios executed (no skip without rationale)
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Count scenarios in `rca-scenarios.yml`
  2. Count results in `monkey-test-results.json`
  3. Skipped scenarios must each have a rationale line in the report
- **pass_condition**: Scenario count matches results count, OR every skipped scenario has a documented rationale
- **depends_on_artifacts**: [docs/plans/*-rca-scenarios.yml, docs/plans/*-monkey-test-results.json, docs/plans/*-monkey-test-report.md]

### MONKEY-TEST-05: Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**: Run `git status --porcelain -- docs/plans/*-monkey-test-*.{md,json}`; confirm zero output lines
- **pass_condition**: `git status --porcelain` returns empty for monkey-test artifacts

### MONKEY-TEST-06: Narrative note captures replay outcomes
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-monkey-test.md`
  2. Verify frontmatter contains `phase: monkey-test` and `run_id: <run_id>`
  3. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify Observations records reproduction scenario results (was the RCA reproduction test now PASS?)
  5. Verify Directives carries forward dogfood exploration targets surfaced by replay
- **pass_condition**: Steps 1-5 all pass
- **fail_diagnosis_hint**: If Observations missing reproduction PASS/FAIL outcome, cross-reference `monkey-test-results.json` and re-derive. If Directives empty, identify which regression hotspots dogfood should explore. See `plugins/belt-agents/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-monkey-test.md]

## Observation Collection

The belt-agents:phase-auditor MUST include `observations[]` in its verdict output. Record
quality/warning-level findings even for criteria that PASS.
