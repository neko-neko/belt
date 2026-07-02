---
name: build
description: >-
  Runs the shared build stage: TDD implementation, multi-perspective code
  review, optional browser-based verification, and integration. Use standalone
  with a hand-written or pre-existing plan, or composed as the downstream
  stage of /belt:feature-dev and /belt:bug-fix. --e2e runs the verify
  sub-stage; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# build

Belt pipeline for the shared build stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the entry check, supplement loading contract, phase-specific runtime notes,
and red flags.

## Entry Check (standalone runs)

Before `belt-agent init`, confirm an implementation plan exists:
`docs/features/<topic>/plan.md` (feature runs), `docs/features/<topic>/fix-plan.md`
(bug runs), or a user-provided plan document. If none exists, pause and ask
the user — execute has nothing to implement. The build stage intentionally
declares no upstream `consumes`; locating the plan is this skill's job.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`execute` / `code-review`) have
no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create), pre-merge checks, PR-body template |

## Stage Delegation

When `args.e2e` is true, `next` returns the verify sub-stage's leaf phases as
`verify/monkey-test` and `verify/dogfood`. Before executing them, read
`plugins/belt/skills/verify/SKILL.md` — its entry check and supplements apply.

## Phase-specific Runtime Notes

- **execute**: orchestrator must reconstruct plan tasks into self-contained
  implementation specs before dispatching `belt-agent:feature-implementer`
  subagents. Do not forward broad research or plan excerpts verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

`execute` and `code-review` produce a narrative note so context can be
restored after `/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never start execute without the Entry Check**: implementing without a plan is the anti-pattern this stage exists to prevent.
- **Never filter or omit review findings**: triage of `/belt:code-review` output is the user's responsibility.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md`.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders, keep headings.

## References

- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `./references/evidence-catalog.md` — evidence items for execute / code-review criteria
- `plugins/belt/skills/verify/SKILL.md` — verify sub-stage contract (when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
