---
name: fix-plan
max_retries: 3
audit: required
---

## Criteria

### FIX-PLAN-01: Fix plan document file exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Search for a fix plan document file using `Glob("docs/plans/YYYY-MM-DD-*-fix-plan.md")`.
- **pass_condition**: At least one Glob result found
- **fail_diagnosis_hint**: Verify that the Phase executor output the fix plan document under `docs/plans/`. Confirm the filename matches the `YYYY-MM-DD-*-fix-plan.md` pattern
- **depends_on_artifacts**: [docs/plans/]

### FIX-PLAN-02: Traceability from RCA Report's Fix Strategy to tasks
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all Fix Strategy items with identifiers from the RCA Report's Fix Strategy section
  2. Enumerate all tasks from the fix plan document's task section
  3. For each Fix Strategy item, verify at least one corresponding task exists in the fix plan document
  4. List any Fix Strategy items that have no corresponding task
- **pass_condition**: Step 3: all Fix Strategy items have at least one corresponding task. Step 4: the list is empty (zero items)
- **fail_diagnosis_hint**: Identify Fix Strategy items without corresponding tasks and add the missing tasks to the fix plan document. Create a mapping table of RCA Fix Strategy item IDs to fix plan task IDs to visualize gaps
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md]

### FIX-PLAN-03: Task granularity is sub-agent executable
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Count the description length (lines) and number of steps in each task of the fix plan document
  2. Verify each task has 10 or fewer steps
  3. Verify no single task contains multiple independent feature changes (detect tasks spanning 3 or more modules)
- **pass_condition**: All tasks have 10 or fewer steps and each task's change scope spans fewer than 3 modules. Zero tasks exceed these limits
- **fail_diagnosis_hint**: Identify tasks exceeding the step count as split candidates. For tasks spanning many modules, consider splitting by module boundary
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### FIX-PLAN-04: Task dependencies are explicit and consistent (no cycles)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Extract all task IDs and each task's dependency target IDs from the fix plan document
  2. Verify all referenced dependency target IDs exist within the fix plan document (no dangling references)
  3. Build a dependency graph and verify no circular dependencies exist (no paths like A->B->C->A)
  4. Check for tasks without explicit dependencies that actually depend on another task's output
- **pass_condition**: Step 2: zero dangling ID references. Step 3: zero circular paths. Step 4: zero implicit dependencies
- **fail_diagnosis_hint**: If circular dependencies are detected, consider splitting tasks or reversing the dependency direction. For dangling references, determine whether it is a typo or a missing task. For implicit dependencies, make the input/output relationship between tasks explicit
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### FIX-PLAN-05: Test cases are specified in Given/When/Then format
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Search the fix plan document's test case section with `Grep` for the Given/When/Then pattern
  2. Verify each test case contains all three elements: Given (preconditions), When (action), Then (expected result)
  3. Verify that Then clauses contain verifiable expected values such as numeric thresholds or pattern-matchable assertions
- **pass_condition**: All test cases include all 3 elements of Given/When/Then (step 2), and Then clauses contain verifiable expected values (step 3). Zero test cases are missing elements
- **fail_diagnosis_hint**: Identify test cases missing Given/When/Then elements and supplement with specific preconditions, actions, and expected results by referencing the RCA Report's Root Cause and Reproduction Test sections
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]
- **forward_check**: During the Execute phase, test cases should be directly translatable from Given/When/Then into test code

### FIX-PLAN-06: Fix plan document is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/plans/*-fix-plan.md` and confirm the fix plan document is not in the uncommitted changes list.
- **pass_condition**: `git status --porcelain` output does not contain the fix plan document path (zero output lines)
- **fail_diagnosis_hint**: If the fix plan document is uncommitted, `git add` + `git commit` may not have been executed. Check the final step of the Phase executor for the commit operation
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### FIX-PLAN-07: Narrative note captures plan decomposition
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-fix-plan.md`
  2. Verify frontmatter contains `phase: fix-plan` and `run_id: <run_id>`
  3. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify Decisions records task decomposition rationale tracing back to RCA Fix Strategy
  5. Verify Directives records test-first requirements and regression scope for the execute phase
- **pass_condition**: Steps 1-5 all pass
- **fail_diagnosis_hint**: If Decisions lacks RCA traceability, re-read the RCA report's Fix Strategy and re-derive task boundaries. If Directives empty, articulate which regression test set must run during execute. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-fix-plan.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
