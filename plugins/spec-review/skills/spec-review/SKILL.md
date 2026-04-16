---
name: spec-review
description: >-
  Multi-perspective spec review via a single consolidated reviewer subagent.
  5 observations: requirements, design-judgment, feasibility, consistency,
  ui-design. Review → grill-me dialogue → selection → fix.
argument-hint: "[--codex]"
---

# Spec Review

Multi-perspective spec review with grill-me dialogue on design-critical findings and direct selection on the rest.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only spec-review-specific concerns (triage, grill-me dialogue, selection, verify).

## Triage

After the reviewer agent returns findings, the orchestrator partitions them:

- **Grill-me group**: findings where `observation` ∈ {`requirements`, `design-judgment`} AND `severity` ∈ {`high`, `medium`}
- **Selection group**: everything else (feasibility, consistency, ui-design, low-severity, codex)

Process grill-me group first, then selection group.

## Grill-me Dialogue (Grill-me group)

Principles (borrowed from `/grill-me`):
- **One question at a time** — present a single finding; do not batch
- **Orchestrator provides a recommended answer** for every question
- **Codebase-answerable questions are not asked** — use Read/Grep to resolve them, update the suggestion, and move on
- **Rounds are unlimited** — iterate until the user explicitly accepts, rejects, or says "enough / move on"
- **Decision-tree order** — if finding A's decision affects finding B's proposal, resolve A first

Loop (pseudo):
```
order = topologically_sort(grill_group, by decision dependency)
for finding in order:
    while not resolved:
        if finding is answerable by codebase inspection:
            explore with Read/Grep
            revise finding.suggestion
            continue
        present finding + recommended_answer to user
        response = await user
        if response in {"accept", "OK", "approved"}:
            finding.resolution = "accept"; break
        if response in {"reject", "skip"}:
            finding.resolution = "reject"; break
        if response in {"enough", "move on"}:
            finding.resolution = "accept_current"; break  # accept revised state
        revise finding.suggestion based on response
```

After the loop, every grill-group finding has `resolution ∈ {accept, reject, accept_current}`.

## Selection Group

Present the selection-group findings as a numbered list sorted by severity descending. User picks by number which to fix.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user
- Ask a user question that could be answered by inspecting the codebase
- Present multiple grill-group findings simultaneously

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Provide a recommended answer with every grill-me question
- Explore the codebase before asking user questions
- Honor the user's "enough / move on" signal without pushback
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
