---
name: monkey-test
max_retries: 3
audit: lite
---

## Criteria

### MONKEY-TEST-01: Monkey test report file exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/monkey-test-report.md")`
  2. Run `git status --porcelain -- docs/features/*/monkey-test-*` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted monkey-test artifacts
- **fail_diagnosis_hint**: `/belt:monkey-test` did not produce or commit the report. Confirm the monkey-test supplement was loaded and the scenarios source was resolvable
- **depends_on_artifacts**: [docs/features/*/monkey-test-report.md]

### MONKEY-TEST-02: Results JSON exists and validates against the schema
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/monkey-test-results.json")` and parse as JSON
  2. Verify the top-level shape matches the schema in `references/monkey-test-supplement.md` (`scenarios` array + `summary` object with total/passed/failed/skipped)
- **pass_condition**: File exists AND JSON parses AND both `scenarios` and `summary` fields are present
- **fail_diagnosis_hint**: Re-run the report-writing step of `/belt:monkey-test`; compare the emitted JSON against the supplement schema
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-03: Every scenario in the source file has a result entry
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Resolve the scenarios source: `docs/features/*/scenarios.yml` (feature runs) or `docs/features/*/rca-scenarios.yml` (bug runs) — whichever exists; on both, prefer the one referenced in the run's notes
  2. Enumerate scenario ids from the source file
  3. Verify each id has a matching entry in `results.json.scenarios` with status `PASS`, `FAIL`, or `SKIP`
- **pass_condition**: Zero source scenarios without a result entry
- **fail_diagnosis_hint**: List missing ids; check whether the replay loop aborted mid-run and resume from the first missing scenario
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-04: Reproduction scenario transitions to PASS (bug runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If the scenarios source is NOT `rca-scenarios.yml`, PASS (vacuously satisfied — feature runs have no reproduction scenario)
  2. Otherwise, the first scenario corresponds to the RCA Reproduction Test; confirm its result in `results.json` is PASS (it FAILed pre-fix per the rca criteria)
- **pass_condition**: Non-bug run, OR first scenario status is PASS
- **fail_diagnosis_hint**: The fix did not resolve the root cause — re-examine the Fix Strategy and execute-phase output. If the scenario itself is malformed, correct its Given/When/Then and re-run monkey-test
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json, docs/features/*/rca-scenarios.yml]

### MONKEY-TEST-05: Critical/high failures are detailed in the report
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Filter `results.json.scenarios` for status FAIL with severity `critical` or `high`
  2. Verify each such failure appears in the report's primary section with expected-vs-actual and at least one screenshot reference
- **pass_condition**: Zero critical/high FAILs missing from the report's primary section
- **fail_diagnosis_hint**: Cross-reference the missing ids against the report; re-emit the report from results.json
- **depends_on_artifacts**: [docs/features/*/monkey-test-report.md, docs/features/*/monkey-test-results.json]

### MONKEY-TEST-06: SKIP entries carry a non-empty skip_reason
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Filter `results.json.scenarios` for status SKIP
  2. Verify each has a non-empty `skip_reason` (feature runs: referencing the incomplete plan task; bug runs: a documented rationale)
- **pass_condition**: Zero SKIP entries with an empty or missing skip_reason
- **fail_diagnosis_hint**: Identify undocumented SKIPs and either replay them or record why they cannot run
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-07: Narrative note captures replay outcomes
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `monkey_test_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: monkey-test` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Observations records per-scenario results (bug runs: whether the reproduction scenario now PASSes)
  6. Verify Directives carries forward dogfood exploration targets surfaced by replay
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: If Observations lacks outcomes, re-derive from `monkey-test-results.json`. If Directives empty, identify regression hotspots dogfood should explore. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [monkey_test_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
