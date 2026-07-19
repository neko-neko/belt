---
name: wayfinder
description: >-
  Experimental (mattpocock/skills wayfinder port, MIT). Charts a foggy effort
  as a shared Linear decision map, resolves its decision tickets across bounded
  sessions, and hands one graduated effort to /belt:requirements. Upstream of
  /belt:feature-dev; one run graduates exactly one effort.
user-invocable: true
argument-hint: "<destination / effort description>"
---

# wayfinder

**Experimental.** Ported from `mattpocock/skills` wayfinder v1.1 (MIT).
Structure, gates, and done-criteria live in `pipeline.yml`; this file
defines only how to execute each phase. Do not restate the `validate:`
lines here.

Every `linear` command in every phase MUST pass `--workspace neko-neko`
explicitly — this machine's default workspace is not neko-neko and there
is no committed `.linear.toml`, so a missing flag writes to the wrong
workspace.

## Phase: map

Establish (or attach to) the shared Linear decision map, then write
`map-ref.md`.

### 1 — Confirm the labels exist

The map uses five labels: `wayfinder-map`, `wf:effort`, `wf:decision`,
`wf:research`, `wf:deep-hitl`. List existing labels in the neko-neko
workspace with
`linear api 'query{issueLabels{nodes{name}}}' --workspace neko-neko`.
(Do NOT use `linear label list --workspace neko-neko`: on `label list`,
`--workspace` is a boolean "workspace-level labels only" filter, not a
slug selector, so it cannot target neko-neko.)

- If `linear issue create --label <name>` auto-creates an unknown label
  in this workspace → skip creation.
- Otherwise → for each of the five that is missing, run
  `linear label create --name <name> --workspace neko-neko` before
  creating any issue (`--name` is required; the label name is not a
  positional argument).

### 2 — First run: create the map and its tickets

Do this only when no `wayfinder-map` issue exists yet for this
destination (see step 3 to detect a prior run first).

1. Create the map issue:
   `linear issue create --workspace neko-neko --team BELT --label wayfinder-map --title "Wayfinder map: <destination>" --description-file <path>`.
   The body has EXACTLY these five sections and nothing else:

       ## Destination        — one-line statement of where this effort is going
       ## Notes              — links to research/context tickets; no restated content
       ## Decisions so far   — one line per resolved decision (filled during resolve)
       ## Not yet specified  — the fog: open items not yet promoted to tickets
       ## Out of scope       — explicit exclusions

   The body is an **index**: it gists and links, never restates a
   ticket's content.

2. Create at least one effort sub-issue:
   `linear issue create --workspace neko-neko --team BELT --parent <map-id> --label wf:effort --title "<effort title>" --description-file <path>`.

3. Under an effort, create the decision sub-issues:
   `linear issue create --workspace neko-neko --team BELT --parent <effort-id> --label <wf:decision|wf:research|wf:deep-hitl> --title "..." --description-file <path>`.
   Each ticket carries exactly one of `wf:decision`, `wf:research`,
   `wf:deep-hitl`. A `wf:deep-hitl` ticket's body includes a
   `## Method` line (see the resolve phase for how it is read).

4. Wire every dependency as a native blocking relation, keeping the
   blocked ticket first:
   `linear issue relation add <blocked-id> blocked-by <blocker-id> --workspace neko-neko`.
   Never use an inverted form that places the blocker first.

5. Dispatch each `wf:research` ticket AFK now, per the **research
   dispatch rule** in the resolve phase.

### 3 — Later run: attach to the existing map

Before creating anything, find a prior map by its label:
`linear issue query --workspace neko-neko --label wayfinder-map --json`.

- If one matches this destination → attach to it; add missing effort or
  decision tickets under it. Never create a second map.
- If none matches → proceed with step 2.

### 4 — Write map-ref.md

