---
name: dogfood-supplement
description: >-
  feature-dev Phase 7 only. Read BEFORE invoking /dogfood to override output
  directory, scope exploration to the feature diff, filter severity, and
  inject prior-phase artifacts as exploration hints.
---

# Dogfood Supplement for feature-dev

Read BEFORE invoking `/dogfood` in Phase 7. This phase only runs when
`args.e2e` is true. Path convention reference: `./path-convention.md`.

## Output Path Override

```
docs/features/<topic>/dogfood-report/
├── report.md
├── screenshots/
└── videos/
```

This overrides /dogfood's default `./dogfood-output/`.

## Scope Override

Restrict exploration to code areas changed by the feature branch:

```bash
git diff <base>..HEAD --name-only
```

Map changed files to corresponding UI pages/components; prioritize those.
Do NOT explore the full site.

## Severity Filter

- `critical` and `high` issues: full detail in report.md primary section.
- `medium` and `low` issues: summary only (counts + one-line description).

## Context Injection (read these BEFORE starting exploration)

### 1. `docs/features/<topic>/design.md`
Focus on:
- **Prerequisites** — a violation is a likely bug.
- **Impact Scope** — modules where side-effects may surface.
- **Impact Analysis > Side Effect Risks** — attempt to reproduce each risk.
- **Must-Verify Checklist** — VERIFY EVERY ITEM during dogfood.

### 2. `docs/features/<topic>/test-strategy.md`
Focus on:
- **Non-functional requirements** (performance, security, accessibility) —
  these are typically uncovered by scripted tests.
- **Boundary / state-transition** items requiring exotic combinations.

### 3. `docs/features/<topic>/scenarios.yml`
Use to AVOID redundant exploration of scripted happy paths. Spend effort on
combinations NOT in scenarios.yml (e.g., scenario A then B, mid-flow
interrupt, concurrent operations, long-idle resumes).

### 4. `docs/features/<topic>/monkey-test-results.json`
- Read all `FAIL` entries. Retry each by hand.
  - Still broken → file as "Known issue re-encountered" (do not double-count
    as a new finding).
  - Fixed → note as "Previously failed, now passing".
- Read all `SKIP` entries. Verify that the SKIP reason still holds in the
  current build.

### 5. `docs/features/<topic>/plan.md`
Read for context on implementation scope (do not re-verify every task).

## Exploration Priority

1. Verify every item in the Must-Verify Checklist from `design.md`.
2. Attempt to reproduce every Side Effect Risk from Impact Analysis.
3. Exercise non-functional requirements from `test-strategy.md`.
4. Combinations and exotic cases not covered by `scenarios.yml`.
5. Surface UI/UX bugs (typos, misalignment, console errors, a11y).

Scripted happy paths: verify existence only (smoke confirm), do not deep-test.

## Report Structure

```markdown
# Dogfood Report: <feature-name>

## Summary
- Exploration time: XX min
- Pages visited: N
- New issues found: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N (from monkey-test-results.json)
- Must-Verify Checklist: X/Y items verified (list any unverified)

## Critical and High Issues (new findings)
<per-issue: id, severity, repro steps, screenshot/video evidence>

## Must-Verify Checklist Verification
<table: item, status (PASS/FAIL/N/A), notes>

## Known Issues Re-encountered
<per-issue: scenarios.yml id, status from monkey-test, dogfood observation>

## Medium and Low Issues (summary)
<counts plus one-line descriptions>
```

## Completion Criteria (for Phase 7 gate)

- `docs/features/<topic>/dogfood-report/report.md` exists and is committed.
- Every Must-Verify Checklist item has a verification status in report.md.
- Every `FAIL` scenario in `monkey-test-results.json` is addressed in
  "Known Issues Re-encountered".
- Either ≥ 5 new issues are well-documented with evidence, OR the report
  explicitly states "No critical or high issues found" with rationale.
