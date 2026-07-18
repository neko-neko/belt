---
name: design
description: >-
  Runs the feature design stage: goal-sheet intake (/belt:goal) followed
  by a design document (architecture + key decisions) reviewed by
  belt:spec-reviewer. Use standalone for design-only work, or composed
  as the first stage of /belt:feature-dev. Task breakdown lives in the
  plan stage (/belt:plan), not here. --codex enables adversarial spec
  review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--codex]"
---

# design

Belt pipeline for the design stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Phase: intake

The invoke is declared in `pipeline.yml`. Pass the user's original
input (ticket id, URL, free text, or a requirements.md path) verbatim
as the skill argument. The skill writes goal-sheet.md and evidence.md.

## Phase: design

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md (resolve the path via `belt-agent status`,
   artifact `goal_sheet`).
2. Explore the code the change touches with Grep/Read. Dispatch
   `belt-agent:explorer` subagents (focus: flow or patterns) only if
   the area is unfamiliar AND spans 10+ files.
3. Write `docs/features/<topic>/design.md` with exactly these sections:
   - `## Architecture` — approach, components, data flow
   - `## Key Decisions` — each decision with a one-line rationale;
     rejected alternatives in one line each
   No task list and no test strategy here — both live in plan.md
   (/belt:plan).
4. Append the design entry to evidence.md (format:
   `plugins/belt-agent/references/authoring-principles.md`).

## Phase: design-review

The invoke is declared in `pipeline.yml`; pass the design.md path as
the review target. Complete the review's batched triage, then append
the design-review entry to evidence.md.

## Red flags

- Never ask the user design questions one at a time — batch each
  round's frontier in one AskUserQuestion call (authoring-principles
  §4).
- Never write a Tasks section in design.md — task breakdown belongs
  to /belt:plan.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