Write the committed artifact
`docs/features/<topic>/map-ref.md` (topic slug rules follow
`plugins/belt/skills/design/references/path-convention.md`). It MUST
contain the literal line `- map_id: <ID>` — the pipeline gate parses
that exact form. `<ID>` is the map issue identifier
(`linear issue view <map-id> --workspace neko-neko`). Layout:

    # Map ref: <topic>

    - map_id: <BELT-NNN>
    - map_url: <https url>
    - destination: <topic-slug>

    ## Efforts (label: wf:effort)
    - <BELT-NNN> — <effort title>

    ## Decisions
    - <BELT-NNN> — <label> — <title>

## Phase: resolve

The cross-session decision-resolution loop. `pipeline.yml` sets
`max_retries: 0`, so each `belt-agent verify` runs exactly one loop
iteration and its gate is the loop condition.

### Frontier definition

The frontier = open × unblocked × unclaimed `wf:decision` tickets among
the map's descendants.

- Scope is the map issue id read from `map-ref.md`, resolved by
  descendant traversal — NOT a workspace-global label query — so a
  second charted map cannot leak its tickets in.
- `wf:research` and `wf:deep-hitl` tickets are NEVER in the frontier.

### Frontier computation (mechanical)

1. Read `map_id` from `map-ref.md`.
2. Query the map's decision descendants with their state and assignee
   via `linear api`:
   `linear api 'query($id:String!){issue(id:$id){children{nodes{labels{nodes{name}} children{nodes{id identifier state{type} assignee{id} labels{nodes{name}}}}}}}}' --variable id=<map-id> --workspace neko-neko`.
3. Keep only tickets labelled `wf:decision` whose `state.type` is
   neither `completed` nor `canceled` (open).
4. Drop any ticket that has an assignee (it is claimed by another
   session).
5. Drop any ticket with an open blocker: run
   `linear issue relation list <id> --workspace neko-neko` and drop the
   ticket if any `blocked-by` relation names a blocker that is still
   open.

The survivors are the frontier.

### Claim / release

- A session claims a ticket by assigning it to self:
  `linear issue update <id> --assignee self --workspace neko-neko`.
- At session end, a session releases every ticket it leaves unresolved
  by clearing that ticket's assignee (unassign).

### Hybrid resolution

Resolve the frontier by ticket kind:

- (a) **Independent frontier decisions** are resolved together via the
  frontier interview per
  `plugins/belt-agent/references/authoring-principles.md` §4: map the
  decisions as a design tree, and each round ask the questions whose
  prerequisites are settled in ONE AskUserQuestion call (up to 4,
  recommended option first), deferring any dependent question to a later
  round.
- (b) A **`wf:deep-hitl`** ticket may occupy a session alone. Its
  execution is bound by the ticket's `## Method` line:
  - `## Method`: open-ended scoping → run `superpowers:brainstorming`.
  - `## Method`: ubiquitous-language / boundary work → run
    `domain-modeling`.
  - `## Method`: throwaway state or UI probe → run `prototype`.
- (c) **`wf:research`** tickets are dispatched to AFK general-purpose
  subagents (see the research dispatch rule).

### Research dispatch rule (self-contained)

Dispatch a `general-purpose` subagent whose prompt contains, resolved in
full: the research question copied verbatim from the ticket, the exact
expected output (a findings summary — a short prose answer plus any
sources), and the completion condition (return the findings summary; ask
nothing of the user). There is no user interaction during execution.
When the subagent returns, the ORCHESTRATOR — not the subagent — posts
the findings back onto the research ticket:
`linear issue comment add <ticket-id> --body-file <path> --workspace neko-neko`.

### Idempotent resolution order (fixed)

For each resolved ticket, in this EXACT order:

1. Post a resolution comment:
   `linear issue comment add <ticket-id> --body-file <path> --workspace neko-neko`.
2. Close the ticket
   (`linear issue update <ticket-id> --state <done-state> --workspace neko-neko`).
3. Add its one-line entry `- [<ticket-id>] <≤100-char gist> (<link>)`
   under the map's *Decisions so far* section.

If a `linear` command fails mid-sequence for a ticket, stop further
writes for that ticket and report the partial state; do not continue to
the next step for it.

### Session-start reconciliation

Before starting new frontier work, diff the closed `wf:decision` tickets
(from the frontier computation query) against the *Decisions so far*
lines in the map body. For any closed decision missing its line, add the
line. This heals a prior session interrupted mid-sequence.

