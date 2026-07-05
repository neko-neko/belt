---
name: code-review
description: >-
  Two-agent parallel code review (belt:code-reviewer + belt:quality-reviewer)
  with deterministic merge. --codex adds an adversarial pass via
  /codex:rescue. Findings are triaged in one batched selection.
argument-hint: "[--codex]"
---

# Code Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Scope detection

- Current branch differs from main → `git diff main...HEAD`
- Else staged changes exist → `git diff --staged`
- Else → report "no diff detected" and exit.

Pass the file list + line counts to each agent.

## Dispatch

1. Run `belt-agent status` and read `resolved_path` for artifacts
   `findings-code`, `findings-quality`, `findings` (and `findings-codex`
   when `--codex`). If no belt run is active, use
   `docs/features/<topic>/review/` as the output directory.
2. Send ONE message with parallel Task calls:
   - `Task(subagent_type: belt:code-reviewer, prompt: <diff + design doc
     path if one exists + output_path>)`
   - `Task(subagent_type: belt:quality-reviewer, prompt: <diff + design
     doc path if one exists + output_path>)`
   With `--codex`, add `/codex:rescue` to the same batch with the diff,
   the findings JSON schema, and its own output_path.
3. Announce what was dispatched.

## Merge (deterministic — no judgment)

1. Read findings-code.json and findings-quality.json.
2. Two findings are duplicates ONLY when `file` AND `line` are equal.
   Keep the higher severity; on equal severity keep the
   findings-code.json one. Codex findings are never deduplicated.
3. Sort by severity (critical > high > medium > low), cap at 20 (note
   truncation as a final low finding), write to the `findings` artifact
   path as `{"findings":[...]}`.

## Triage (batched)

Present ALL merged findings as one numbered list (severity order, one
line + suggestion each). Ask once which numbers to fix. No per-finding
dialogue across turns.

## Fix apply + verify

1. Apply selected suggestions serially with Edit.
2. Run the project linter and test suite (Cargo.toml → `cargo clippy --
   -D warnings` + `cargo test`; package.json → `npm run lint` + `npm
   test`; pyproject.toml → `ruff check .` + `pytest`; go.mod → `go vet
   ./...` + `go test ./...`; Makefile → `make lint` + `make test`).
3. Report failures honestly — never suppress.

## Red flags

- Never modify code before user selection.
- Never filter findings before presenting them.
- Never let an agent read another agent's findings-*.json.
