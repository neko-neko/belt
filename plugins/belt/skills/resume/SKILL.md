---
name: resume
description: >-
  Resumes a previously handed-over belt pipeline run by reading handover.md
  and state.json from the current worktree, verifying preconditions, and
  invoking the owning pipeline skill in resume mode via Skill tool args.
  Use when continuing belt pipeline work after /belt:handover and /clear,
  when the user invokes /belt:resume, or after a session restart to pick up
  an in-progress run in the current worktree.
user-invocable: true
---

# /belt:resume

Resume an in-progress belt pipeline run from a handover note.

## Overview

`/belt:resume` is the counterpart of `/belt:handover`. It reads the latest
run's `handover.md`, loads the Resume hint, and invokes the owning pipeline
skill (feature-dev, bug-fix, etc.) in resume mode. The pipeline's protocol
driver sees the `resume run_id=<id>` args and follows `resume-mode.md`,
which skips `belt-agent init` and jumps to `belt-agent status --run <id>`.

## Workflow

```
Resume Progress:
- [ ] Step 1: Run preconditions #1..#5 (short-circuit on first failure)
- [ ] Step 2: Resolve handover path via `belt-agent locate belt://current/handover.md`; read the file at the returned path; load Resume hint into context
- [ ] Step 3: Invoke Skill(skill="belt:<pipeline>", args="resume run_id=<id>")
- [ ] Step 4: Protocol driver follows resume-mode.md (init is skipped)
```

## Preconditions

| # | Check | Failure message | Recovery |
|---|---|---|---|
| 1 | `command -v belt-agent` succeeds | `"belt-agent CLI not installed or not on PATH"` | install / fix PATH / abort |
| 2 | `belt-agent status` returns a latest run (exit 0) | `"No belt runs found in current directory"` | `cd <worktree>` then rerun / abort |
| 3 | `belt-agent locate belt://current/handover.md` returns exists=true | `"No handover note for latest run. Run /belt:handover first."` | run `/belt:handover` first / abort |
| 4 | `current_phase != "COMPLETED"` | `"Last run already completed. Start a new run?"` | `belt-agent init` / abort |
| 5 | current branch (`git rev-parse --abbrev-ref HEAD`) equals handover.md `branch` | `"Branch changed A→B since handover. Continue anyway? (y/N)"` | `y` proceeds / `N` aborts / `git checkout <branch>` then rerun |

Preconditions run in order; the first failure stops the flow. #4 is
informational (COMPLETED runs are a valid state, not a fault). #5 is a
warning — the user decides whether to proceed.

## Skill args format

```
Skill(
  skill="belt:feature-dev",
  args="resume run_id=01947abc-0000-7000-8000-000000000000"
)
```

Literal prefix `resume ` followed by `run_id=<uuid>`. The protocol driver
detects this shape and follows `resume-mode.md`; extra tokens are not
permitted and will not be detected as resume mode.

The `<pipeline>` segment of `skill="belt:<pipeline>"` comes from
`state.json`'s `pipeline` field — read it through `belt-agent status` before
constructing the Skill call.

## Error Recovery

When a precondition fails, show the exact message and the available options
below it. Never auto-execute recovery actions; the user keeps control over
the working tree.

| Precondition | Options shown to user |
|---|---|
| #1 | (a) install belt-agent, (b) fix PATH, (c) abort |
| #2 | (a) `cd <correct worktree>` then rerun, (b) abort |
| #3 | (a) run `/belt:handover` first then rerun, (b) abort |
| #4 | (a) start a new run via `belt-agent init <pipeline.yml>`, (b) abort |
| #5 | (a) `y` to proceed on current branch, (b) `N` / abort, (c) `git checkout <branch>` then rerun |

## Red Flags

- **Never skip preconditions** — they are the contract; short-circuiting defeats fail-loud.
- **Never pass additional args beyond `resume run_id=<id>`** — the protocol driver's detection is narrow by design.
- **Never auto-cd or auto-checkout** — the user keeps control over working tree and branch.
- **Never assume the owning pipeline skill re-init automatically** — resume mode deliberately suppresses `belt-agent init` via the reference.

## References

- `plugins/belt-agent/skills/protocol/references/resume-mode.md` — driver-side behavior under `resume run_id=<id>` args
- `plugins/belt/skills/handover/SKILL.md` — the counterpart writer
