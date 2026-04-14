---
name: design-done-criteria
audit: lite
phase: design
---

# Phase 1 (design) Done Criteria

All items must be satisfied for the phase to pass.

- **DESIGN-01**: `docs/features/<topic>/design.md` exists and is committed
  inside the feature worktree.
- **DESIGN-02**: The document contains all required sections:
  - `Prerequisites`
  - `Impact Scope`
  - `Impact Analysis` with subsections
    `Reverse Dependencies`, `Shared State`, `Implicit Contracts`,
    `Side Effect Risks`
  - `Must-Verify Checklist`
  - `Test Perspectives`
- **DESIGN-03**: `Test Perspectives` covers at minimum one case for EACH of:
  normal, boundary, abnormal, state-transition.
- **DESIGN-04**: Worktree branch `feature/<YYYY-MM-DD-topic>` exists
  (verify with `git branch --list`).
- **DESIGN-05**: Baseline tests pass in the worktree at the time of
  `design.md` commit (verify via the worktrunk pre-start hook output or a
  fresh `cargo test` / project-appropriate test command).
- **DESIGN-06**: `git status` in the worktree is clean after the `design.md`
  commit.
