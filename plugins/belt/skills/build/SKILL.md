---
name: build
description: >-
  Runs the shared build stage: TDD implementation from the plan
  document's task list and two-agent code review with autonomous triage.
  Use standalone with an existing plan.md/fix-plan.md, or composed as
  the build stage of /belt:feature-dev and /belt:bug-fix. QA runs as its
  own stage (/belt:qa) after build; integration happens at the
  orchestrator's integrate phase.
user-invocable: true
argument-hint: "[--codex]"
---

# build

Belt pipeline for the build stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Entry check

A plan document must exist: `docs/features/<topic>/plan.md` with a
Tasks section (feature runs) or `docs/features/<topic>/fix-plan.md`
(bug runs). If neither exists, stop and ask the user. If
`docs/features/<topic>/evidence.md` does not exist, create it now with
the header `# Evidence: <topic>`.

## Phase: execute

This phase has no `invoke` — execute these steps directly:

1. Read the plan document's task list (Tasks in plan.md, or the task
   list in fix-plan.md).
2. For each unchecked task, dispatch ONE `belt-agent:implementer`
   subagent with a self-contained prompt containing: the task text, the
   exact file paths, the test(s) to write first, the relevant design
   constraints copied into the prompt (never "see the design doc"), and
   the project's test command.
3. After each subagent returns: run the project test suite yourself,
   check the task's checkbox in the plan document, and commit.
4. Tasks whose target files do not overlap MAY run in parallel;
   overlapping tasks run serially.
5. Append the execute entry to evidence.md (test + lint commands and
   observed results).

## Phase: code-review

The invoke is declared in `pipeline.yml`. In pipeline mode its triage
is autonomous: critical/high findings are fixed and committed, or
recorded as deferred with a reason. Append the code-review entry to
evidence.md including the deferred list.

## Red flags

- Never start execute without the Entry check.
- Never forward the whole plan document to implementer subagents — copy
  the relevant constraints into each prompt.
- Never skip the per-task test run after a subagent returns.
- Never let subagents write evidence.md — orchestrator only.

## References

- `plugins/belt/skills/design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
