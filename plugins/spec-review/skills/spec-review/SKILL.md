---
name: spec-review
description: >-
  Multi-perspective spec review with parallel observation subagents.
  Dispatches feasibility, ui-design, and cross-cutting-spec reviewers in
  parallel; merges findings with severity-first + actionability-priority
  dedup. Findings in requirements/design-judgment with high/medium severity
  enter a grill-me dialogue; everything else uses selection triage.
  --codex adds an adversarial pass via /codex:rescue.
argument-hint: "[--codex]"
---

# Spec Review

Parent dispatcher for parallel multi-observation spec review with grill-me dialogue on design-critical findings. This skill runs in the main context (no `context: fork`) because grill-me dialogue requires user interaction.

## Scope

Locate the target spec document (most recent `*-design.md` under `docs/`, or user-supplied path).

## Parallel Dispatch

Dispatch observation agents in parallel via the Agent (Task) tool. Send all Task calls in **one single message**:

- `Task(subagent_type: spec-review:feasibility-reviewer, prompt: <spec path + path to write findings-feasibility.json>)`
- `Task(subagent_type: spec-review:ui-design-reviewer, prompt: <spec path + path to write findings-ui-design.json>)` — agent will early-exit with zero findings if spec has no UI content
- `Task(subagent_type: spec-review:cross-cutting-spec-reviewer, prompt: <spec path + path to write findings-cross-cutting-spec.json>)`

If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the spec, expected findings format, and output path `.belt/runs/<run_id>/review/findings-codex.json`.

Announce each dispatched agent before sending.

## Merge + Cross-agent Dedup

After all agents complete:

1. Read each `findings-<observation>.json` file under `.belt/runs/<run_id>/review/`.
2. Determine same-issue candidates using `section` overlap + description vocabulary (LLM judgment).
3. Apply dedup rule:
   - **Severity-first**: keep highest severity.
   - **Tie-break — observation priority (actionability order)**:
     `Feasibility > Requirements > Design-judgment > Consistency > UI-design`
   - **Codex findings are NOT deduplicated** (same as code-review).
4. Write `.belt/runs/<run_id>/review/findings.json` (cap 20 findings).

## Triage

After merge, partition findings:

- **Grill-me group**: `observation ∈ {requirements, design-judgment}` AND `severity ∈ {high, medium}`
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

Present selection-group findings as a numbered list sorted by severity descending. User picks by number which to fix.

## Fix apply

For each accepted / selected finding, apply the suggestion via Edit tool on the target spec document.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user (except via the dedup rule)
- Ask a user question that could be answered by inspecting the codebase
- Present multiple grill-group findings simultaneously
- Read other agents' `findings-*.json` from inside any observation agent

**Always:**
- Announce each dispatched agent (and Codex, if `--codex`)
- Dispatch observation agents in a single parallel batch
- Apply the dedup rule deterministically
- Preserve Codex findings (no dedup into other observations)
- Provide a recommended answer with every grill-me question
- Explore the codebase before asking user questions
- Honor the user's "enough / move on" signal without pushback
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
