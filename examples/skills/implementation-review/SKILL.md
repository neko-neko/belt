---
name: implementation-review
description: >-
  Multi-perspective implementation-plan review via a single consolidated
  reviewer subagent. 4 observations: clarity, feasibility, consistency,
  ui-spec. Direct selection triage. Optional Codex adversarial pass.
argument-hint: "[--codex]"
---

# Implementation Review

Multi-perspective plan review with direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only implementation-review-specific concerns (related design doc resolution, triage, verify).

## Related Design Doc Resolution

The reviewer agent resolves the related design doc internally (see `implementation-reviewer` agent). The orchestrator does not pre-resolve.

## Triage

Categories: `clarity`, `feasibility`, `consistency`, `ui-spec`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

## Verify (after fix)

1. `git diff` — confirm only target plan files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve
4. Design doc alignment — modified sections still reference correct design decisions

## Red Flags

**Never:**
- Modify plan without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
