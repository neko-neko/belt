---
name: code-review
description: >-
  7-perspective code review pipeline. Dispatches 6 review agents + /simplify
  in parallel with N-way voting. File/line-based semantic similarity.
argument-hint: "[--codex] [--iterations N] [--swarm]"
---

# Code Review

7-perspective code review with N-way voting and direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only code-review-specific concerns (voting, triage, fix strategy, verify).

## Voting Protocol

Activated when this phase's `invoke.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (file/line-based):
- Match: same `file` + line within ±10 lines + similar `description`
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup
- Simplify findings: not voted, included directly

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `simplify`, `quality`, `security`, `performance`, `test`, `ai-antipattern`, `impact`, `codex`

**No dialogue phase.** All findings presented as numbered list sorted by severity descending.
User selects which to fix by number.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope
2. **Auto-detect and run project linter:**

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. **Auto-detect and run project tests:**

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
- Ignore consensus vote results
- Suppress or hide test/linter failures

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Run linter and tests after applying fixes
- Apply fixes serially to avoid merge conflicts in the same file
