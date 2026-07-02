---
name: design
description: >-
  Runs the feature design stage: brainstormed design document, test strategy,
  spec review, and implementation plan. Use standalone for design-only work,
  or composed as the upstream stage of /belt:feature-dev. --e2e also authors
  agent-browser scenarios; --codex enables adversarial spec review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# design

Belt pipeline for the feature design stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`test-scenarios` / `spec-review`)
have no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| design | `./references/brainstorming-supplement.md` | parallel exploration (code-explorer / code-architect / impact-analyzer), implicit-rules extraction, required design sections, worktree creation order |
| plan | `./references/writing-plans-supplement.md` | path override, Must-Verify, scenarios cross-referencing |

## Phase-specific Runtime Notes

- **spec-review**: grill-me dialogue for `requirements` / `design-judgment`
  findings; direct selection triage for the remaining observations.

## Narrative Notes

`design` and `plan` produce a narrative note so context can be restored after
`/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip supplement loading when listed above**: phase-specific overrides are lost and behavior drifts.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders and keep headings.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT, shared by all stages)
- `./references/brainstorming-supplement.md` — design phase overrides
- `./references/writing-plans-supplement.md` — plan phase overrides
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
