---
name: feature-dev
description: >-
  Quality-gated feature pipeline from ticket to integration: goal-sheet
  intake, single design document with spec review, context-reset
  checkpoint, TDD implementation, two-agent code review, optional browser
  verification, and integration — with an evidence.md trail. Accepts a
  Linear id, URL, or free-text task. --e2e enables browser verification;
  --codex enables adversarial review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--e2e] [--codex]"
---

# feature-dev

Composed pipeline: design stage → checkpoint → build stage. `belt-agent
init` expands the three `invoke.pipeline` references into namespaced
leaves (`design/intake` ... `build/integrate`) in a single run.

Keep the user's original task input (ticket id, URL, or free text): the
`design/intake` phase passes it verbatim to `/belt:goal`.

## Stage skills

When `next` returns a phase, read the owning stage's SKILL.md before
executing it:

- `design/*` → `plugins/belt/skills/design/SKILL.md`
- `pre-execute-handover/*` → run `/belt:handover`, then `/clear`, then
  `/belt:resume` in the new session
- `build/*` → `plugins/belt/skills/build/SKILL.md`

Smaller runs: `/belt:design` (design only), `/belt:build` (plan already
exists), `/belt:goal` (intake only), `/belt:verify` (browser check only).

## Red flags

- Never execute a stage phase without its stage SKILL.md loaded.
- Never bypass the pre-execute-handover checkpoint — the context reset
  before execute is the pipeline's core ergonomics.
