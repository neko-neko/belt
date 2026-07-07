---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix
  planning, TDD repair, autonomous code review, mandatory QA replay of
  the reproduction scenarios with human-readable evidence, and
  integration. Use when a bug needs structured diagnosis and verified
  repair. --codex enables adversarial review.
user-invocable: true
argument-hint: "[--codex]"
---

# bug-fix

Composed belt pipeline: diagnose → checkpoint → build → qa → integrate.
`pipeline.yml` declares four `invoke.pipeline` references plus the
`integrate` leaf; `belt-agent init` expands them inline, so `next`
returns namespaced leaf phases (`diagnose/rca`, `build/execute`,
`qa/qa`, ...) in a single run.

Human touchpoints are exactly three: the diagnosis approval
(fix-plan-review presents the RCA summary and the fix plan together),
the checkpoint pause, and integrate.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — entry checks, phase execution steps, and red flags live
there:

- `diagnose/*` → `plugins/belt/skills/diagnose/SKILL.md`
- `pre-execute-handover/*` → (none — follow the phase description:
  `/belt:handover`, `/clear`, `/belt:resume`)
- `build/*` → `plugins/belt/skills/build/SKILL.md`
- `qa/*` → `plugins/belt/skills/qa/SKILL.md` (replays
  rca-scenarios.yml)
- `integrate` → `plugins/belt/skills/feature-dev/SKILL.md`, Phase:
  integrate (the leaf definition is identical by contract)

Smaller runs: `/belt:diagnose` for diagnosis-only work, `/belt:build`
when a fix plan already exists, `/belt:qa` for QA alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md.**
- **Never skip diagnose**: root cause must precede fix. "Fix first" is
  the anti-pattern (enforced by the diagnose stage's own red flags).
- **Never bypass the pre-execute-handover checkpoint.**

## References

- `plugins/belt/skills/diagnose/SKILL.md` — diagnose stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/qa/SKILL.md` — QA stage contract
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
