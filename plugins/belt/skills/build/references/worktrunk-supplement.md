---
name: worktrunk-supplement
description: >-
  build stage integrate phase only. Read BEFORE invoking /worktrunk to define
  the merge-vs-PR user choice flow, pre-merge checks, and the PR-body template.
---

# Worktrunk Supplement for the build stage (integrate)

Read BEFORE invoking `/worktrunk` in the integrate phase.

## Required User Prompt

At the start of the integrate phase, present exactly:

```
Select integration mode:
(A) merge — run `wt merge` to parent branch, then `wt remove` this worktree
(B) PR    — run `gh pr create` with an auto-generated body, keep worktree
```

Wait for explicit (A) or (B). Do not proceed on any other input.

## Branch Naming

- Feature runs: `feature/<YYYY-MM-DD-topic>`
- Bug runs: `bugfix/<YYYY-MM-DD-topic>`

See `plugins/belt/skills/design/references/path-convention.md`.

## Pre-merge Checks

Before invoking `/worktrunk`:

1. Project test suite (e.g. `cargo test`) exit 0
2. Project linter (e.g. `cargo clippy --workspace -- -D warnings`) exit 0
3. Formatter check (e.g. `cargo fmt --check`) exit 0 for modified packages
4. `belt lint` exit 0 for any modified pipeline.yml files
5. Bug runs: the reproduction test (from the RCA report) PASSes on this branch

If any check fails, abort the integrate phase and report to the user — do
NOT merge.

## (A) Merge Flow

1. Ensure worktree is clean (`git status` shows no uncommitted changes).
2. Invoke `/worktrunk` with `wt merge`. This runs the project's pre-merge
   hook (typically tests + build). Abort if the hook fails.
3. After a successful fast-forward merge, invoke `wt remove` to delete the
   worktree.
4. Record the merged commit SHA.

## (B) PR Flow

1. Ensure worktree is clean.
2. Push the branch to origin (`git push -u origin <branch>`).
3. Run `gh pr create --title "<title>" --body "<body>"` with:
   - title: `feat: <topic>` (from `docs/features/<topic>/` directory name;
     strip the date prefix)
   - body: use the template below.
4. Record the PR URL.
5. Do NOT run `wt merge` or `wt remove`.

## PR Body Template

```markdown
## Summary

<One paragraph from the "Summary" or opening section of design.md.>

## Changes

<Bulleted list of task titles from plan.md, Task 1..N.>

## Testing

### Code Review
- Findings: { critical: N, high: N, medium: N, low: N }
- Status: (all addressed | N outstanding — see comments)

### Monkey Test (when args.e2e)
- Scenarios: <total>; Passed <n>; Failed <n>; Skipped <n>
- Link: docs/features/<topic>/monkey-test-report.md

### Dogfood (when args.e2e)
- New issues: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N
- Link: docs/features/<topic>/dogfood-report/report.md

## Must-Verify Checklist

<Table copied from dogfood-report.md's "Must-Verify Checklist Verification"
section. If args.e2e is false, copy directly from design.md with an
"Unverified (no e2e run)" note.>

## Spec and Plan

- Spec: <link to design.md at the merged SHA>
- Plan: <link to plan.md at the merged SHA>
```

## Commit Message Convention (bug runs)

Fix commits (from the execute phase) should follow:

```
fix(<scope>): <short description of bug fix>
```

Where `<scope>` is derived from the RCA Impact Scope (primary module).
Example: `fix(auth): redirect expired session cookies to /login instead of 500`.

## Completion Criteria (for the integrate gate)

- User explicitly selected (A) or (B).
- (A): merge commit exists in parent branch; worktree removed.
- (B): PR URL exists; body populated from template (no template placeholder
  text like `<...>` remains in the published body).
