---
name: test-review
description: >-
  3-perspective test review pipeline. Dispatches coverage, quality, and
  design-alignment agents in parallel with N-way voting. Produces requirement map.
argument-hint: "[--codex] [--iterations N] [--swarm]"
---

# Test Review

3-perspective test review with N-way voting, requirement mapping, and direct
selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only test-review-specific concerns (design spec resolution, requirement map, voting, triage, fix strategy, verify).

## Design Spec Resolution

Before the orchestrator dispatches agents, resolve the design spec path. The `test-review-design-alignment` agent requires it:
1. Check output directory for `*-design.md`
2. If not found, check `docs/plans/*-design.md` with matching date prefix
3. Pass as `design_doc_path` context to the agent
4. If no design spec found → dispatch agent without it (reduced coverage)

## Requirement Map

The `design-alignment` agent returns a `requirement_map` in addition to findings:

| Column | Content |
|---|---|
| # | Sequential number |
| Requirement | Requirement from design spec |
| Source | Section in design spec |
| Test | Test file:line covering this requirement (or "—") |
| Gap | Missing coverage description (or "—") |

The requirement map is **informational only** — not subject to voting or selection.
Present it as a table in the review report. Gap entries inform coverage findings.

## Voting Protocol

Activated when this phase's `invoke.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (file/line-based):
- Match: same `file` + line within ±10 lines + similar `description`
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup
- `requirement_map`: not voted, most detailed version is adopted

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `test-coverage`, `test-quality`, `test-design-alignment`, `codex`

**No dialogue phase.** All findings presented as numbered list sorted by severity descending.
User selects which to fix by number.

### Fix Strategy by Category

| Category | Fix action |
|---|---|
| `test-coverage` | Add new test cases for uncovered paths |
| `test-quality` | Improve existing test structure, naming, assertions |
| `test-design-alignment` | Add requirement-based tests from requirement map gaps |

## Verify (after fix)

1. `git diff` — confirm changes are test files only
2. **Auto-detect and run project tests:**

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
- Ignore consensus vote results
- Classify test failures as acceptable without investigation

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Include requirement map in report even if no gaps found
- Run full test suite after applying fixes
