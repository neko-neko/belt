---
name: feature-dev
description: >-
  Quality-gated feature pipeline from ticket to integration: goal-sheet
  intake, design document with spec review, implementation plan with QA
  scenarios, context-reset checkpoint, TDD implementation, autonomous
  code review, mandatory QA with human-readable evidence (screenshots /
  transcripts), and integration with evidence publishing. Accepts a
  Linear id, URL, free text, or a requirements.md path. --codex enables
  adversarial review.
user-invocable: true
argument-hint: "<linear-id | url | free-text | requirements.md path> [--codex]"
---

# feature-dev

Composed pipeline: design → plan → checkpoint → build → qa → integrate.
`belt-agent init` expands the five `invoke.pipeline` references into
namespaced leaves (`design/intake` ... `qa/qa`) plus the `integrate`
leaf in a single run.

Keep the user's original task input (ticket id, URL, free text, or
requirements.md path): the `design/intake` phase passes it verbatim to
`/belt:goal`.

Human touchpoints are exactly four: design approval, plan approval, the
checkpoint pause, and integrate. build and qa run autonomously; their
leftovers (deferred findings, accepted FAILs, QA fix commits,
exploratory advisories) are reported at integrate.

## Stage skills

When `next` returns a phase, read the owning stage's SKILL.md before
executing it:

- `design/*` → `plugins/belt/skills/design/SKILL.md`
- `plan/*` → `plugins/belt/skills/plan/SKILL.md`
- `pre-execute-handover/*` → run `/belt:handover`, then `/clear`, then
  `/belt:resume` in the new session
- `build/*` → `plugins/belt/skills/build/SKILL.md`
- `qa/*` → `plugins/belt/skills/qa/SKILL.md`
- `integrate` → this file, below

## Phase: integrate

Ask the user once: A) `wt merge` or B) `gh pr create`, presenting in
the same message the deferred findings, accepted FAILs, QA fix commits,
and exploratory advisories collected in evidence.md. Pass the chosen
mode as the argument to the invoke declared in `pipeline.yml`. Then
publish QA evidence per the `[qa] evidence` config (see
`plugins/belt/skills/qa/SKILL.md`):

- PR route: push the QA evidence directory to the `qa-evidence` orphan
  branch under `<run-id>/`, then post one PR comment containing the
  qa-report scenario table with evidence links (public repos: inline
  image embeds via raw URLs; private repos: blob URL links).
- No PR: attach the evidence to the Linear issue when an id is known
  (same upload and fallback rules as the qa phase); otherwise report
  the local evidence path to the user with an explicit warning.

Record the published destination URL in evidence.md's integrate entry.

## Red flags

- Never execute a stage phase without its stage SKILL.md loaded.
- Never bypass the pre-execute-handover checkpoint — the context reset
  before execute is the pipeline's core ergonomics.
- Never merge or create the PR before qa-report.md exists.

## Smaller runs

`/belt:design` (design only), `/belt:plan` (plan from an existing
design), `/belt:build` (plan already exists), `/belt:qa` (QA only),
`/belt:goal` (intake only), `/belt:requirements` (requirements
document, upstream of this pipeline).
