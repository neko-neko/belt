---
name: integrate
max_retries: 3
audit: lite
---

## Criteria

### INTEGRATE-01: Integration method was chosen by the user and executed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The worktrunk-supplement A/B prompt was presented to the user
  2. Either `wt merge` (option A) or `gh pr create` (option B) was executed
  3. Execution logs (or git state) reflect the chosen method
- **pass_condition**: One of the two methods was executed per an explicit user choice
- **fail_diagnosis_hint**: If no explicit choice is recorded, re-present the A/B prompt — never default silently
- **depends_on_artifacts**: []

### INTEGRATE-02: All pre-merge checks pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Project test suite (e.g. `cargo test`) exit 0
  2. Project linter (e.g. `cargo clippy --workspace -- -D warnings`) exit 0
  3. Formatter check (e.g. `cargo fmt --check`) exit 0 for modified packages
  4. `belt lint` exit 0 for any modified pipeline.yml files
- **pass_condition**: All applicable checks exit 0
- **fail_diagnosis_hint**: Fix failures before integration; a red pre-merge check must never be merged around
- **depends_on_artifacts**: []

### INTEGRATE-03: Merge flow completed (A selected)
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. If option B was selected, PASS (vacuously satisfied)
  2. Verify a merge commit containing the branch exists on the parent branch and the pre-merge hook succeeded
  3. Verify the worktree has been removed (`wt list` no longer lists the branch)
- **pass_condition**: Option B, OR (merge commit exists AND worktree removed)
- **fail_diagnosis_hint**: If the pre-merge hook failed, resolve and re-run `wt merge`; if the worktree remains, run `wt remove`
- **depends_on_artifacts**: []

### INTEGRATE-04: PR flow completed (B selected)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If option A was selected, PASS (vacuously satisfied)
  2. Verify a PR exists on origin with a non-empty body whose sections (`Summary`, `Changes`, `Testing`, `Must-Verify Checklist`, `Spec and Plan`) are populated from the worktrunk-supplement template
  3. Verify no literal `<...>` placeholder remains in the published body
- **pass_condition**: Option A, OR (PR exists AND all template sections populated AND zero placeholders)
- **fail_diagnosis_hint**: Re-generate the body from the template and `gh pr edit` the published PR
- **depends_on_artifacts**: []

### INTEGRATE-05: All produced artifacts are present at the integrated commit
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Enumerate the run's produced domain artifacts under `docs/features/<topic>/`
  2. Verify each is present in the parent branch at the merge commit (A) or at the PR head commit (B)
- **pass_condition**: Zero missing artifacts at the integrated commit
- **fail_diagnosis_hint**: An uncommitted or unpushed artifact — commit it and amend the merge/PR
- **depends_on_artifacts**: [docs/features/]

### INTEGRATE-06: Reproduction test PASSes on the integrated branch (bug runs)
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. If `docs/features/<topic>/rca-report.md` does not exist (feature runs), PASS (vacuously satisfied)
  2. Re-run the reproduction test identified in the RCA report's Reproduction Test section on the integrated branch (post-merge main, or the PR head)
  3. Confirm it PASSes (it FAILed pre-fix)
- **pass_condition**: Non-bug run, OR reproduction test PASSes on the integrated branch
- **fail_diagnosis_hint**: The merge introduced a regression or test expectations drifted — review the integration diff
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### INTEGRATE-07: No uncommitted changes remain
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain` in the worktree (A: before `wt remove`; B: at PR head) and confirm zero output lines.
- **pass_condition**: `git status --porcelain` output is empty
- **fail_diagnosis_hint**: Commit or intentionally discard the stragglers before closing the phase
- **depends_on_artifacts**: []

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
