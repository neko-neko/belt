---
name: design
description: >-
  Runs the feature design stage: goal-sheet intake (/belt:goal) followed by
  a single design document (architecture + test strategy + implementation
  tasks) reviewed by belt:spec-reviewer. Use standalone for design-only
  work, or composed as the upstream stage of /belt:feature-dev. --e2e also
  authors agent-browser scenarios; --codex enables adversarial spec review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--e2e] [--codex]"
---

# design

Belt pipeline for the design stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Phase: intake

Invoke `/belt:goal`, passing the user's original input (ticket id, URL,
or free text) verbatim. The skill writes goal-sheet.md and evidence.md.

## Phase: design

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md (resolve the path via `belt-agent status`, artifact
   `goal_sheet`).
2. Explore the code the change touches with Grep/Read. Dispatch explorer
   subagents only if the area is unfamiliar AND spans 10+ files.
3. Write `docs/features/<topic>/design.md` with exactly these sections:
   - `## Architecture` — approach, components, data flow; rejected
     alternatives in one line each
   - `## Test Strategy` — for each acceptance criterion in goal-sheet.md,
     the test(s) that verify it (level: unit/integration/e2e + test name)
   - `## Implementation Tasks` — checkbox list; every task names its
     target files and its test
   If the e2e arg is true, also write `scenarios.yml` (Given/When/Then,
   one scenario per acceptance criterion).
4. Invoke `/belt:spec-review` with the design.md path as the target
   (pass `--codex` if the codex arg is true) and complete its batched
   triage.
5. Append the design entry to evidence.md (format:
   `plugins/belt-agent/references/authoring-principles.md`).

## Red flags

- Never ask the user design questions one at a time — batch remaining
  open points in one AskUserQuestion call.
- Never write an Implementation Task without file paths — execute
  dispatches subagents from this list alone.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
