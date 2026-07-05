---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix
  planning, code review, and regression verification. Use when a bug needs
  structured diagnosis and verified repair. --e2e adds browser-based
  regression tests; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Composed belt pipeline: the diagnose stage, a context-reset checkpoint, and
the shared build stage. `pipeline.yml` declares three `invoke.pipeline`
references; `belt-agent init` expands them inline, so `next` returns
namespaced leaf phases (`diagnose/rca`, `build/execute`,
`build/e2e`, ...) in a single run — status, resume, and
narrative notes work exactly as in a flat pipeline.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — entry checks, phase execution steps, and red flags
live there:

| Phase id prefix | Stage skill |
|---|---|
| `diagnose/` | `plugins/belt/skills/diagnose/SKILL.md` |
| `pre-execute-handover/` | (none — follow the phase description: `/belt:handover`, `/clear`, `/belt:resume`) |
| `build/` | `plugins/belt/skills/build/SKILL.md` |

Smaller runs are available directly: `/belt:diagnose` for diagnosis-only
work, `/belt:build` when a fix plan already exists, `/belt:verify` for
browser verification alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md**: entry checks and phase execution steps are defined per stage, not here.
- **Never skip diagnose**: root cause must precede fix. "Fix first" is the anti-pattern (enforced by the diagnose stage's own red flags).
- **Never bypass the pre-execute-handover checkpoint**: the context reset before execute is the pipeline's core ergonomics.

## References

- `plugins/belt/skills/diagnose/SKILL.md` — diagnose stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/verify/SKILL.md` — browser verification contract (invoked by build's e2e phase when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
