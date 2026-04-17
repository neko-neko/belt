# Narrative Note Convention

Convention for phase-scoped narrative notes produced by narrative-producing phases in `feature-dev` and `bug-fix`. belt does not parse note content, so this convention is owned by the SKILL layer.

## Purpose

After the user resets session context with `/clear` or `/belt:resume`, reading the narrative note restores each phase's decisions, concerns, directives, and observations. Domain artifacts (`design.md`, `plan.md`, `rca-report.md`, etc.) record **what was produced**, while narrative notes record **why the call was made, what remains unresolved, and what the next phase must assume**.

## Path

```
.belt/runs/{run_id}/notes/phase-{phase_id}.md
```

- `{run_id}` is template-expanded by belt-core (the Engine creates `<run_dir>/notes/` during init).
- `{phase_id}` matches `phases[].id` in `pipeline.yml` (hyphens preserved: `monkey-test` → `phase-monkey-test.md`).
- Placed under a directory separate from domain artifacts (`docs/features/*`, `docs/plans/*`).

## File Schema

```markdown
---
phase: <phase_id>
run_id: <run_id>
---

## Decisions

<design decisions and directions settled in this phase>

## Concerns

<unresolved concerns, risks, and items downstream phases should watch>

## Directives

<instructions and preconditions for subsequent phases>

## Observations

<factual records, findings from exploration, test results, and so on>
```

## Rules

1. **Frontmatter is limited to two required fields**: `phase` and `run_id`. belt does not parse them, but downstream consumers (skill / LLM) can track origin. The LLM copies the `run_id` value from `belt-agent step` / `belt-agent status` output.
2. **All four sections are required**: `## Decisions` / `## Concerns` / `## Directives` / `## Observations`. Keep the heading even when empty (so downstream consumers are not confused by a missing section).
3. **Section order is fixed**: Decisions → Concerns → Directives → Observations.
4. **Keep each section concise**: Include the minimum information an LLM needs after `/clear` to reconstruct the decisions. Avoid copying content that is already in a domain artifact (a path reference is enough).
5. **Code blocks and links are free to use**: Anything within Markdown convention is acceptable, but avoid redundant re-explanation.

## Per-Section Guidance

### Decisions
- What was decided, and why alternatives were dropped.
- The information that answers "why this choice?" when a downstream phase asks.
- Example: "Dropped NoSQL candidates and adopted PostgreSQL. Reason: the existing schema-migration infra can be reused."

### Concerns
- Unresolved risks, assumptions, and unverified items.
- Time bombs downstream phases should watch for.
- Example: "E2E was not exercised in monkey-test. Manual confirmation is required during dogfood."

### Directives
- Constraints and preconditions subsequent phases must honor.
- Example: "In the plan phase, keep task granularity to 30 minutes or less (agreed during design)."

### Observations
- Facts discovered during exploration (especially those that do not fit in a domain artifact).
- Context useful for future investigation.
- Example: "The existing `FooService` does not actually implement the Bar interface (confirmed via a lint warning)."

## Example: feature-dev design phase

```markdown
---
phase: design
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- Reuse the existing belt-core narrative mechanism (2026-04-14 spec) for the context-reset capability; do not add new code to belt-core.
- Produce narrative notes only for six phases (lightweight phases excluded, per user agreement).

## Concerns

- `/clear` depends on manual user action. Without documenting "when to reset" in SKILL.md, the notes may never be consulted.

## Directives

- plan phase: keep implementation tasks at a granularity of 30 minutes or less.
- execute phase: do not quote the narrative's Decisions into commit messages (it becomes noise).

## Observations

- `narrative-convention.md` sits alongside existing references under `plugins/belt-agent/references/`.
- Criteria have been made per-plugin during the plugin migration (parity test detects drift).
```
