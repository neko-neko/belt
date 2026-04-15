---
name: test-review
description: >-
  Multi-perspective test review via a single consolidated reviewer subagent.
  3 observations: coverage, quality, design-alignment. Produces findings and
  requirement map. Optional Codex adversarial pass.
argument-hint: "[--codex]"
---

# Test Review

Multi-perspective test review with requirement mapping and direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only test-review-specific concerns (requirement map handling, triage, fix strategy, verify).

## Design Spec Resolution

The reviewer agent resolves the design spec path internally (see `test-reviewer` agent). The orchestrator does not pre-resolve.

## Requirement Map

The reviewer agent emits a `requirement_map` array in `findings.json` alongside `findings`. Columns: number, requirement, source (design-spec section), test (file:line or `—`), gap (description or `—`).

The requirement map is **informational only** — not subject to selection. Present it as a table in the review report. Gap entries inform coverage findings in the numbered list.

## Triage

Categories: `coverage`, `quality`, `design-alignment`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

### Fix Strategy by Observation

| Observation | Fix action |
|---|---|
| `coverage` | Add new test cases for uncovered paths |
| `quality` | Improve existing test structure, naming, assertions |
| `design-alignment` | Add requirement-based tests from requirement-map gaps |

## Verify (after fix)

1. `git diff` — confirm changes are test files only
2. Auto-detect and run project tests:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

3. No linter step (test-only review)
4. If tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify production code (only test files)
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Classify test failures as acceptable without investigation

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Include the requirement map in the report even if no gaps found
- Run the full test suite after applying fixes
