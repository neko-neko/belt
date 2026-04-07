---
name: design
max_retries: 3
audit: required
---

## Criteria

### DESIGN-01: Design document file exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Search for a design document file using `Glob("docs/plans/YYYY-MM-DD-*-design.md")`.
- **pass_condition**: At least one Glob result found
- **fail_diagnosis_hint**: Verify that the Phase executor output the design document under `docs/plans/`. Confirm the filename matches the `YYYY-MM-DD-*-design.md` pattern
- **depends_on_artifacts**: [docs/plans/]
- **forward_check**: The design document path will be passed as input to the Spec Review phase

### DESIGN-02: Required sections contain substantive content
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  Read each subsection of the Investigation Record and determine:
  1. prerequisites: Contains at least one dependency library name/version or file path reference
  2. impact_scope: Lists at least one target module or file for modification
  3. reverse_dependencies: Identifies at least one consumer of the changed code
  4. shared_state: Documents impact on global state, databases, caches, etc. (explicitly stating "none" is acceptable if no impact exists)
  5. implicit_contracts: Lists at least one implicit precondition
  6. side_effect_risks: Documents at least one potential side effect and its mitigation
- **pass_condition**: All 6 items above meet their respective criteria. Zero sections contain only headings or generic boilerplate
- **fail_diagnosis_hint**: Identify which numbered item failed and check the corresponding subsection in the Investigation Record. Cross-reference with impact-analyzer output to fill in missing information
- **depends_on_artifacts**: [docs/plans/*-design.md]

### DESIGN-03: Impact scope is consistent with the codebase
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Extract file/directory paths listed in the design document's impact_scope
  2. Verify each path exists using `Glob`
  3. For paths that do not exist, check whether the design document explicitly marks them as newly created files
- **pass_condition**: All listed paths either exist via Glob or are explicitly documented as new files. Zero paths are unaccounted for
- **fail_diagnosis_hint**: List the non-existent paths and determine whether they are typos or missing new-file declarations. Also check for discrepancies between the current codebase state and the state at the time the design document was written
- **depends_on_artifacts**: [docs/plans/*-design.md]

### DESIGN-04: Test perspectives cover 4 categories with at least 2 items each
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Read the test perspectives section of the design document and count items in each of the 4 categories (happy path, error handling, edge cases, non-functional). Use `Grep` to locate category headings or labels and count the items under each.
- **pass_condition**: All 4 categories have at least 2 items. Total count is 8 or more
- **fail_diagnosis_hint**: Identify the deficient categories and supplement the test perspectives section. For happy path/error handling gaps, review the requirements. For edge case gaps, analyze boundary values. For non-functional gaps, consider performance and security aspects
- **depends_on_artifacts**: [docs/plans/*-design.md]

### DESIGN-05: Alternatives have been evaluated
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Search the design document for an "Alternatives", "Alternative", or "Options" section or similar content
  2. If alternatives are documented, verify each has an adoption/rejection rationale
- **pass_condition**: An alternatives section exists and each option includes a rationale (at least one sentence). If no alternatives exist, the document explicitly states why with justification
- **fail_diagnosis_hint**: If no alternatives section is found, consider adding one. If rationale is missing, supplement with trade-off analysis covering cost, complexity, and maintainability
- **depends_on_artifacts**: [docs/plans/*-design.md]

### DESIGN-06: Worktree created and baseline tests pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git worktree list` and confirm the current branch is in a worktree separate from the main branch
  2. Execute the project's test command (package.json test script, Cargo test, pytest, etc.) and record the exit code
- **pass_condition**: A worktree exists (`git worktree list` output has 2 or more lines) and the test command exit code is 0
- **fail_diagnosis_hint**: If no worktree exists, verify that `wt switch -c` was executed. If tests fail, check whether pre-existing test failures exist on the base branch (main/master) by running `git stash && test command` to isolate the cause
- **depends_on_artifacts**: []

### DESIGN-07: Design document is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/plans/*-design.md` and confirm the design document is not in the uncommitted changes list.
- **pass_condition**: `git status --porcelain` output does not contain the design document path (zero output lines)
- **fail_diagnosis_hint**: If the design document is uncommitted, `git add` + `git commit` may not have been executed. Check the final step of the Phase executor for the commit operation
- **depends_on_artifacts**: [docs/plans/*-design.md]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
