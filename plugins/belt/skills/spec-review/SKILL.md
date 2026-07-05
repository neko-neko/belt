---
name: spec-review
description: >-
  Spec review via the consolidated belt:spec-reviewer agent. Findings are
  triaged in one batched selection. --codex adds an adversarial pass via
  /codex:rescue in the same parallel batch.
argument-hint: "[--codex]"
---

# Spec Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Target

The spec document: use the user-supplied path if given, otherwise the
most recently modified `*-design.md` or `goal-sheet.md` under `docs/`.

## Dispatch

1. Run `belt-agent status` and read `resolved_path` for artifacts
   `findings-spec` (and `findings-codex` when `--codex`). If no belt run
   is active (status fails), use `docs/features/<topic>/review/` as the
   output directory instead.
2. Dispatch `Task(subagent_type: belt:spec-reviewer, prompt: <spec path
   + output_path>)`. With `--codex`, invoke `/codex:rescue` in the same
   message with the spec path, the findings JSON schema from the
   spec-reviewer agent, and its own output_path.
3. Announce what was dispatched.

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
