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

Composed belt pipeline: the design stage, a context-reset checkpoint, and the
shared build stage. `pipeline.yml` declares three `invoke.pipeline`
references; `belt-agent init` expands them inline, so `next` returns
namespaced leaf phases (`design/design`, `build/execute`,
`build/verify/monkey-test`, ...) in a single run — status, resume, and
narrative notes work exactly as in a flat pipeline.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — the supplement loading contracts, entry checks, and red flags
live there:

| Phase id prefix | Stage skill |
|---|---|
| `design/` | `plugins/belt/skills/design/SKILL.md` |
| `pre-execute-handover/` | (none — follow the phase description: `/belt:handover`, `/clear`, `/belt:resume`) |
| `build/verify/` | `plugins/belt/skills/verify/SKILL.md` |
| `build/` (other) | `plugins/belt/skills/build/SKILL.md` |

Smaller runs are available directly: `/belt:design` for design-only work,
`/belt:build` when a plan already exists, `/belt:verify` for browser
verification alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md**: supplement contracts are defined per stage, not here.
- **Never bypass the pre-execute-handover checkpoint**: the context reset before execute is the pipeline's core ergonomics.

## References

- `plugins/belt/skills/design/SKILL.md` — design stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/verify/SKILL.md` — verify stage contract (when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
