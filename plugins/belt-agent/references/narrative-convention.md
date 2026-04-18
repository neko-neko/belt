# Narrative Note Convention

Convention for phase-scoped narrative notes produced by narrative-producing phases across any belt pipeline (development, data-sync, audit, triage, or other domains). belt does not parse note content, so this convention is owned by the SKILL layer.

## Purpose

After the user resets session context with `/clear` or `/belt:resume`, reading the narrative note restores each phase's decisions, concerns, directives, and observations. Domain artifacts (`design.md`, `plan.md`, `rca-report.md`, etc.) record **what was produced**, while narrative notes record **why the call was made, what remains unresolved, and what the next phase must assume**.

## Path

Each phase's narrative note is declared in pipeline.yml as a `produces` artifact:

```yaml
produces:
  - name: design_notes
    path: "belt://current/notes/phase-design.md"
```

Resolve the physical path via `belt-agent status` (read `phases[].produces[].resolved_path`)
or `belt-agent locate belt://current/notes/phase-design.md`.

Convention: artifacts named `<id>_notes` use the `belt://current/notes/phase-<id>.md`
URI by convention. belt-core does not enforce this — it is owned by the SKILL layer.

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

## Example (generic phase)

```markdown
---
phase: <phase_id>
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- <Key choice made in this phase with rationale. Prefer statements
  that answer "why this choice, not the alternatives?"—future
  phases will ask.>

## Concerns

- <Unresolved risk or unverified assumption that downstream phases
  must watch. Prefer concrete leads over generic worries.>

## Directives

- <Constraint or precondition the next phase must honor. Place
  narrow, actionable rules, not broad philosophies.>

## Observations

- <Factual finding from exploration that does not fit a domain
  artifact but matters for future investigation or audit.>
```

The example above is intentionally domain-neutral; the same four sections
fit development (design, plan, execute, code-review), data-sync (scan,
analyze, approve, sync), audit (rca, fix-plan, verify), and other
workflows uniformly. See each skill's `SKILL.md` for which phases produce
narrative notes.
