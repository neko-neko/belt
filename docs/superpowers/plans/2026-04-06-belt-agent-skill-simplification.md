# Belt-Agent SKILL.md Simplification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `skills/belt-agent/SKILL.md` from a protocol-heavy 176-line document into a ~60-80 line command-example-driven reference.

**Architecture:** Single-file rewrite. No code changes, no behavioral changes. Replace pseudo-code and JSON examples with concrete shell commands and a decision rules table.

**Tech Stack:** Markdown only.

---

### Task 1: Rewrite SKILL.md

**Files:**
- Modify: `skills/belt-agent/SKILL.md`

- [ ] **Step 1: Replace SKILL.md with new content**

Overwrite `skills/belt-agent/SKILL.md` with the following:

````markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving belt-agent CLI — a deterministic state machine for pipeline execution.

## Commands

```bash
# Start a new run
belt-agent init <pipeline.yml>
belt-agent init <pipeline.yml> --arg smoke=true --arg team=BELT

# Get current phase info (or completion signal)
belt-agent next
belt-agent next --run <run_id>

# Run gate checks for current phase
belt-agent verify
belt-agent verify --run <run_id>

# Advance to next phase
belt-agent step
belt-agent step --confirm          # required when phase has validate criteria
belt-agent step --run <run_id>

# Inspect full run state
belt-agent status
belt-agent status --run <run_id>
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → execute phase → verify (if gates) → step → next → ... → completed
```

1. `init` starts a run and returns the first active phase.
2. `next` returns the current phase info. If `completed: true`, the pipeline is done.
3. Execute the phase work (see `config.skill`).
4. If the phase has `gate`, run `verify`. On FAIL, fix and re-verify.
5. `step` to advance. If `advanced: false`, check the `reason` field.
6. Repeat from `next`.

## Decision Rules

| Situation | Action |
|-----------|--------|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. |
| `verify` FAIL with `regate` phases | Fix **those regate phases' work** (not the current phase). Re-run `verify`. |
| Phase has `validate` criteria | Verify each criterion yourself, then `step --confirm`. |
| `step` returns `advanced: false` | Read `reason`. Typically `confirmation_required` — verify criteria and retry with `--confirm`. |

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Only the following key has a defined meaning in this protocol:

| Key | Type | Meaning |
|-----|------|---------|
| `config.skill` | `string` | Skill to invoke for this phase. |

### Rules

- Unknown config keys MAY be ignored (forward compatibility).
- Pipeline-specific skills MAY add custom keys freely (belt does not interpret them).
- Dispatch implementation (which agents to launch, how to execute) is the
  pipeline-specific skill's responsibility.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase, you MUST NOT run
`belt-agent step --confirm` without verifying each criterion.

validate is a list of criteria that belt returns for LLM judgment.
belt cannot know whether these criteria were actually evaluated.
The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>
````

- [ ] **Step 2: Verify line count**

Run: `wc -l skills/belt-agent/SKILL.md`
Expected: 60-80 lines.

- [ ] **Step 3: Commit**

```bash
git add skills/belt-agent/SKILL.md
git commit -m "docs(belt-agent): simplify SKILL.md to command-example-driven reference"
```
