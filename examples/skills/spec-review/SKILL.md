---
name: spec-review
description: >-
  4-perspective spec review pipeline. Dispatches requirements, design-judgment,
  feasibility, and consistency agents in parallel with N-way voting.
argument-hint: "[--codex] [--iterations N] [--ui] [--swarm]"
---

# Spec Review

4-perspective spec review with N-way voting and interactive dialogue resolution.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Add `config.ui_agent` if `args.ui`. Add Codex (`adversarial-review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → triage → present |
| `fix` phase | Dispatch `feature-implementer` with accepted findings. Modify spec bottom-up to prevent line-shift |

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (section-based):
- Match: same `section` + similar `description` (>80% semantic overlap)
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup

**Base selection**: iteration with most findings becomes the base set.
Subsequent iterations vote to confirm/deny each base finding.
Complementary findings (present in minority but absent from base) are added if unique.

## Triage

Categories: `requirements`, `design-judgment`, `feasibility`, `consistency`, `codex-adversarial`, `ui-design`

**Dialogue group** (interactive, max 3 rounds per finding):
- `requirements` findings with severity high or medium
- `design-judgment` findings with severity high or medium

Present each finding. Ask user for intent/context. Revise suggestion based on response.
After dialogue, user confirms revised suggestion or rejects.

**Selection group** (direct accept/reject):
- All other findings (feasibility, consistency, low-severity, codex, ui)

Present as numbered list sorted by severity descending. User selects which to fix.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user
- Ignore consensus vote results

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
