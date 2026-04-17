---
name: code-review
max_retries: 3
audit: required
---

## Criteria

### CODE-REVIEW-01: Review executed across all 7 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/code-review-review.json` or review log) and confirm execution records exist for all 7 perspectives (simplify, code-quality, code-security, code-performance, code-test, ai-antipattern, code-impact).
- **pass_condition**: Execution records exist for all 7 perspectives. Recorded perspective count is 7
- **fail_diagnosis_hint**: Identify the missing perspective and check the /code-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### CODE-REVIEW-02: All user-approved findings have been fixed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all user-approved findings from the review results
  2. Identify the target file and line for each finding
  3. Read the target file location using `Read` and verify the fix corresponding to each finding has been applied
  4. List findings where the fix has not been applied
- **pass_condition**: Step 4 list is empty (all user-approved findings have fixes applied)
- **fail_diagnosis_hint**: Identify unapplied findings and check the target file at the relevant line. Determine whether the fix was omitted or applied in a different form. Use `git log --oneline` to check for the existence of fix commits
- **depends_on_artifacts**: [artifacts/reviews/, src/]

### CODE-REVIEW-03: No uncommitted changes and branch is within 50 commits of latest main
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git status --porcelain` to detect uncommitted changes
  2. Run `git rev-list --count HEAD ^main` (or `^master` if main does not exist) to get the branch divergence commit count
- **pass_condition**: Step 1 output is empty (zero uncommitted changes) and step 2 commit count is 50 or fewer
- **fail_diagnosis_hint**: If uncommitted changes exist, check for missed `git add` + `git commit` execution. If divergence exceeds 50 commits, consider rebasing onto the main branch. Long-lived branches carry high conflict risk; recommend early merging or branch splitting
- **depends_on_artifacts**: []

### CODE-REVIEW-04: High-severity impact findings have undergone user review
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Extract findings from the review result file where category is code-impact and severity is high or critical
  2. If zero such findings exist, PASS
  3. If one or more such findings exist, verify each has one of the following records:
     a. Fixed: a corresponding code change is included in a commit
     b. User-approved deferral: user's approval statement exists in the conversation log
     c. User-approved rejection: rationale for false positive was presented to the user and the user approved the rejection
  4. Verify no findings were deferred or rejected by the orchestrator without user confirmation
- **pass_condition**: All findings in step 3 fall under a, b, or c, and step 4 finds zero user-unconfirmed deferrals/rejections
- **fail_diagnosis_hint**: Identify user-unconfirmed findings and PAUSE to request user judgment. Check whether the orchestrator auto-deferred any findings without user confirmation
- **depends_on_artifacts**: [artifacts/reviews/]

### CODE-REVIEW-05: Narrative note captures review findings and directives
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-code-review.md`
  2. Verify frontmatter contains `phase: code-review` and `run_id: <run_id>`
  3. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify Decisions records which review findings were accepted / rejected and why
  5. Verify Directives flags carry-over concerns for downstream phases (e.g. regression tests to run in monkey-test)
- **pass_condition**: Steps 1-5 all pass; narrative records specific review outcomes not abstract "code reviewed"
- **fail_diagnosis_hint**: If Decisions lacks accept/reject rationale, re-read review findings and enumerate. If Directives empty, consider whether monkey-test / dogfood needs specific regression coverage. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-code-review.md]

### CODE-REVIEW-06: Merged findings.json exists at the canonical path
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/review/findings.json`
  2. Parse as JSON and confirm the `findings` array field is present
- **pass_condition**: File exists AND parses as valid JSON AND contains a `findings` array
- **fail_diagnosis_hint**: The `/belt:code-review` invocation was interrupted or the merge step was skipped. Re-invoke from the code-review phase
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json]

### CODE-REVIEW-07: All findings in findings.json have a user-approved disposition
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate every finding in `.belt/runs/<run_id>/review/findings.json`
  2. For each finding, verify the user has recorded a disposition (accepted for fix, or explicitly rejected)
  3. List findings lacking a user-approved disposition
- **pass_condition**: Step 3 list is empty — every finding has been triaged by the user
- **fail_diagnosis_hint**: Resume the triage loop in the main context; do NOT let the orchestrator auto-dispose findings. Each finding needs user acknowledgement of accept / reject
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json]

### CODE-REVIEW-08: All accepted findings are applied to the codebase
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Filter findings in `findings.json` where the user disposition is "accept" (or equivalent)
  2. For each accepted finding, verify the corresponding code change appears in `git diff` against the parent phase baseline
  3. List accepted findings with no corresponding diff
- **pass_condition**: Step 3 list is empty — every accepted finding has a matching diff
- **fail_diagnosis_hint**: Identify unapplied findings and apply them, or downgrade the disposition to "reject" with a recorded rationale. Use `git log --oneline` to verify fix commits exist
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json]

### CODE-REVIEW-09: Project linter passes on the modified files
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Identify modified crates / packages from `git diff --name-only`
  2. Run the project linter on those crates (Rust: `cargo clippy --package <pkg> -- -D warnings`; equivalent per language for other stacks)
  3. Capture exit code and any warnings
- **pass_condition**: Linter exits 0 with zero warnings on every modified crate
- **fail_diagnosis_hint**: Address lint findings before proceeding. If a lint must be suppressed, record the rationale in the code-review narrative
- **depends_on_artifacts**: []

### CODE-REVIEW-10: Project tests pass on the modified crates/packages
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Identify modified crates / packages from `git diff --name-only`
  2. Run the project test suite scoped to those crates (Rust: `cargo test -p <pkg>`; equivalent per language for other stacks)
  3. Capture exit code and failure count
- **pass_condition**: Test runner exits 0 with zero failing tests
- **fail_diagnosis_hint**: Failing tests indicate either a regression introduced by the fix or a pre-existing issue. Investigate, fix, and rerun before returning PASS
- **depends_on_artifacts**: []

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
