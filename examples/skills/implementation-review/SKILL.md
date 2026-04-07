---
name: implementation-review
description: >-
  3-perspective implementation plan review pipeline. Dispatches clarity,
  feasibility, and consistency agents in parallel with N-way voting.
argument-hint: "[--codex] [--iterations N] [--ui] [--swarm]"
---

# Implementation Review

3-perspective implementation plan review with N-way voting and interactive
dialogue resolution.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Add `config.ui_agent` if `args.ui`. Add Codex (`adversarial-review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → triage → present |
| `fix` phase | Dispatch `feature-implementer` with accepted findings. Modify plan bottom-up to prevent line-shift |

### Related Design Doc Detection

Before dispatching agents, detect the related design spec:
1. Extract date prefix from plan filename (e.g., `2026-04-07` from `2026-04-07-foo-plan.md`)
2. Find matching `docs/plans/<prefix>*-design.md`
3. Pass as `design_doc_path` context to the `consistency` agent

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (section-based):
- Match: same `section` + similar `description` (>80% semantic overlap)
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `impl-clarity`, `impl-feasibility`, `impl-consistency`, `codex-adversarial`, `impl-ui-spec`

**Dialogue group** (interactive, max 3 rounds per finding):
- `impl-clarity` findings with severity high or medium
- `impl-feasibility` findings with severity high

**Selection group** (direct accept/reject):
- All other findings

Present as numbered list sorted by severity descending. User selects which to fix.

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
- Ignore consensus vote results

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Pass related design doc to consistency agent
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
