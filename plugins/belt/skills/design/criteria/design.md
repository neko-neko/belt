---
name: design
max_retries: 3
audit: lite
---

## Criteria

### DESIGN-01: Design document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/design.md")`
  2. Run `git status --porcelain -- docs/features/*/design.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes for the matched file
- **fail_diagnosis_hint**: Confirm the design phase wrote the document under `docs/features/<YYYY-MM-DD-topic>/` (see `../references/path-convention.md`) and committed it inside the feature worktree
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-02: Required sections are present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the design document
  2. Verify headings exist: `Prerequisites`, `Impact Scope`, `Impact Analysis` (with subsections `Reverse Dependencies`, `Shared State`, `Implicit Contracts`, `Side Effect Risks`), `Must-Verify Checklist`, `Test Perspectives`
- **pass_condition**: Zero missing headings from the list above
- **fail_diagnosis_hint**: The brainstorming supplement defines the required sections — resume the design conversation for the missing ones
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-03: Test Perspectives covers the four case classes
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the `Test Perspectives` section
  2. Verify at least one case exists for EACH of: normal, boundary, abnormal, state-transition
- **pass_condition**: All four classes have at least one case
- **fail_diagnosis_hint**: Derive the missing classes from the design's input parameters and state model
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-04: Feature worktree branch exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Take the topic directory name from the DESIGN-01 Glob match
  2. Run `git branch --list "feature/<YYYY-MM-DD-topic>"` and confirm the branch is listed
- **pass_condition**: The branch exists and its name matches the `docs/features/` directory name
- **fail_diagnosis_hint**: The worktree creation order in the brainstorming supplement was skipped — create the worktree/branch now
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-05: Baseline tests pass at the design commit
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Run the project-appropriate test command in the worktree (or read the worktrunk pre-start hook output captured at worktree creation) and record the exit code.
- **pass_condition**: Test command exit code is 0
- **fail_diagnosis_hint**: A red baseline invalidates later execute-phase regression attribution — fix the baseline or record the known failures before proceeding
- **depends_on_artifacts**: []

### DESIGN-06: Worktree is clean after the design commit
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain` in the worktree and confirm zero output lines.
- **pass_condition**: `git status --porcelain` output is empty
- **fail_diagnosis_hint**: Commit or intentionally discard the stragglers
- **depends_on_artifacts**: []

### DESIGN-07: Narrative note captures design decisions
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `design_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: design` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Decisions records the chosen approach and rejected alternatives with rationale
  6. Verify Directives records constraints for the plan / execute phases
- **pass_condition**: Steps 1-6 all pass; empty sections may carry `(none)` but headings must be present
- **fail_diagnosis_hint**: If Decisions lacks rejected alternatives, re-derive them from the brainstorming dialogue. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [design_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
