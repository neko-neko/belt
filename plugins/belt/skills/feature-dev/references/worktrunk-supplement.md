---
name: worktrunk-supplement
description: >-
  feature-dev Phase 8 only. Read BEFORE invoking /worktrunk to define the
  merge-vs-PR user choice flow and the PR-body template.
---

# Worktrunk Supplement for feature-dev (Phase 8 Integrate)

Read BEFORE invoking `/worktrunk` in Phase 8.

## Required User Prompt

At the start of Phase 8, present exactly:

```
Select integration mode:
(A) merge — run `wt merge` to parent branch, then `wt remove` this worktree
(B) PR    — run `gh pr create` with an auto-generated body, keep worktree
```

Wait for explicit (A) or (B). Do not proceed on any other input.

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

## Completion Criteria (for Phase 8 gate)

- User explicitly selected (A) or (B).
- (A): merge commit exists in parent branch; worktree removed.
- (B): PR URL exists; body populated from template (no template placeholder
  text like `<...>` remains in the published body).
