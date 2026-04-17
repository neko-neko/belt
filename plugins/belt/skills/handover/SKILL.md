---
name: handover
description: >-
  Writes a handover note (Resume hint) under the current belt run directory
  so a later session can pick up where the pipeline was paused. Use when
  pausing a multi-phase belt pipeline (feature-dev, bug-fix, debug-flow)
  before /clear or session end, or when the user invokes /belt:handover.
user-invocable: true
---

# /belt:handover

Save a Resume hint so a later session can resume an in-progress belt
pipeline run.

## Overview

When a belt pipeline run is in progress, `/belt:handover` writes
`.belt/runs/<run_id>/handover.md` in the current worktree. The note captures
why the pause happened and what the resumed session should do first. All
other pipeline state (run_id, pipeline, branch, current_phase, pipeline_file)
is already kept in `state.json` — this skill saves only the transient context
that would otherwise be lost.

## Workflow

```
Handover Progress:
- [ ] Step 1: Verify belt-agent is on PATH
- [ ] Step 2: Verify cwd is inside a git worktree
- [ ] Step 3: Query latest run via `belt-agent status`
- [ ] Step 4: Draft the Resume hint (Pause reason / First action / Transient context)
- [ ] Step 5: Write .belt/runs/<run_id>/handover.md
- [ ] Step 6: Tell the user "Handover written. Run /clear then /belt:resume."
```

### Step detail

1. If `command -v belt-agent` fails, abort with `"belt-agent CLI not found. Install or fix PATH."`
2. If `git rev-parse --git-dir` fails, abort with `"Not inside a git worktree; belt handover requires a git-based pipeline workspace."`
3. Run `belt-agent status` with no `--run` flag. Parse the JSON for `run_id`, `pipeline`, `branch`, `current_phase`. If no run is returned, abort with `"No belt run in progress; nothing to hand over."`
4. Draft the Resume hint in the LLM's head. Three bullets, each 1–3 lines:
   - **Pause reason**: why stopping here now (context heat, end of day, waiting on a decision)
   - **First action on resume**: the very next concrete step (for example: run `belt-agent status`, read `phase-plan.md`, start execute Task 1)
   - **Transient context**: anything said verbally or decided that is not yet written to `state.json` or `phase-*.md`
5. Write the file at `.belt/runs/<run_id>/handover.md` with the schema below. Overwrite if it exists.
6. Emit a single-line confirmation to the user: `Handover written. Run /clear then /belt:resume to continue.`

## Schema

```markdown
---
run_id: <uuid-v7>
branch: <branch captured in step 3>
created_at: <ISO 8601 UTC, e.g. 2026-04-17T18:23:44Z>
---

## Resume hint

- **Pause reason**: <one or two sentences>
- **First action on resume**: <concrete next step>
- **Transient context**: <context not in state.json or phase-*.md>
```

### Frontmatter rules

- Exactly three fields: `run_id`, `branch`, `created_at`. No additions.
- `run_id` matches the run directory name.
- `branch` is the branch at handover time (`git rev-parse --abbrev-ref HEAD` or the `branch` field from `belt-agent status`).
- `created_at` is UTC ISO 8601 with a `Z` suffix.

### Body rules

- Exactly one section: `## Resume hint` with exactly three bullet items.
- Do **not** describe phase progress. That is the role of `notes/phase-<id>.md`. Overlap between the two files is forbidden.

## Red Flags

- **Never write phase-level narrative in handover.md** — use `notes/phase-<id>.md` for phase records.
- **Never add fields to the frontmatter** — the three listed are the contract; extra fields are a future-compatibility hazard.
- **Never run /belt:handover outside a belt run** — if `belt-agent status` returns no run, abort with a clear message.

## References

- `plugins/belt-agent/skills/protocol/references/resume-mode.md` — driver-side resume handling
- `plugins/belt-agent/references/narrative-convention.md` — phase narrative note convention (separate file, separate role)
