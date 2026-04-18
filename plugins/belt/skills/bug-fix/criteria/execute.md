---
name: execute
max_retries: 3
audit: required
---

## Criteria

### EXECUTE-01: Code changes exist for every task
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all task IDs from the plan document
  2. Retrieve the list of changed files using `git diff --name-only`
  3. For each task ID, verify at least one corresponding code change exists among the changed files
  4. List task IDs that have no corresponding code changes
- **pass_condition**: Step 4 list is empty (all tasks have corresponding code changes)
- **fail_diagnosis_hint**: Identify task IDs without code changes and check the corresponding tasks in the plan document. Determine whether it is a missing implementation or a documentation-only task whose changes do not appear in git diff
- **depends_on_artifacts**: [docs/plans/*-plan.md]

### EXECUTE-02: Build/compilation succeeds
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's build command (`npm run build`, `cargo build`, `go build ./...`, etc.) and record the exit code. Identify the build command from package.json, Cargo.toml, go.mod, Makefile, etc.
- **pass_condition**: Build command exit code is 0
- **fail_diagnosis_hint**: Check the first error message in the build error log. Identify whether it is a type error, import resolution failure, or missing dependency package. Report the affected file and line number
- **depends_on_artifacts**: [src/, artifacts/build/]

### EXECUTE-03: No lint or type-check errors
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's linter/type checker (`npm run lint`, `eslint`, `tsc --noEmit`, `cargo clippy`, `ruff check`, etc.) and record the error count.
- **pass_condition**: Linter/type checker exit code is 0, with zero error-level findings
- **fail_diagnosis_hint**: Check the file path and line number of each error finding. For type errors, check for type definition inconsistencies. For lint errors, review coding convention violations. Determine whether `--fix` can auto-correct the issues
- **depends_on_artifacts**: [src/, artifacts/lint/]

### EXECUTE-04: Full test suite passes
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's test command (`npm test`, `cargo test`, `pytest`, `go test ./...`, etc.) and record the test result summary (total, passed, failed, skipped).
- **pass_condition**: Test command exit code is 0 with zero failed tests
- **fail_diagnosis_hint**: Check the names and error messages of failed tests. Use `git diff -- tests/` to distinguish between regressions of existing tests and first-time failures of new tests. For regressions, run `git stash && test command` to compare against the baseline
- **depends_on_artifacts**: [tests/, artifacts/test-results/]

### EXECUTE-05: Test code exists for every planned test case
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all test cases from the plan document's test case section
  2. Extract all test function/method names from the test code directories (tests/, __tests__/, spec/, etc.) using `Grep`
  3. For each planned test case, verify at least one corresponding test code implementation exists
  4. List planned test cases without corresponding test code
- **pass_condition**: Step 4 list is empty (all planned test cases have corresponding test code)
- **fail_diagnosis_hint**: Identify test cases without corresponding test code and check the naming conventions. If test function names differ from planned test case names, verify correspondence by content inspection
- **depends_on_artifacts**: [docs/plans/*-plan.md, tests/]

### EXECUTE-06: Implementation respects component boundaries
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the component boundaries (module partitioning, layer architecture) defined in the design document
  2. Retrieve changed files using `git diff --name-only` and identify which component each file belongs to
  3. Verify that changes do not introduce new direct dependencies (import/require) that cross component boundaries
- **pass_condition**: Zero new direct dependencies that cross design-defined boundaries
- **fail_diagnosis_hint**: Identify the boundary-violating import/require statements and cross-reference with the design document's component diagram. Check for cases where dependencies should go through an interface layer instead of direct references
- **depends_on_artifacts**: [docs/plans/*-design.md, src/]

### EXECUTE-07: End-to-end traceability from design to plan to implementation
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all requirements from the design document
  2. Identify the plan document task ID corresponding to each requirement
  3. Identify the implementation file/function corresponding to each task ID (using `git diff` + code search)
  4. Verify no gaps exist in the requirement-to-task-to-implementation three-tier mapping
  5. Verify no "surplus implementation" exists that has no corresponding requirement or task
- **pass_condition**: Step 4: zero gaps in the three-tier mapping. Step 5: zero implementation files without corresponding plan tasks
- **fail_diagnosis_hint**: Identify where the mapping gap occurs (between requirement-to-task or task-to-implementation). For surplus implementations, determine whether to add them to the design/plan documents or remove the excess code
- **depends_on_artifacts**: [docs/plans/*-design.md, docs/plans/*-plan.md, src/]

### EXECUTE-08: Newly added tests are not tautological
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List new or modified test files using `git diff --name-only -- tests/ __tests__/ spec/`
  2. Extract assertion statements (expect, assert, should, etc.) from each test file
  3. Verify that assertions exercise actual implementation code logic (i.e., call the function/method under test and verify its result)
  4. Check that assertions are not: constant-to-constant comparisons (`expect(1).toBe(1)`), mock return value verifications only (bypassing real code paths), or empty tests (no assertions)
- **pass_condition**: Zero tautological tests matching step 4 criteria. Every test includes at least one assertion that exercises a real implementation code path
- **fail_diagnosis_hint**: Identify tautological tests by file path and line number. Determine whether it is excessive mocking, missing assertions, or failure to invoke the function under test. Rewrite assertions to verify the actual behavior of implementation code
- **depends_on_artifacts**: [tests/, src/]

### EXECUTE-09: Test cases cover both requirement coverage and impact scope
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all requirements from the design document and assign IDs
  2. Enumerate the impacted files/modules from the belt-agent:impact-analyzer output
  3. For each requirement ID, verify at least one corresponding test case exists
  4. For each impacted file/module, verify corresponding tests exist
  5. Verify no existing tests for impacted code have been deleted or disabled (detect additions of `.skip`, `.only`, test function commenting out, or deletion via `git diff`)
- **pass_condition**: Step 3: all requirements have corresponding tests. Step 4: all impacted areas have corresponding tests. Step 5: zero deletions or disablements of existing tests
- **fail_diagnosis_hint**: For step 3 gaps, identify missing requirement IDs and add tests. For step 4 gaps, identify uncovered impacted files and assess whether existing tests exist or new ones are needed. For step 5 detections, verify whether the deletion/disablement was intentional; if not, restore the tests
- **depends_on_artifacts**: [docs/plans/*-design.md, tests/, src/]
- **forward_check**: Prevents "insufficient test coverage" findings during the Code Review phase

### EXECUTE-10: Narrative note captures phase decisions and directives
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `execute_notes` resolved_path
  2. Verify the file exists at the resolved_path (gate already enforces existence, re-confirm)
  3. Verify frontmatter contains both `phase: execute` and `run_id: <run_id>` fields
  4. Verify 4 required sections exist in order: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify each section is either populated or retains its heading (empty sections may carry `(none)` placeholder but heading must be present)
  6. Verify Decisions / Directives are specific enough for a `/clear`-ed LLM to reconstruct the phase outcome (not vague generalities)
- **pass_condition**: Steps 1-5 all pass; Step 6 narrative is concrete (references task IDs / file paths / decisions made, not abstract statements)
- **fail_diagnosis_hint**: If heading missing, add empty heading. If frontmatter missing, copy `run_id` from `belt-agent step` / `belt-agent status` JSON output. If content is vague, rewrite to cite concrete artifacts (task IDs, file paths, decision triggers). See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [execute_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
