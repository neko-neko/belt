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
- **fail_diagnosis_hint**: Identify the missing perspective and check the /code-review:code-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
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

## Observation Collection

The belt-agents:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
