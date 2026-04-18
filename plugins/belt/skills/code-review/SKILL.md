---
name: code-review
description: >-
  Multi-perspective code review with parallel observation subagents.
  Dispatches security, test, ai-antipattern, and cross-cutting reviewers
  in parallel; merges findings with severity-first + actionability-priority
  cross-agent dedup. --codex adds an adversarial pass via /codex:rescue.
argument-hint: "[--codex]"
---

# Code Review

Parent dispatcher for parallel multi-observation code review. This skill runs in the main context (no `context: fork`) because triage (user selection) and fix apply require user dialogue and direct Edit tool usage.

## Scope Detection

Determine the diff scope before dispatching observation agents:

1. If the current branch differs from `main` → `git diff main...HEAD`
2. Else if staged changes exist → `git diff --staged`
3. Else → report "no diff detected" and exit without dispatching.

Pass the diff summary (file list + line counts) as context to each observation agent.

## Impact Observation Context

If a design document exists in the current run's output directory (filename matches `*-design.md`), pass the Impact Analysis section content as additional context to the `cross-cutting-reviewer` agent's prompt. The Impact observation inside cross-cutting consumes it.

## Parallel Dispatch

Before dispatching agents, call `belt-agent status` and read each finding artifact's
`resolved_path` (artifacts named `findings-security`, `findings-test`,
`findings-ai-antipattern`, `findings-cross-cutting`, optionally `findings-codex`,
and `findings` for the merged output). Pass the resolved physical path to each
agent in its prompt as `output_path`. Agents write to that path without knowing
the underlying URI semantics.

Dispatch observation agents in parallel via the Agent (Task) tool. Send all Task calls in **one single message** with multiple tool-use blocks so they run concurrently:

- `Task(subagent_type: belt:security-reviewer, prompt: <diff + output_path: <resolved-findings-security>>)`
- `Task(subagent_type: belt:test-reviewer, prompt: <diff + output_path: <resolved-findings-test>>)`
- `Task(subagent_type: belt:ai-antipattern-reviewer, prompt: <diff + output_path: <resolved-findings-ai-antipattern>>)`
- `Task(subagent_type: belt:cross-cutting-reviewer, prompt: <diff + optional design-doc Impact Analysis + output_path: <resolved-findings-cross-cutting>>)`

If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the diff, the expected `findings-codex.json` format (same shape as observation agents, `source: "codex"`), and the resolved `output_path` (from `belt-agent status` `findings-codex` artifact).

Each finding artifact is independent — no race condition between agents.

Announce each dispatched agent (and Codex, if `--codex`) before sending.

## Merge + Cross-agent Dedup

After all agents complete:

1. For each finding artifact name (`findings-security`, `findings-test`,
   `findings-ai-antipattern`, `findings-cross-cutting`, optionally `findings-codex`),
   call `belt-agent status` to get the resolved_path, then read the JSON file at that path.
2. For each finding, determine if it is the same issue as a finding from another agent. Use file + line + description overlap as the primary signal (LLM judgment — when `file` + `line` match and descriptions share core vocabulary, treat as the same issue candidate).
3. For same-issue candidates, apply the dedup rule:
   - **Severity-first**: keep the finding with the highest severity (critical > high > medium > low).
   - **Tie-break on severity equality — observation priority (actionability order)**:
     `Security > Impact > Quality > Test > AI-antipattern > Performance > Simplification`
   - **Codex findings are NOT deduplicated**. If a Codex finding overlaps with another observation, keep both — Codex signal carries separate "external-provider adversarial" value.
4. Resolve `findings` artifact's path via `belt-agent status` (or
   `belt-agent locate belt://current/review/findings.json`) and write the merged JSON there:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "security|test|ai-antipattern|quality|performance|impact|simplification|codex",
      "severity": "critical|high|medium|low",
      "file": "<path>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ]
}
```

- Cap at 20 findings total. If more exist after dedup, keep the highest-severity ones and append a final `low`-severity finding of observation `quality` noting the truncation.
- If no findings at all, write `{"findings": []}`.

## Triage

Present all merged findings as a numbered list sorted by severity descending, then by observation priority. User selects which to fix by number. No dialogue phase (dialogue is reserved for spec-review).

## Fix apply

For each user-selected finding:
1. Read the file at the `file` / `line` location.
2. Apply the `suggestion` via the Edit tool in the main context.
3. Note in a scratch area which findings are applied (for verification).

Apply fixes serially to avoid merge conflicts in the same file.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope.
2. Auto-detect and run the project linter:

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. Auto-detect and run the project test suite:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

4. If linter or tests fail → report honestly, do not suppress.

## Red Flags

**Never:**
- Modify code without user approval of findings.
- Change files outside the diff scope.
- Omit or filter findings before presenting to user (except via the severity-first + observation-priority dedup rule above).
- Suppress or hide test/linter failures.
- Attempt to read other agents' `findings-*.json` files inside any observation agent's prompt (those agents must stay self-contained).

**Always:**
- Announce each dispatched agent (and Codex, if `--codex`).
- Dispatch all observation agents in a single parallel batch (one message, multiple Task tool uses).
- Apply the dedup rule deterministically — severity first, observation priority only on tie.
- Preserve Codex findings as separate entries (no dedup into other observations).
- Run linter and tests after applying fixes.
- Apply fixes serially to avoid merge conflicts.
