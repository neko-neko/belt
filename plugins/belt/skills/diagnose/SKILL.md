---
name: diagnose
description: >-
  Runs the bug diagnosis stage: root-cause analysis with a failing
  reproduction test, fix planning, and adversarial plan review. Use standalone
  for diagnosis-only work, or composed as the upstream stage of
  /belt:bug-fix. --e2e also authors reproduction scenarios; --codex enables
  adversarial plan review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# diagnose

Belt pipeline for the bug diagnosis stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

Artifacts follow the unified `docs/features/<YYYY-MM-DD-topic>/` layout with
branch `bugfix/<YYYY-MM-DD-topic>` — see
`plugins/belt/skills/design/references/path-convention.md` (SSOT).

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. `fix-plan-review` has no supplement; invoke its
declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| rca | `./references/rca-supplement.md` | RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, `rca-scenarios.yml` produce (when `--e2e`) |
| fix-plan | `./references/fix-plan-supplement.md` | RCA Fix Strategy -> task traceability, Given/When/Then test cases, verifiable completion conditions, task granularity |

## Phase-specific Runtime Notes

- **fix-plan-review**: `/belt:spec-review` is reused for fix-plan review. The
  grill-me prompt under the `design-judgment` observation does not fire by
  default (design decisions are already settled in rca / fix-plan). If it
  fires, treat it as a signal that upstream phases need to be revisited.

## Narrative Notes

`rca` and `fix-plan` produce a narrative note so context can be restored
after `/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip rca**: root cause must precede fix. "Fix first" is the anti-pattern.
- **Never proceed without a failing reproduction test**: RCA blocker.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never skip supplement loading when listed above.**
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders and keep headings.

## References

- `./references/rca-supplement.md` — rca phase override
- `./references/fix-plan-supplement.md` — fix-plan phase override
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
