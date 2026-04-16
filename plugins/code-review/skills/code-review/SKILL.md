---
name: code-review
description: >-
  Reviews a diff across seven dimensions: quality, security, performance, tests,
  AI anti-patterns, impact, and simplification. Use when code changes need
  multi-perspective critique before merging. --codex adds an adversarial pass.
argument-hint: "[--codex]"
---

# Code Review

Multi-perspective code review with direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only code-review-specific concerns (scope detection, impact context, triage, verify).

## Scope Detection

Determine diff scope before dispatching the reviewer agent:

1. If branch differs from `main` → `git diff main...HEAD`
2. Else if staged changes exist → `git diff --staged`
3. Else → report "no diff detected" and exit without dispatching

Pass the diff summary (file list + line counts) as context to the reviewer agent.

## Impact Observation Context

If a design doc exists in the current run's output directory (filename matches `*-design.md`), pass the Impact Analysis section content as additional context to the reviewer agent. The Impact observation consumes it.

## Triage

Categories: `quality`, `security`, `performance`, `test`, `ai-antipattern`, `impact`, `simplification`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope
2. Auto-detect and run project linter:

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. Auto-detect and run project tests:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

4. If linter or tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify code without user approval of findings
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Suppress or hide test/linter failures

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Run linter and tests after applying fixes
- Apply fixes serially to avoid merge conflicts in the same file
