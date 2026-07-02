---
name: dogfood-supplement
description: >-
  verify stage only. Read BEFORE invoking /dogfood to override the output
  directory, scope exploration to the change diff, filter severity, and inject
  prior-phase artifacts as exploration hints.
---

# Dogfood Supplement for the verify stage

Read BEFORE invoking `/dogfood`. Path convention reference:
`plugins/belt/skills/design/references/path-convention.md`.

## Output Path Override

```
docs/features/<topic>/dogfood-report/
├── report.md
├── screenshots/
└── videos/
```

This overrides /dogfood's default `./dogfood-output/`. Always create the
`screenshots/` and `videos/` directories (empty is acceptable) to keep the
artifact structure consistent.

## Scope Override

Restrict exploration to code areas changed by the branch:

```bash
git diff <base>..HEAD --name-only
```

Map changed files to corresponding UI pages/components; prioritize those.
Do NOT explore the full site. Bug runs additionally prioritize:

1. **Impact Scope areas** from `fix-plan.md` — the files and modules modified by the fix
2. **Symmetry pairs** from the RCA report — paired paths that may exhibit the same mechanism
3. **Root Cause mechanism re-emergence** — verify the mechanism described in
   the RCA `## Root Cause` section does not recur in adjacent code paths, and
   include an explicit statement in the report:

   > After the fix, the <mechanism description from RCA Root Cause> condition
   > no longer triggers. Verified by: <specific exploration path / inputs>.

   This satisfies `criteria/dogfood.md` DOGFOOD-03.

## Severity Filter

- `critical` and `high` issues: full detail in report.md primary section.
- `medium` and `low` issues: summary only (counts + one-line description).

## Context Injection (read those that exist BEFORE starting exploration)

### 1. `docs/features/<topic>/design.md` (feature runs)
Focus on: **Prerequisites** (a violation is a likely bug), **Impact Scope**,
**Impact Analysis > Side Effect Risks** (attempt to reproduce each risk), and
**Must-Verify Checklist** (VERIFY EVERY ITEM during dogfood).

### 2. `docs/features/<topic>/rca-report.md` + `fix-plan.md` (bug runs)
Focus on: **Root Cause** mechanism (re-verification target), **Symmetry
Check** pairs, and the fix's **Impact Scope**.

### 3. `docs/features/<topic>/test-strategy.md`
Focus on non-functional requirements and boundary / state-transition items
requiring exotic combinations — typically uncovered by scripted tests.

### 4. `docs/features/<topic>/scenarios.yml` / `rca-scenarios.yml`
Use to AVOID redundant exploration of scripted paths. Spend effort on
combinations NOT in the scenarios file (scenario A then B, mid-flow
interrupt, concurrent operations, long-idle resumes).

### 5. `docs/features/<topic>/monkey-test-results.json`
- Read all `FAIL` entries. Retry each by hand: still broken -> file as
  "Known issue re-encountered" (do not double-count); fixed -> note as
  "Previously failed, now passing".
- Read all `SKIP` entries. Verify the SKIP reason still holds.

## CLI-only Graceful Degradation (UI-free changes)

When the change scope contains **zero UI files** (CLI / API / backend-only):

1. Substitute visual exploration with CLI output capture (stdout / stderr),
   API response inspection (JSON / headers), log file inspection, and DB
   state queries.
2. DOGFOOD-05's evidence requirement is satisfied by a rationale paragraph
   in `report.md`:

   > The change scope contains no UI files (<list affected paths>).
   > Exploration is CLI-only; evidence is captured as CLI output / API
   > response / log excerpts in this report.

## Report Structure

```markdown
# Dogfood Report: <topic>

## Summary
- Exploration time: XX min
- Pages visited: N
- New issues found: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N (from monkey-test-results.json)
- Must-Verify Checklist: X/Y items verified (feature runs; list any unverified)

## Critical and High Issues (new findings)
<per-issue: id, severity, repro steps, screenshot/video evidence>

## Must-Verify Checklist Verification  (feature runs)
<table: item, status (PASS/FAIL/N/A), notes>

## Root Cause Re-verification  (bug runs)
<explicit mechanism re-verification statement + exploration coverage map>

## Known Issues Re-encountered
<per-issue: scenario id, status from monkey-test, dogfood observation>

## Medium and Low Issues (summary)
<counts plus one-line descriptions>
```

## Completion Criteria (for the dogfood gate)

- `docs/features/<topic>/dogfood-report/report.md` exists and is committed.
- Feature runs: every Must-Verify Checklist item has a verification status.
- Bug runs: the Root Cause re-verification statement is present.
- Every `FAIL` scenario in `monkey-test-results.json` is addressed in
  "Known Issues Re-encountered".
- Evidence files exist, or the CLI-only rationale paragraph is present.
