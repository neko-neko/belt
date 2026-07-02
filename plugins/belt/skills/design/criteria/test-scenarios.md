---
name: test-scenarios
max_retries: 3
audit: lite
---

## Criteria

### TEST-SCENARIOS-01: Test strategy document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/test-strategy.md")`
  2. Run `git status --porcelain -- docs/features/*/test-strategy.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes
- **fail_diagnosis_hint**: Confirm `/belt:test-scenarios` wrote and committed the strategy under the topic directory
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### TEST-SCENARIOS-02: Required strategy sections are present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the strategy document
  2. Verify sections exist: `Test Design Techniques` (ISTQB-based: equivalence partitioning, boundary-value analysis, decision tables, state transitions), `Quality Characteristics` (ISO 25010-based), `Priority Matrix` mapping characteristics to criticality
- **pass_condition**: All three sections are present
- **fail_diagnosis_hint**: Re-invoke `/belt:test-scenarios` for the missing sections; the section names are its output contract
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### TEST-SCENARIOS-03: Must-Verify Checklist items are covered
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate every item ID in the design document's `Must-Verify Checklist`
  2. For each ID, verify at least one corresponding entry exists in the strategy (ID cross-reference)
  3. List IDs with no corresponding entry
- **pass_condition**: Step 3 list is empty
- **fail_diagnosis_hint**: Add strategy entries for the uncovered checklist IDs
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/test-strategy.md]

### TEST-SCENARIOS-04: Scenarios file exists when --e2e
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Read `args.e2e` from `belt-agent status` JSON output
  2. If `args.e2e=false`, PASS (vacuously satisfied)
  3. If `args.e2e=true`:
     a. Search with `Glob("docs/features/*/scenarios.yml")` and confirm the file is committed
     b. Verify it contains at least 3 scenarios
     c. Verify every scenario has `id` (kebab-case), `category`, `severity` (`critical|high|medium|low`), `given`, `when`, `then`
     d. Verify `preconditions` / `postconditions` are present when applicable
- **pass_condition**: `args.e2e=false`, OR (file exists, committed, >= 3 scenarios, zero scenarios missing required keys)
- **fail_diagnosis_hint**: If missing, the e2e flag did not reach `/belt:test-scenarios` — check the invoke args passthrough
- **depends_on_artifacts**: [docs/features/*/scenarios.yml]

### TEST-SCENARIOS-05: At least one non-functional requirement with acceptance criterion
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the strategy document
  2. Verify at least one non-functional requirement (performance, security, or accessibility) is listed with a concrete acceptance criterion (numeric threshold or pattern-matchable assertion)
- **pass_condition**: At least one such requirement with a concrete criterion exists
- **fail_diagnosis_hint**: Derive one from the design's Quality Characteristics discussion; vague adjectives do not count as criteria
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
