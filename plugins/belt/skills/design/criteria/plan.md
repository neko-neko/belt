---
name: plan
max_retries: 3
audit: lite
---

## Criteria

### PLAN-01: Plan document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/plan.md")`
  2. Run `git status --porcelain -- docs/features/*/plan.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes
- **fail_diagnosis_hint**: Confirm `/writing-plans` honored the writing-plans supplement's path override
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-02: Required plan header is present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the plan document
  2. Verify it contains `Goal`, `Architecture`, `Tech Stack`, and at least one `Task N` section
- **pass_condition**: All four elements present
- **fail_diagnosis_hint**: The writing-plans header template was skipped — regenerate the header
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-03: Every task follows the TDD shape
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. For each `Task N`, verify the step sequence covers: failing test -> minimal implementation -> passing test -> commit
  2. Verify code steps contain explicit code blocks and run steps contain explicit commands
- **pass_condition**: Zero tasks missing the TDD sequence or explicit code/commands
- **fail_diagnosis_hint**: Identify non-conforming tasks and expand them per the writing-plans skill's task structure
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-04: Must-Verify Checklist items are cited by tasks
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate item IDs from the design document's `Must-Verify Checklist` (e.g. `MV-01`)
  2. Verify each ID is cited by at least one task
  3. List uncited IDs
- **pass_condition**: Step 3 list is empty
- **fail_diagnosis_hint**: Map each uncited item to an existing task or add a covering task
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/plan.md]

### PLAN-05: No placeholder language remains
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. `Grep` the plan for placeholder patterns: `TBD`, `TODO`, `add appropriate error handling`, `similar to Task`
  2. Verify no referenced type/function is undefined across all tasks
- **pass_condition**: Zero placeholder matches AND zero unresolved references
- **fail_diagnosis_hint**: Expand each placeholder into concrete content; the executor cannot fill gaps
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-06: Scenario IDs are cited when --e2e
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `args.e2e=false`, PASS (vacuously satisfied)
  2. Enumerate every `id` in `scenarios.yml` and verify each is cited by at least one task
- **pass_condition**: Non-e2e run, OR zero uncited scenario ids
- **fail_diagnosis_hint**: Add citations mapping scenarios to the tasks that implement their behavior
- **depends_on_artifacts**: [docs/features/*/scenarios.yml, docs/features/*/plan.md]

### PLAN-07: Input parameters have four-class Given/When/Then coverage
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate input parameters surfaced in `test-strategy.md`
  2. For each, verify Given/When/Then coverage exists for: normal, boundary, abnormal, state-transition
- **pass_condition**: Zero parameters missing any of the four classes
- **fail_diagnosis_hint**: Derive the missing cases from the strategy's test design techniques section
- **depends_on_artifacts**: [docs/features/*/test-strategy.md, docs/features/*/plan.md]

### PLAN-08: Narrative note captures plan decomposition
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `plan_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: plan` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Decisions records task decomposition rationale and granularity choices
  6. Verify Directives records constraints for the execute phase (e.g. commit granularity rules, test-first enforcement)
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [plan_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
