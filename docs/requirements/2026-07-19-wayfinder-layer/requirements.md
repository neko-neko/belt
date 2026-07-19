# Requirements: wayfinder-layer

## Background

belt's four-stage feature-dev assumes the task is already statable: goal /
requirements intake starts from a ticket, URL, or free text that can be
interviewed into a document. No layer exists for efforts that are "too much
fog to spec directly" — larger than one agent session, with no visible route
from here to the destination. The 2026-07-02 abandonment analysis showed what
happens when such efforts are forced into a single pipeline run: 11.7–27.0h
completions, 1,600-line plans, and eventual abandonment of the runtime.

mattpocock/skills `wayfinder` (v1.1, MIT) addresses exactly this gap: it
charts a foggy effort as a shared map of decision tickets on the issue
tracker and resolves them across sessions until the way is clear. Its
sibling `batch-grill-me` frontier algorithm is already ported into belt as
authoring-principles.md §4 (2026-07-18). This experiment ports the wayfinder
model as a new upstream layer, driven as a belt pipeline, feeding the
existing (unmodified) requirements → feature-dev chain.

## Goals

- G1: A foggy, multi-session effort can be charted as a Linear decision map
  and converged to a requirements handoff with zero changes to existing
  stage skills — demonstrated end-to-end on the pilot (YAML Universe).
- G2: Sessions stay bounded: a resolution session defaults to at most 3
  interview rounds, and interruption at any round boundary loses nothing.
- G3: After the pilot, promotion (experimental → official) can be judged
  from recorded evidence: the map, run state, and the graduated
  requirements document.

## Functional requirements

- FR-1: A new skill `/belt:wayfinder` exists at
  `plugins/belt/skills/wayfinder/` (SKILL.md + pipeline.yml + belt.toml),
  marked experimental in its SKILL.md description and in README.
- FR-2: The wayfinder flow runs as a belt run: `belt-agent init` on the
  wayfinder pipeline advances linearly from map establishment through
  decision resolution to a handoff that invokes the unmodified
  `/belt:requirements` skill.
- FR-3: One run graduates exactly one effort. The map persists on Linear
  across runs; a later run attaches to the existing map and must not create
  a duplicate map issue.
- FR-4: The map is a single Linear issue (workspace `neko-neko`) carrying a
  dedicated wayfinder-map label and exactly these body sections:
  Destination, Notes, Decisions so far, Not yet specified, Out of scope.
  The hierarchy is three levels: the map's sub-issues are effort tickets
  (graduation units — created at map creation, or during resolution as soon
  as a graduation unit becomes nameable), and each effort ticket's
  sub-issues are its decision tickets. A decision ticket not yet
  attributable to an effort may sit directly under the map until it is
  re-parented. Dependencies use native blocking relations. The map is an
  index: it gists and links, never restates ticket content.
- FR-5: Tickets are classified by a dedicated Linear label as decision,
  research, or deep-HITL. Every decision ticket resolves a decision, not a
  build slice. The ticket-vs-fog test is: the question can be stated
  precisely now. Items failing the test stay under Not yet specified until
  they graduate to tickets.
- FR-6: The frontier is computed as open × unblocked × unclaimed decision
  tickets among all descendants of the map (whether parented to the map or
  to an effort ticket) via linear-cli queries (client-side filtering
  permitted). Research and deep-HITL tickets are never part of the
  frontier. Unclaimed means no assignee; a session claims a ticket by
  assigning it, and releases the claim (unassigns) if the session ends
  without resolving it.
- FR-7: Resolution sessions use the hybrid model: independent frontier
  tickets are resolved in batches via the §4 frontier interview (multiple
  tickets per session); tickets flagged as deep HITL work (prototype /
  domain-modeling class) may occupy a session alone; research tickets are
  dispatched AFK to parallel subagents and never block a frontier round.
- FR-8: Session rounds default to 3, overridable via belt.toml
  `[wayfinder] rounds` following the §4 rounds convention (0 = until the
  frontier is empty).
- FR-9: Each resolved ticket receives a resolution comment, is closed, and
  gains a one-line link entry under the map's Decisions so far — applied in
  that fixed, idempotent order. If a linear-cli call fails mid-sequence,
  the session stops further Linear writes for that ticket and reports the
  partial state. On session start the skill reconciles: any closed decision
  ticket missing its Decisions-so-far line gets the line repaired before
  new frontier work begins.
- FR-10: Research ticket findings are recorded as comments on the ticket
  without user interaction during execution.
- FR-11: Establishing the map (destination naming, initial tickets, research
  dispatch) is an independent completion point; the same session may
  continue into the first resolution round.
- FR-12: When an effort is fully resolved — defined as all decision tickets
  parented under that effort being closed, a predicate distinct from "the
  frontier is empty" — its resolved decisions are consolidated into the
  effort ticket's description, and the handoff invokes `/belt:requirements`
  with that ticket's Linear ID. The requirements skill remains
  byte-unmodified.
