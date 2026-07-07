---
name: plan
description: >-
  Runs the implementation planning stage: writes plan.md (test strategy +
  task list) and scenarios.yml (QA replay scenarios) from an approved
  design.md, then reviews the plan via belt:spec-reviewer. Use standalone
  when a design already exists, or composed between the design and build
  stages of /belt:feature-dev. --codex enables adversarial plan review.
user-invocable: true
argument-hint: "[--codex]"
---

# plan

Belt pipeline for the implementation planning stage. Structure, gates,
and done criteria live in `pipeline.yml`; this file defines how to
execute the single phase.

## Entry check

`docs/features/<topic>/goal-sheet.md` and `docs/features/<topic>/design.md`
must exist. In a composed run resolve them via `belt-agent status`
artifacts; standalone, take the most recently modified glob match. If
either is missing, stop and ask the user.

## Phase: plan

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md and design.md.
2. Write `docs/features/<topic>/plan.md` with exactly these sections:
   - `## Test Strategy` — for each acceptance criterion in
     goal-sheet.md, the test(s) that verify it (level:
     unit/integration/qa + test name)
   - `## Tasks` — checkbox list; every task names its target files and
     its test
3. Write `docs/features/<topic>/scenarios.yml` — at least one scenario
   per acceptance criterion, schema below.
4. Invoke `/belt:spec-review` with the plan.md path as the target and
   `findings-plan` as the output artifact (pass `--codex` if the codex
   arg is true), and complete its triage.
5. If a finding contests an approved design decision, do not edit
   design.md — present the finding to the user as an objection to the
   approved design; on acceptance, re-run the design stage standalone.
6. Append the plan entry to evidence.md.

## scenarios.yml schema

    setup:                        # required if any scenario is kind: browser
      start: "pnpm dev"           # launch command; omit when nothing to launch
      url: "http://localhost:3000"
      teardown: auto              # auto = QA kills processes it started
    scenarios:
      - id: login-ok              # kebab-case, unique in this file
        kind: browser             # browser | cli
        given: "a registered user on the login page"
        when: "they submit valid credentials"
        then: "the dashboard is shown"

For `kind: cli`, `when` is the exact command to run and `then` states
the expected stdout, exit code, or produced files.

## Red flags

- Never write a task without file paths — build dispatches subagents
  from this list alone.
- Never leave an acceptance criterion without both a Test Strategy row
  and a scenario.
- Never author a `kind: browser` scenario without a `setup:` block.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `../design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
