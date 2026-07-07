---
name: rca
max_retries: 3
audit: lite
---

## Criteria

### RCA-01: RCA Report file exists with required sections
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Search for an RCA Report file using `Glob("docs/features/*/rca-report.md")`
  2. Verify the file contains 5 section headings: `## Symptom`, `## Investigation Record`, `## Root Cause`, `## Reproduction Test`, `## Fix Strategy`
- **pass_condition**: At least one Glob result found, and all 5 section headings are present
- **fail_diagnosis_hint**: Verify that the RCA phase executor output the report under `docs/features/`. Confirm the file is named `rca-report.md` inside the `docs/features/<YYYY-MM-DD-topic>/` directory and section headings use `##` level with exact titles
- **depends_on_artifacts**: [docs/features/]
- **forward_check**: Fix Plan phase uses the RCA Report's Fix Strategy as input

### RCA-02: Investigation Record has substantive content in 4 subsections
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  Read each Investigation Record subsection and verify:
  1. Code Flow Trace: at least one call chain documented (file path + function name pair)
  2. Architecture Context: at least one relevant pattern, convention, or implicit rule described
  3. Impact Scope: at least one affected file or module listed
  4. Symmetry Check: determination of whether the change target has a "pair" (if none, rationale required)
- **pass_condition**: All 4 subsections meet their criteria. Zero heading-only or generic-content-only subsections
- **fail_diagnosis_hint**: Identify which subsection is deficient. Cross-reference with belt-agent:explorer (focus: flow / patterns / impact) output to fill gaps
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### RCA-03: Impact Scope file paths exist in the codebase
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Extract file paths from the Impact Scope subsection using regex
  2. Verify each path exists using `Glob`
- **pass_condition**: All extracted paths exist. Zero non-existent paths
- **fail_diagnosis_hint**: List non-existent paths and determine if they are typos or deleted files. Verify the codebase state matches the RCA Report's analysis point
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### RCA-04: At least 1 excluded hypothesis recorded
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the Excluded Hypotheses section (within Investigation Record or as a standalone section)
  2. Verify each hypothesis entry contains: hypothesis statement, verification method, rejection reason
  3. If the first hypothesis was correct, verify documentation of why alternatives were excluded
- **pass_condition**: At least 1 excluded hypothesis with all 3 elements present
- **fail_diagnosis_hint**: If zero hypotheses are recorded, the investigation may not have considered alternative causes. List potential alternative root causes and document why each was excluded
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### RCA-05: Reproduction test exists and its result is FAIL
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Extract the test file path from the Reproduction Test section
  2. Verify the test file exists using `Glob`
  3. Execute the test command and confirm the specific reproduction test FAILs
- **pass_condition**: Test file exists and test execution shows the reproduction test as FAIL
- **fail_diagnosis_hint**: If the test file does not exist, the RCA executor did not create it. If the test PASSes, the test does not correctly capture the bug — review the Root Cause mechanism and fix the assertion
- **depends_on_artifacts**: [docs/features/*/rca-report.md, tests/]

### RCA-06: Root Cause contains specific file path, line number, and mechanism
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the Root Cause section
  2. Check for at least one file path (string containing `/` or `.`)
  3. Check for at least one line number (`:` + digits or "line" + digits)
  4. Check for at least one mechanism explanation (why the code at that location causes the problem)
- **pass_condition**: File path, line number, and mechanism explanation all present
- **fail_diagnosis_hint**: If missing, use belt-agent:explorer (focus: flow) output to identify the exact fault location. Write the mechanism as "input X passes through Y and produces state Z because of [specific code behavior]"
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### RCA-07: RCA Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/features/*/rca-report.md` and confirm the file is not in the uncommitted changes list.
- **pass_condition**: `git status --porcelain` output does not contain the RCA Report path (zero output lines)
- **fail_diagnosis_hint**: If uncommitted, `git add` + `git commit` was not executed. Check the final step of the RCA phase executor
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### RCA-08: Symmetry Check evaluates asymmetry risk
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  Read the Symmetry Check subsection and verify documentation of:
  1. Whether the change target has a "pair" (determination result; if none, rationale required)
  2. If pairs exist: file paths, function names, and pair type for each
  3. Symmetry comparison of filter/scope conditions
  4. Asymmetry risk assessment and impact scope
- **pass_condition**: All 4 dimensions documented. If "no pair" is determined, the rationale is specific (not generic)
- **fail_diagnosis_hint**: If Symmetry Check is empty or incomplete, reference belt-agent:explorer (focus: impact) output to identify paired paths. For "no pair" determinations, verify against Reverse Dependencies and Shared State analysis
- **depends_on_artifacts**: [docs/features/*/rca-report.md]
- **forward_check**: Fix Plan must include tasks for paired paths if asymmetry risk is identified

### RCA-09: Reproduction scenarios file exists
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Search for the scenarios file using `Glob("docs/features/*/rca-scenarios.yml")`
  2. Verify it contains at least one Given/When/Then scenario and every scenario has `kind: browser` or `kind: cli`
- **pass_condition**: file exists with ≥1 scenario, each carrying a kind
- **fail_diagnosis_hint**: If the file is missing, the RCA executor did not load `rca-supplement.md`. Confirm supplement injection in the diagnose SKILL.md rca invocation
- **depends_on_artifacts**: [docs/features/*/rca-scenarios.yml]
- **forward_check**: the qa phase replays `rca_scenarios` via belt:qa-verifier

### RCA-10: Narrative note captures root-cause investigation
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `rca_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: rca` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Decisions records the chosen root cause hypothesis and which alternative candidates were rejected, with rationale
  6. Verify Directives records Fix Strategy constraints (test-first requirements, regression scope) for the fix-plan phase
- **pass_condition**: Steps 1-6 all pass; Concerns section flags any ambiguity in the reproduction window or environment coupling that downstream phases must verify
- **fail_diagnosis_hint**: If Decisions lacks rejected-candidate rationale, re-read the RCA report's Investigation Record. If Directives empty, derive Fix Strategy constraints from the RCA report's Fix Strategy section. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [rca_notes]

## Observation Collection

The orchestrator MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
