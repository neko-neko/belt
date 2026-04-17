---
name: integrate-done-criteria
audit: lite
phase: integrate
---

# Phase 8 (integrate) Done Criteria

- **INT-01**: User explicitly selected (A) merge or (B) PR per the
  `references/worktrunk-supplement.md` prompt.
- **INT-02** (A selected): a merge commit containing the feature branch
  exists on the parent branch, the pre-merge hook succeeded, and the
  worktree has been removed (`wt list` does not list
  `feature/<YYYY-MM-DD-topic>`).
- **INT-03** (B selected): a PR exists on origin with a non-empty body
  whose sections (`Summary`, `Changes`, `Testing`, `Must-Verify Checklist`,
  `Spec and Plan`) are populated from the template with no literal
  `<...>` placeholder remaining.
- **INT-04**: All phase produces artifacts for the feature are:
  - (A) present in the parent branch at the merge commit, OR
  - (B) present at the PR head commit.
- **INT-05**: No uncommitted changes remain in the worktree (A only
  applicable before the `wt remove`).
