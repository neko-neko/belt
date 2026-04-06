# BELT-AGENT SKILL.md Simplification

## Problem

Current `skills/belt-agent/SKILL.md` (176 lines) is protocol-heavy with pseudo-code and JSON response examples. In practice, agents still run `belt-agent --help` after loading the skill, indicating it fails as a quick reference. The JSON response structure is better learned from actual output than from a static document that may drift.

## Goal

Rewrite SKILL.md as a concise command-example-driven reference (~60-80 lines) that eliminates the need for `--help` and focuses on decision points where agents make mistakes.

## Design

### Structure (4 sections)

#### 1. Basic Loop (command examples)

Show the full workflow as concrete shell commands, not pseudo-code:

```
belt-agent init <pipeline.yml>
belt-agent init <pipeline.yml> --arg smoke=true --arg team=BELT
belt-agent next
belt-agent next --run <run_id>
belt-agent verify
belt-agent step
belt-agent step --confirm
belt-agent status
```

A short narrative walks through the typical sequence:
`init` -> `next` -> (execute phase) -> `verify` (if gates) -> `step` -> `next` -> ... -> `completed: true`.

No JSON response examples. Agents read the actual output.

#### 2. Decision Rules (table)

Three rules that cover where agents commonly err:

| Situation | Rule |
|-----------|------|
| Phase has no `gate` | Skip `verify`, go directly to `step` |
| `verify` returns FAIL | Read `checks`, fix failing items, re-run `verify`. If `regate` phases listed, fix **those phases' work** (not current phase) |
| Phase has `validate` criteria | Verify each criterion yourself, then `step --confirm`. Never `--confirm` without verification (HARD-GATE) |

#### 3. Well-known Config Keys (keep as-is)

Preserves the boundary between belt (opaque passthrough) and belt skills (interpretation):

- `config` is an opaque map that belt passes through without interpretation.
- `config.skill`: skill to invoke for this phase.
- Unknown keys MAY be ignored (forward compatibility).
- Pipeline-specific skills MAY add custom keys freely.
- Dispatch implementation is the pipeline-specific skill's responsibility.

#### 4. HARD-GATE (keep as-is)

The `validate` + `--confirm` constraint block. Unchanged from current version.

### What gets removed

| Current section | Reason for removal |
|---|---|
| Protocol Loop (pseudo-code) | Replaced by command examples in section 1 |
| Command Response Handling (init/next/verify/step/status) | JSON structure learned from actual output; decision logic captured in section 2 |
| JSON response examples (all) | Static examples drift from implementation; agents parse real output |
| Regate (standalone section) | Folded into Decision Rules table row |

### Constraints

- No new features or behavioral changes to belt-agent CLI
- Frontmatter (`name`, `description`, `user-invocable: false`) preserved
- Target: 60-80 lines total

## Non-Goals

- `--version` support for belt-agent CLI (separate ticket)
- Changes to belt-agent command behavior or output format
- Changes to other skill files
