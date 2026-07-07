---
name: spec-review
description: >-
  Spec review via the consolidated belt:spec-reviewer agent. Reviews any
  spec-family document (requirements.md, goal-sheet.md, design.md,
  plan.md). Findings are triaged in one batched selection. --codex adds
  an adversarial pass via /codex:rescue in the same parallel batch.
argument-hint: "[<target-path>] [--codex]"
---

# Spec Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Target

The spec document: use the caller-supplied path if given, otherwise the
most recently modified `design.md`, `plan.md`, `*-design.md`,
`goal-sheet.md`, or `requirements.md` under `docs/`.

## Output resolution

1. Caller supplied an output artifact name (e.g. `findings-plan`) →
   run `belt-agent status` and read that artifact's `resolved_path`.
2. No artifact name but a belt run is active (status succeeds and at
   least one phase is not COMPLETED or SKIPPED) → use the `findings-spec`
   artifact's `resolved_path`.
3. No belt run active → use the caller-supplied output directory; if
   none was supplied, use `<target document's directory>/review/`.
   The findings file is `findings-spec.json` in that directory.
4. With `--codex`: in a belt run, read the `findings-codex` artifact's
   `resolved_path` the same way; otherwise use `findings-codex.json` in
   the same output directory.

## Dispatch

1. Dispatch `Task(subagent_type: belt:spec-reviewer, prompt: <spec path
   + output_path>)`. With `--codex`, invoke `/codex:rescue` in the same
   message with the spec path, the findings JSON schema from the
   spec-reviewer agent, and its own output_path.
2. Announce what was dispatched.

## Triage (batched)

Read the findings JSON file(s). Present ALL findings as one numbered
list, sorted by severity (critical > high > medium > low). For each:
one line of description + the suggested fix. Ask the user once, via
AskUserQuestion or a single message, which numbers to apply. Do not ask
per-finding questions across multiple turns.

## Fix apply

Apply accepted suggestions to the spec with Edit. Then:

1. `git diff` — confirm only the target spec changed.
2. Re-check internal links and headings still resolve.

## Red flags

- Never modify the spec before user selection.
- Never filter findings before presenting them.
- Never ask one-question-at-a-time across turns — batch the triage.
