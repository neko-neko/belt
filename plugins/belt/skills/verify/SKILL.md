---
name: verify
description: >-
  Runs the browser-based verification stage: scripted scenario replay
  (monkey-test) followed by exploratory testing (dogfood). Use standalone to
  verify a change against existing scenarios, or composed as the e2e leg of
  /belt:build. Requires agent-browser.
user-invocable: true
---

# verify

Belt pipeline for the browser-based verification stage. Pipeline structure,
phase order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the entry check, supplement loading contract, and red flags.

## Entry Check (standalone runs)

Before `belt-agent init`, locate the scenarios source:

- `docs/features/<topic>/scenarios.yml` (feature runs), or
- `docs/features/<topic>/rca-scenarios.yml` (bug runs).

On glob collision, prefer the most recently modified. If neither exists,
pause and ask the user — monkey-test has nothing to replay. When this stage
runs composed under `/belt:build` (phase ids `verify/monkey-test` /
`verify/dogfood`), the same resolution applies at the monkey-test phase.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides.

| Phase | Supplement | Purpose |
|---|---|---|
| monkey-test | `./references/monkey-test-supplement.md` | scenarios source resolution, context injection, reproduction-scenario rule (bug runs), output paths |
| dogfood | `./references/dogfood-supplement.md` | output override, diff-scoped exploration, Root Cause re-verification (bug runs), CLI-only degradation |

## Narrative Notes

Both phases produce a narrative note so context can be restored after
`/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip supplement loading**: output paths and scope overrides live there.
- **Never write files outside `docs/features/<topic>/`.**
- **Never auto-retry FAIL scenarios silently** — report them.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders, keep headings.

## References

- `./references/monkey-test-supplement.md` — monkey-test phase overrides
- `./references/dogfood-supplement.md` — dogfood phase overrides
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
