# Worktrunk Supplement (Phase 8 override for `/worktrunk`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 8 (INVOKE 1 = Read this file; INVOKE 2 = prompt user for mode; INVOKE 3 = `/worktrunk`).

## A/B choice prompt

After all review / monkey-test / dogfood phases pass, prompt the user:

> Integration mode:
>   A. `wt merge` — Merge bugfix branch to main locally (worktree-first workflow)
>   B. `gh pr create` — Open a PR on GitHub for remote review
>
> Which? (A / B)

Default: no default; always require explicit user choice. This is a debug-flow Red Flag ("Never bypass the Phase 8 A/B choice").

## Branch naming convention

Debug-flow branches follow: `bugfix/YYYY-MM-DD-<topic>` (see `./path-convention.md`).

## Pre-merge checks

Before invoking `/worktrunk`:

1. `cargo test` (or project-appropriate test command) exit 0
2. `cargo clippy --workspace -- -D warnings` exit 0 (if Rust project)
3. `cargo fmt --check` exit 0 for modified packages
4. belt lint exit 0 for any modified pipeline.yml files
5. Reproduction test (from RCA-05) PASSes on this branch (INTEGRATE-03 blocker)

If any check fails, abort Phase 8 and report to user — do NOT merge.

## Post-merge verification

After `wt merge`:
- Re-run reproduction test on main branch to confirm PASS
- Confirm `git log` shows the fix commits merged

After `gh pr create`:
- Confirm PR URL is reachable
- No further action in debug-flow — user follows up externally

## Commit message convention for the fix

Fix commits (from `execute` phase) should follow:

```
fix(<scope>): <short description of bug fix>
```

Where `<scope>` is derived from the RCA Impact Scope (primary module). Example: `fix(auth): redirect expired session cookies to /login instead of 500`.
