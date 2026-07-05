---
name: build
description: >-
  Runs the shared build stage: TDD implementation from the plan document's
  task list, two-agent code review, optional browser verification (--e2e),
  and integration. Use standalone with an existing design.md/fix-plan.md,
  or composed as the downstream stage of /belt:feature-dev and
  /belt:bug-fix.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# build

Belt pipeline for the build stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Entry check

A plan document must exist: `docs/features/<topic>/design.md` with an
Implementation Tasks section (feature runs) or
`docs/features/<topic>/fix-plan.md` (bug runs). If neither exists, stop
and ask the user. If `docs/features/<topic>/evidence.md` does not exist
(bug runs), create it now with the header `# Evidence: <topic>`.

## Phase: execute

This phase has no `invoke` — execute these steps directly:

1. Read the plan document's task list (Implementation Tasks in
   design.md, or the task list in fix-plan.md).
2. For each unchecked task, dispatch ONE `belt-agent:feature-implementer`
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

Invoke `/belt:code-review` (pass `--codex` if the codex arg is true) and
complete its batched triage. Append the code-review entry to
evidence.md.

## Phase: e2e (when the e2e arg is true)

Invoke `/belt:verify`. Append the e2e entry to evidence.md.

## Phase: integrate

Ask the user once: A) `wt merge` or B) `gh pr create`. Invoke
`/worktrunk` with the chosen mode. Confirm evidence.md has one entry per
completed phase, then append the integrate entry.

## Red flags

- Never start execute without the Entry check.
- Never forward the whole design doc to implementer subagents — copy the
  relevant constraints into each prompt.
- Never skip the per-task test run after a subagent returns.
- Never decide merge-vs-PR yourself — always the user's choice.
- Never let subagents write evidence.md — orchestrator only.

## References

- `plugins/belt/skills/design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