- FR-13: `belt lint` passes on the new pipeline.
- FR-14: When the frontier is empty while open decision tickets remain
  (e.g. a blocking-relation cycle, or tickets parked behind an open
  deep-HITL or research ticket), the session reports the blocking set —
  naming any detected relation cycle — and ends with an explicit
  non-graduated status instead of looping or claiming convergence.

## Non-functional requirements

- NFR-1: No Rust changes — `crates/` is untouched; the flow uses only
  existing engine features (existing gate types, linear phases, run state).
- NFR-2: No behavior change to feature-dev, bug-fix, design, plan, build,
  qa, diagnose, goal, requirements, the protocol skill, or the 6 agents.
- NFR-3: The new SKILL.md follows authoring-principles.md (inline validate
  lists, no criteria/*.md or supplement chains, §4 referenced not restated).
- NFR-4: Interruption safety — a session can stop at any round boundary
  with zero loss; state reconstructs from the Linear map plus belt run
  state, and the run is `/belt:handover` + `/belt:resume` compatible.
- NFR-5: External dependencies (linear-cli >= 2.0.0 — required features:
  sub-issues via `issue create --parent`, the `issue relation` subcommand,
  the global `--workspace` flag, and structured `issue query --json` — plus
  any HITL deep-dive skills chosen at design time) are declared in README
  alongside the existing external-dependency entries, with a one-line MIT
  attribution to mattpocock/skills wayfinder v1.1 (batch-grill-me
  precedent).
- NFR-6: All linear-cli invocations target the workspace explicitly — the
  machine's default Linear workspace is not `neko-neko`, so implicit
  workspace resolution must not be relied on.

## Acceptance criteria

- AC-1: `belt lint plugins/belt/skills/wayfinder/pipeline.yml` exits 0.
- AC-2: Running `/belt:wayfinder` for the pilot (YAML Universe) creates on
  Linear (workspace `neko-neko`, team BELT) one labeled map issue with the
  five body sections and ≥5 child decision tickets including ≥1 blocking
  relation — observable via `linear issue view` and
  `linear issue relation list`.
- AC-3: A child ticket blocked by an open ticket is absent from the
  computed frontier; after its blocker closes it appears — observable in
  the session's frontier listing.
- AC-4: One resolution session resolves ≥2 independent decision tickets and
  stops at ≤3 rounds under the default config; each resolved ticket shows a
  resolution comment, closed state, and a Decisions-so-far line on the map.
- AC-5: ≥1 research ticket completes AFK with findings posted as a ticket
  comment, with no user interaction during its execution.
- AC-6: `/belt:requirements`, given the graduated effort ticket ID,
  produces `docs/requirements/<YYYY-MM-DD-topic>/requirements.md`
  reflecting the consolidated decisions, while `git diff` for
  `plugins/belt/skills/requirements/` stays empty.
- AC-7: After graduation, `belt-agent status` reports the wayfinder run
  completed; a subsequent run for the next effort reuses the same map issue
  (no second map issue exists afterwards).
- AC-8: `git status` shows no modifications to any pre-existing file except
  README and plugin/marketplace manifests if required; in particular
  nothing under `crates/` and no existing plugin skill/agent file changes.
  New files are limited to the wayfinder skill directory and the flow's own
  documentation outputs (`docs/requirements/<topic>/`, and
  `docs/features/<topic>/evidence.md` where authoring-principles.md
  mandates evidence entries).
- AC-9: Ending a session mid-resolve and starting a new session recovers:
  the new session reconstructs the frontier from Linear and continues the
  same run to completion.

## Out-of-scope

- Behavior changes to any existing skill, pipeline, or agent.
- belt-core / belt-agent Rust changes, including new gate types, loop
  constructs, or engine-level Linear integration.
- Full adoption of the mattpocock pipeline — its QA deprecation contradicts
  belt's deterministic-gate + QA-evidence core (rejected 2026-07-18).
- Local-markdown tracker fallback, or trackers other than Linear.
- Automation of `/clear` or session lifecycle (Claude Code runtime
  constraint).
- Promotion of wayfinder to official status, and to-tickets / implement
  analogues — downstream execution remains the existing feature-dev chain.

## Open decisions

- Linear placement details for the pilot map (assumed workspace
  `neko-neko` / team BELT; project and label naming finalized at map
  creation).
- Workspace targeting mechanism (`.linear.toml` committed to the repo vs
  `--workspace` flags in skill instructions) — design stage.
- Research ticket execution vehicle (generic subagent prompt vs an
  existing research skill) — design stage.
- HITL deep-dive skill bindings (grilling / domain-modeling / prototype
  availability and declaration) — design stage.
- Exact deterministic gate commands for map existence and effort readiness
  (cmd gates over linear-cli output) — design stage.
- Promotion criteria from experimental to official (user judgment after the
  pilot completes).
