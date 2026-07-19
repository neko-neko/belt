# Goal sheet: wayfinder-layer

## Goal

Give belt a way to take on efforts that are too foggy to spec directly —
larger than one agent session, with no visible route from here to the
destination — by porting mattpocock/skills `wayfinder` as an experimental
upstream planning layer. A new `/belt:wayfinder` skill charts a foggy
effort as a shared Linear decision map, resolves its decision tickets
across bounded sessions, and hands each graduated effort to the existing,
unmodified `/belt:requirements` → `/belt:feature-dev` chain. The
experiment succeeds when the pilot (YAML Universe) is charted and
converged to a requirements handoff with zero changes to existing stage
skills.

## In-scope

- A new experimental skill `/belt:wayfinder` under
  `plugins/belt/skills/wayfinder/` (SKILL.md + pipeline.yml + belt.toml),
  driven as a belt pipeline (map establishment → decision resolution →
  requirements handoff), one run graduating exactly one effort.
- A three-level Linear decision map (map issue → effort sub-issues →
  decision sub-issues) in workspace `neko-neko`, with label-based ticket
  classification (decision / research / deep-HITL), native blocking
  relations, and a frontier computed over open × unblocked × unclaimed
  decision tickets.
- Hybrid resolution: batched §4 frontier interview for independent
  decisions, solo sessions for deep-HITL tickets, AFK subagents for
  research tickets. Session rounds default to 3 via belt.toml
  `[wayfinder] rounds`.
- Idempotent per-ticket resolution (comment → close → map line) with a
  session-start reconciliation pass, an explicit non-graduated stall
  status when the frontier empties with open tickets remaining, and
  interruption safety at every round boundary.
- README + manifest updates marking the skill experimental, with a
  minimum linear-cli version and an MIT attribution to mattpocock/skills
  wayfinder.

## Out-of-scope

- Any behavior change to existing skills, pipelines, or agents
  (feature-dev, requirements, design, plan, build, qa, diagnose, goal,
  protocol, and the 6 agents stay byte-unmodified).
- belt-core / belt-agent Rust changes, including new gate types, loop
  constructs, or engine-level Linear integration.
- Full adoption of the mattpocock pipeline (its QA deprecation contradicts
  belt's deterministic-gate + QA-evidence core).
- Local-markdown or non-Linear trackers; automation of `/clear`;
  promotion of wayfinder to official status and any to-tickets / implement
  analogue.

## Acceptance criteria

1. `belt lint plugins/belt/skills/wayfinder/pipeline.yml` exits 0.
2. Running `/belt:wayfinder` for the pilot creates on Linear (workspace
   `neko-neko`, team BELT) one labeled map issue with the five body
   sections and ≥5 child decision tickets including ≥1 blocking relation,
   observable via `linear issue view` and `linear issue relation list`.
3. A ticket blocked by an open ticket is absent from the computed
   frontier and appears once its blocker closes — observable in the
   session's frontier listing.
4. One resolution session resolves ≥2 independent decision tickets,
   stopping at ≤3 rounds under the default config; each resolved ticket
   shows a resolution comment, closed state, and a Decisions-so-far line
   on the map.
5. ≥1 research ticket completes AFK with findings posted as a ticket
   comment and no user interaction during its execution.
6. `/belt:requirements`, given the graduated effort ticket ID, produces a
   `docs/requirements/<topic>/requirements.md` reflecting the consolidated
   decisions, while `git diff` for `plugins/belt/skills/requirements/`
   stays empty.
7. `git status` shows no modifications under `crates/` nor to any existing
   plugin skill/agent file; new files are confined to the wayfinder skill
   directory and the flow's own doc outputs.
8. Ending a session mid-resolve and starting a new session reconstructs
   the frontier from Linear and continues the same run to completion.

## Open risks

- Linear placement, workspace-targeting mechanism (`.linear.toml` vs
  `--workspace` flags), research execution vehicle, HITL deep-dive skill
  bindings, and the deterministic gate commands (map existence, effort
  readiness) are all deferred to the design stage.
- Promotion criteria from experimental to official is a post-pilot user
  judgment.
- No repo `.linear.toml` exists and this machine's default Linear
  workspace is not `neko-neko`, so every linear-cli call must target the
  workspace explicitly or the map lands in the wrong workspace.
