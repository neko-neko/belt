# Pilot runbook: wayfinder-layer (live Linear acceptance)

These scenarios verify the behavioral acceptance criteria (AC-2, AC-3, AC-4,
AC-5, AC-6-live, AC-8) that require **real writes to the `neko-neko` Linear
workspace**. They are deliberately kept out of `scenarios.yml`: belt QA has
no per-scenario authorization gate and `belt:qa-verifier` replays every
`scenarios.yml` entry autonomously, so a live scenario there would either
write to Linear ungated or trigger the fix-loop. This runbook is a
**user-driven acceptance run**, executed only with explicit authorization —
running it *is* the YAML Universe pilot the experiment is about.

## Acceptance bar and merge policy

- The authored skill may be merged on the `scenarios.yml` deterministic
  checks alone (lint, structure, command-correctness, README/manifest
  declarations, git hygiene). On that path the experiment status is
  **"authored, pilot-unverified"**.
- The experiment is **"verified"** only after this runbook's AC-2..AC-5,
  AC-6-live and AC-8 pass in a user-authorized pilot. This pilot is a
  **required follow-up**, recorded in evidence.md at integrate, not an
  optional extra.

## Preconditions

- Explicit user authorization to create issues in the `neko-neko` Linear
  workspace, team BELT.
- **Labels.** The map phase applies `wayfinder-map`, `wf:effort`,
  `wf:decision`, `wf:research`, `wf:deep-hitl`. Confirm before the run
  whether `linear issue create --label <name>` auto-creates an unknown
  label in this workspace; if it does not, create the five labels first
  (`linear label create --workspace neko-neko ...`). A missing label makes
  the first `issue create` fail.
- The wayfinder skill is installed (this feature merged, or run from the
  feature branch).

## Scenarios

- **P1 — pilot-map-created (AC-2).** Run `/belt:wayfinder` for the YAML
  Universe pilot through the map phase. Expect one `wayfinder-map` issue
  with the five body sections, ≥5 `wf:decision` child tickets, and ≥1
  blocking relation. Verify: `linear issue view <map-id> --workspace
  neko-neko` and `linear issue relation list <ticket-id>`.
- **P2 — pilot-frontier-blocking (AC-3).** Compute the frontier; confirm a
  ticket with an open blocker is absent. Close the blocker; recompute;
  confirm it now appears.
- **P3 — pilot-resolve-session (AC-4).** Run one resolution session under
  the default `[wayfinder] rounds = 3`. Expect ≥2 decision tickets closed
  within ≤3 rounds, each with a resolution comment and a Decisions-so-far
  line on the map.
- **P4 — pilot-research-afk (AC-5).** A `wf:research` ticket is dispatched
  AFK **by the map phase** and its findings are harvested during the
  resolve loop. Expect a findings comment on the ticket with no user
  interaction during its execution.
- **P5 — pilot-handoff (AC-6-live).** When an effort's decision children
  are all closed, the handoff phase consolidates decisions into the effort
  ticket and invokes `/belt:requirements` with its Linear id. Expect a
  `docs/requirements/<topic>/requirements.md` reflecting the consolidated
  decisions and `belt://current/handoff-ref.json` written.
- **P6 — pilot-interruption-recovery (AC-8).** End the session mid-resolve
  (`/belt:handover`), start a new session (`/belt:resume`). Expect the new
  session to recompute the frontier from Linear and continue the same run
  to completion.

## Teardown

The pilot leaves real Linear state. After acceptance, delete or archive the
created issues using the ids recorded in `map-ref.md`:

    for id in $(<ids from map-ref.md>); do
      linear issue delete "$id" --workspace neko-neko
    done

Delete the map issue last (deleting a parent may orphan children depending
on workspace settings — verify children are gone first). If the five
`wf:*` / `wayfinder-map` labels were created solely for the pilot, remove
them too. Record teardown completion (or a deliberate decision to keep the
map) in evidence.md.
