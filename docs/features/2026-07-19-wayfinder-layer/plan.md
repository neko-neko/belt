# Plan: wayfinder-layer

The deliverable is authored files (no Rust): the wayfinder skill triad
(`pipeline.yml`, `SKILL.md`, `belt.toml`), README updates, and manifest
description updates. Acceptance is split between *deterministic static
checks* (lint, structure, command-correctness greps, git hygiene) and *live
Linear behavior* (map creation, frontier, resolution, handoff). The two live
in different files for a mechanical reason: belt QA has no per-scenario
authorization gate — `belt:qa-verifier` replays every `scenarios.yml` entry
autonomously and FAILs trigger the fix-loop. So `scenarios.yml` holds only
the deterministic checks qa-verifier can safely run, and the live behaviors
live in `pilot-runbook.md` as a user-driven acceptance run (running it *is*
the YAML Universe pilot).

**Merge / acceptance bar.** The authored skill may merge on the
`scenarios.yml` deterministic checks alone — status "authored,
pilot-unverified". The experiment is "verified" only after the
`pilot-runbook.md` scenarios pass in a user-authorized pilot; that pilot is
a **required follow-up** recorded in evidence.md at integrate, not optional.

## Test Strategy

Scenario file: `S` = `scenarios.yml` (autonomous QA), `R` =
`pilot-runbook.md` (user-driven live pilot).

| AC (goal-sheet) | Level | Test | Scenario (file) |
|---|---|---|---|
| AC-1 `belt lint` exits 0 | cli | run `belt lint` on the new pipeline.yml | `lint-passes` (S) |
| AC-2 map + ≥5 decision tickets + ≥1 blocking relation | qa (live) | run the pilot; assert map issue with 5 body sections, ≥5 `wf:decision` children, ≥1 blocking relation via `linear issue view` / `relation list` | `P1` (R) |
| AC-3 blocked ticket absent from frontier, appears when blocker closes | qa (live) | during the pilot, list the frontier before/after closing a blocker | `P2` (R) |
| AC-4 ≥2 decisions resolved, ≤3 rounds, comment+close+map-line each | qa (live) | run one resolution session; assert two tickets closed with resolution comments and Decisions-so-far lines | `P3` (R) |
| AC-5 ≥1 research ticket AFK with comment, no user interaction | qa (live) | assert a `wf:research` ticket (dispatched by the map phase) has a findings comment posted during execution | `P4` (R) |
| AC-6 handoff produces requirements.md; requirements skill unmodified | qa (live) + cli | pilot graduates one effort → `/belt:requirements` produces requirements.md; `git diff plugins/belt/skills/requirements/` empty | `P5` (R) + `requirements-skill-untouched` (S) |
| AC-7 no `crates/` or existing skill/agent changes | cli | `git diff --name-only main...HEAD` touches only allowed paths | `git-hygiene` (S) |
| AC-8 mid-resolve interruption recovers | qa (live) | end a session mid-resolve, resume, assert frontier reconstructed and run completes | `P6` (R) |
| Command correctness (design HIGH findings) | cli | grep SKILL.md for `blocked-by` form, `--workspace neko-neko`, `linear api`, `belt://current/handoff-ref.json`, `--run`; assert inverted `blocks <blocker>` absent | `command-correctness` (S) |
| Structure | cli | pipeline.yml has phases map/resolve/handoff and `max_retries: 0` on resolve | `pipeline-structure` (S) |
| Config | cli | SKILL.md documents `[wayfinder] rounds` | `skill-config-rounds` (S) |
| Deps + attribution | cli | README declares linear-cli >=2.0.0, the three deep-HITL skills, and the mattpocock attribution (each a separate AND grep) | `readme-declarations` (S) |
| Manifests | cli | both manifests list `/belt:wayfinder` | `manifests-list-wayfinder` (S) |

The `scenarios.yml` checks run unconditionally in QA. The `pilot-runbook.md`
scenarios (`P1`–`P6`) write to real Linear and run only in a user-authorized
pilot per the merge/acceptance bar above.

## Tasks

- [x] **T1 — pipeline.yml.** Create
  `plugins/belt/skills/wayfinder/pipeline.yml`: three phases. `map`
  (produces `map_ref` at `docs/features/*/map-ref.md` — the glob is the
  standard belt reusable-skill convention, since pipeline.yml cannot
  hardcode a per-run topic; the **authoritative** map-existence check is the
  `cmd` gate that reads map-ref and confirms the recorded map id exists via
  `linear api --workspace neko-neko`, so it is map-id-scoped and cannot pass
  on a stale/unrelated map-ref; the `file_exists docs/features/*/map-ref.md`
  gate is only a presence backstop; inline validate: 5 body sections / ≥1
  effort / tickets labelled). `resolve` (gate: `cmd` reading map-ref,
  `linear api` returning 0 iff some `wf:effort` has ≥1 `wf:decision` child
  all closed; `max_retries: 0`; inline validate: resolution invariants).
  `handoff` (no invoke; gate `file_exists belt://current/handoff-ref.json`
  — run-scoped, not a workspace glob; inline validate: effort consolidated
  + requirements.md produced). Test: `lint-passes`, `pipeline-structure`.
- [x] **T2 — SKILL.md.** Create `plugins/belt/skills/wayfinder/SKILL.md`:
  experimental description + one-line MIT attribution; `## Phase: map`,
  `## Phase: resolve`, `## Phase: handoff` execution steps; `## Config:
  [wayfinder] rounds` (default 3, 0 = frontier-empty); the frontier
  algorithm (map-scoped `linear api` descendant query + client-side
  unblocked/unclaimed filter, all calls `--workspace neko-neko`); resolution
  invariants (fixed comment→close→map-line order, session-start
  reconciliation, stall + deterministic cycle detection, interruption via
  handover/resume); ticket-vs-fog promotion checklist; deep-HITL binding by
  `## Method` line → `superpowers:brainstorming` / `domain-modeling` /
  `prototype`; research AFK subagent dispatch + comment-back; the
  `blocked-by` relation form; `--run <wayfinder-id>` pinning after the nested
  `/belt:requirements`; `## Red flags`. Follows authoring-principles (inline
  validate only, no criteria files, §4 referenced). Test:
  `command-correctness`, `config-and-readme`.
- [x] **T3 — belt.toml.** Create `plugins/belt/skills/wayfinder/belt.toml`
  with `pipeline = "./pipeline.yml"`. Test: `lint-passes` (pipeline
  resolves).
- [x] **T4 — README.** Edit `README.md`: add an experimental
  `/belt:wayfinder` entry to the plugins/skills section; extend the
  external-dependency table with linear-cli `>= 2.0.0` (required features),
  the three deep-HITL skills, and the mattpocock/skills wayfinder v1.1 MIT
  attribution. Test: `config-and-readme`.
- [x] **T5 — manifests.** Edit `plugins/belt/.claude-plugin/plugin.json` and
  `.claude-plugin/marketplace.json` to add `/belt:wayfinder (experimental)`
  to the belt plugin description. Test: `manifests-list-wayfinder`.
- [x] **T6 — pilot runbook.** `docs/features/2026-07-19-wayfinder-layer/pilot-runbook.md`
  already holds the live acceptance scenarios (P1–P6), teardown, label
  precondition, and merge/acceptance bar. No build action beyond keeping it
  in sync if T1–T5 change command forms; it is the required-follow-up
  acceptance doc, not autonomous QA. Test: (doc; no automated test).