### Ticket-vs-fog promotion checklist

An item may move out of the map's *Not yet specified* section into a
real `wf:decision` ticket only if ALL THREE hold:

1. Its question is one sentence ending in `?`.
2. It names no unresolved upstream decision.
3. At least two candidate answers can be listed.

Fail any one → it stays in *Not yet specified* (fog).

### Stall + cycle detection

Graduation ("all of an effort's `wf:decision` children are closed") is
distinct from "frontier empty".

If the frontier empties while open decision tickets remain — a blocking
cycle, or tickets parked behind an open `wf:deep-hitl` or `wf:research`
ticket — end the session with an explicit non-graduated stall status
that names the blocking set. Do not loop.

Cycle detection is concrete: from every ticket's
`linear issue relation list <id> --workspace neko-neko` output, build
the set of blocks-edges (blocker → blocked), then run a depth-first
back-edge check over that set. If a back-edge exists, report the
specific edge list forming the cycle.

### Interruption safety

A session may stop at any round boundary via `/belt:handover`. A new
session resumes via `/belt:resume`, recomputes the frontier from Linear
(the frontier computation above is the single source of truth), and
continues the SAME belt run.

## Phase: handoff

Graduate exactly one effort and hand it to `/belt:requirements`.

### 1 — Identify the graduated effort

From the SAME `linear api` result the resolve gate used, the graduated
effort is the `wf:effort` ticket that has at least one `wf:decision`
child and whose `wf:decision` children are all closed
(`completed` or `canceled`).

### 2 — Consolidate the effort description

Consolidate that effort's resolved decisions into its Linear ticket
description:
`linear issue update <effort-id> --description-file <path> --workspace neko-neko`.

### 3 — Capture the run id, then invoke /belt:requirements

Capture the wayfinder run id BEFORE the nested invocation: record the
`run_id` field from `belt-agent status` (its stdout is JSON; there is no
`--json` flag) and hold it as `<wayfinder-run-id>`.

Then invoke `/belt:requirements` with the effort's Linear id.

The nested `/belt:requirements` runs its own `belt-agent init`, which
becomes the newest run and would shadow this one under latest-run
resolution. Therefore EVERY wayfinder `belt-agent` call AFTER the nested
invocation — `verify`, `step`, `status`, `locate` — MUST pin the run
explicitly, e.g. `belt-agent verify --run <wayfinder-run-id>`. Do not
rely on latest-run after the nested invocation.

### 4 — Write handoff-ref.json

After the nested run completes, write `belt://current/handoff-ref.json`.
Resolve its filesystem path with
`belt-agent locate belt://current/handoff-ref.json --run <wayfinder-run-id>`,
then write JSON recording the graduated effort id and the produced
requirements.md path:

    {
      "effort_id": "<BELT-NNN>",
      "requirements_path": "docs/requirements/<YYYY-MM-DD-topic>/requirements.md"
    }

## Config: [wayfinder] rounds

`belt.toml` key controlling the resolve-phase frontier interview round
cap:

    [wayfinder]
    rounds = 3    # default; 0 = until the frontier is empty

- Absent `belt.toml` or missing key → default `3`.
- `rounds = N` (N ≥ 1) → cap at N rounds; settle any unresolved frontier
  decisions with the recommended option.
- `rounds = 0` → no cap; continue rounds until the frontier is empty.

## Red flags

- Never omit `--workspace neko-neko` on any `linear` command — the
  default workspace is not neko-neko.
- Never leave a ticket claimed (assigned to self) at session end; release
  every unresolved claim by unassigning it.
- Never write the inverted blocking form — always
  `linear issue relation add <blocked-id> blocked-by <blocker-id> --workspace neko-neko`,
  blocked ticket first.
- Never forget to pin `--run <wayfinder-run-id>` on wayfinder
  `belt-agent` calls after the nested `/belt:requirements` invocation.
- Never restate a ticket's content in the map body — the map is an index
  that gists and links only.
- Never graduate a childless effort — a graduated effort must have at
  least one closed `wf:decision` child.
