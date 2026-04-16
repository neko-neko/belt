---
name: feature-dev
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Belt pipeline for quality-gated development. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags
that cannot be expressed in pipeline.yml.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`test-scenarios` / `spec-review` /
`execute` / `code-review`) have no supplement; invoke their declared skill
directly.

| Phase | Supplement | Purpose |
|---|---|---|
| design | `./references/brainstorming-supplement.md` | parallel exploration (code-explorer / code-architect / impact-analyzer), implicit-rules extraction, required design sections, worktree creation order |
| plan | `./references/writing-plans-supplement.md` | path override, Must-Verify, scenarios cross-referencing |
| monkey-test | `./references/monkey-test-supplement.md` | context injection for replay |
| dogfood | `./references/dogfood-supplement.md` | overrides and prior-phase artifact hints |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **spec-review**: grill-me dialogue for `requirements` / `design-judgment`
  findings; direct selection triage for the remaining observations.
- **execute**: orchestrator must reconstruct plan tasks into self-contained
  implementation specs before dispatching `belt-agents:feature-implementer`
  subagents. Do not forward broad research verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `design` / `plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` itself is the user's call — Claude Code runtime constraints prevent
automation. Use narrative notes as an option when context has grown large
after a heavy phase (for example, right after design, execute, or
code-review).

## Red Flags

- **Never skip supplement loading when listed above**: phase-specific overrides are lost and behavior drifts.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: the gate is `file_exists` only and empty sections still pass, but downstream consumers cannot restore context. Use at least `(none)` as a placeholder and always keep the heading.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — design phase overrides
- `./references/writing-plans-supplement.md` — plan phase overrides
- `./references/monkey-test-supplement.md` — monkey-test phase context injection
- `./references/dogfood-supplement.md` — dogfood phase overrides and hints
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with bug-fix)
