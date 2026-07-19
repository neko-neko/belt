# Design: wayfinder-layer

## Architecture

### Shape

A new experimental skill `/belt:wayfinder` lives at
`plugins/belt/skills/wayfinder/` with the standard belt-pipeline triad:
`SKILL.md` (how to execute each phase), `pipeline.yml` (phases, gates,
validate), and `belt.toml` (`pipeline = "./pipeline.yml"`). It is a
plugins/belt user-invocable skill, not a new plugin, marked experimental
in its SKILL.md description and in README. No Rust changes; no changes to
any existing skill, pipeline, or agent.

The skill sits upstream of the existing chain: its terminal phase hands a
graduated effort to the unmodified `/belt:requirements`, whose output
feeds `/belt:feature-dev`. wayfinder is the fog-clearing layer that did
not exist before intake.

### Pipeline topology (three phases)

The belt engine is a linear state machine with no loop construct. The
wayfinder model — "resolve decision tickets across sessions until an
effort is ready" — is expressed as three linear phases where the *middle
phase's gate is the loop condition*:

    map      → resolve → handoff

- **map** — Establish the Linear decision map for the destination, or
  attach to the existing one. On a first run the phase creates the map
  issue, its initial effort and decision sub-issues, wires blocking
  relations, and dispatches research tickets AFK. On a later run it finds
  the existing map and attaches (no duplicate). The phase writes a
  committed **map-ref** artifact (`docs/features/<topic>/map-ref.md`)
  recording the map issue ID, the destination slug, and the created effort
  and decision ticket IDs. This artifact is the *scoping key*: every
  downstream linear-cli/GraphQL query is scoped to the map issue ID (via
  descendant traversal), not to a workspace-global label, so the gates stay
  correct once a second map is charted. Deterministic gate: a `cmd` reads
  map-ref and confirms via `linear api` that the recorded map issue exists
  and carries the map label; plus `file_exists` on map-ref itself.

- **resolve** — The cross-session decision-resolution loop. The LLM
  computes the frontier, resolves tickets (hybrid model below), and
  re-checks. The phase does not advance until its gate — a `cmd` that reads
  map-ref and runs a `linear api` GraphQL query returning success **iff
  there exists a `wf:effort` descendant of the map with ≥1 `wf:decision`
  child and all its `wf:decision` children in a completed/canceled state**
  — passes. The `≥1 child` clause closes the vacuous-truth hole (an effort
  with zero decisions never graduates); the map-ref scoping resolves the
  dynamic-map-ID and no-`--parent`-filter limitations of `issue query`.
  The gate answers only "*some* effort graduated"; the skill layer, reading
  the same GraphQL result, identifies *which* effort and carries its ID to
  handoff. `max_retries: 0` (unlimited) makes each `verify` a loop
  iteration rather than a bounded retry, following the belt
  `max_retries = 0` convention (engine skips the retry guard at 0, and
  `step` blocks with `verify_required` until a passing `verify` is
  recorded). Long life is punctuated across sessions by the existing
  `/belt:handover` + `/belt:resume` machinery (no dedicated checkpoint
  phase — those commands work on any belt run).

- **handoff** — Consolidate the graduated effort's resolved decisions into
  its Linear ticket description, then invoke `/belt:requirements` with that
  effort's Linear ID. The invocation is skill-driven (the effort ID is
  determined at resolve time and cannot be templated into a static belt
  invoke), so this phase carries no `invoke:` in pipeline.yml; its
  execution is defined by SKILL.md. The nested `/belt:requirements` runs
  its own `belt-agent init`, which makes *its* run the newest — so
  `latest_run_id()` would shadow the wayfinder run. Therefore every
  wayfinder `belt-agent` call after the nested invocation (verify / step /
  status) MUST pin `--run <wayfinder-run-id>`; SKILL.md states this and
  captures the wayfinder run ID before invoking requirements. Gate:
  `file_exists` on a **run-scoped** `belt://current/handoff-ref.json` that
  the handoff phase writes after the nested run completes (recording the
  effort ID and the produced requirements.md path) — a workspace-global
  glob like `docs/requirements/*/requirements.md` is rejected because
  `execute_file_exists` has no mtime filter and the repo already contains a
  requirements.md, so it would pass spuriously. One wayfinder run graduates
  exactly one effort; remaining fog stays on the map for the next run.

### Linear decision map (three-level hierarchy)

Workspace `neko-neko`, team BELT. All linear-cli calls pass
`--workspace neko-neko` explicitly.

    map issue                      (label: wayfinder-map)
      └─ effort ticket             (label: wf:effort)      — graduation unit
           └─ decision ticket      (label: wf:decision | wf:research | wf:deep-hitl)

The map issue body carries exactly five sections — Destination, Notes,
Decisions so far, Not yet specified, Out of scope — and is an index: it
gists and links, never restating ticket content. Effort tickets are the
map's sub-issues; decision tickets are an effort's sub-issues. A decision
not yet attributable to an effort may sit directly under the map until
re-parented. Dependencies use Linear native blocking relations, recorded
as `linear issue relation add <blocked> blocked-by <blocker>` (equivalently
`add <blocker> blocks <blocked>`) — note linear-cli's `add <issueId> blocks
<relatedIssueId>` makes the *first* argument the blocker, so the
`blocked-by` form is used to keep the blocked ticket first without inverting
the graph.

**Ticket classification** is by Linear label: `wf:decision` (resolves a
decision — the only class in the frontier), `wf:research` (AFK
fact-finding), `wf:deep-hitl` (a decision needing a solo prototype /
domain-modeling session). The ticket-vs-fog test — "the question can be
stated precisely now" — gates promotion from the map's *Not yet
specified* section to a real ticket.

**Frontier** = open × unblocked × unclaimed decision tickets among all
descendants of the map. Scope is the map issue ID from map-ref (descendant
traversal), not a workspace-global label, so a second charted map does not
leak tickets into this frontier. Computed by querying open `wf:decision`
descendants of the map, then client-side filtering: drop any with an open
blocker (from `linear issue relation list`) and any with an assignee.
`issue query` filters by label/state/assignee but not by parent, so the
descendant scoping uses `linear api` GraphQL from the map ID (or, for the
pilot's single map, the `wf:decision` label is already map-unique — the
single-map assumption is stated explicitly and only the GraphQL path
survives a second map). *Unclaimed* means no assignee; a session *claims* a
ticket by assigning it to self and *releases* (unassigns) any ticket it
leaves unresolved at session end. Research and deep-HITL tickets are never
in the frontier.

### Hybrid resolution

- **Batched decisions** — independent frontier decision tickets are
  resolved together in one session via the §4 frontier interview: each
  round asks the settled-prerequisite questions in one AskUserQuestion
  call; dependent questions defer to a later round.
- **Deep-HITL** — a `wf:deep-hitl` ticket may occupy a session alone. Its
  execution binds to one of three concrete, already-available skills, keyed
  by the ticket's `## Method` line (set at ticket creation): open-ended
  scoping → `superpowers:brainstorming`; ubiquitous-language / boundary
  work → `domain-modeling`; a throwaway state/UI probe → `prototype`. These
  three are declared in README's external-dependency table (NFR-5).
- **Research** — `wf:research` tickets are dispatched to AFK
  general-purpose subagents with self-contained prompts (§3); the
  orchestrator posts each subagent's findings back as a comment on the
  ticket, with no user interaction during execution.

Session rounds default to 3, read from belt.toml `[wayfinder] rounds`
following the §4 rounds convention (`0` = until the frontier is empty).
The default is documented in SKILL.md `## Config`.

### Resolution invariants (idempotency, stall, interruption)

- **Idempotent resolution** — resolving a ticket applies three effects in
  a fixed order: post a resolution comment, close the ticket, add its
  one-line entry under the map's *Decisions so far*. On a mid-sequence
  linear-cli failure the session stops further writes for that ticket and
  reports the partial state.
- **Session-start reconciliation** — before new frontier work, the skill
  diffs closed `wf:decision` tickets against *Decisions so far* lines and
  repairs any missing line, healing partial writes from a prior
  interrupted session.
- **Stall detection** — graduation ("all of an effort's decision children
  closed") is a predicate distinct from "frontier empty". When the
  frontier empties while open decision tickets remain (a blocking cycle,
  or tickets parked behind an open deep-HITL/research ticket), the resolve
  gate stays FAIL; the skill reports the blocking set — naming any detected
  relation cycle — and ends the session with an explicit non-graduated
  status rather than looping.
- **Interruption safety** — a session may stop at any round boundary with
  zero loss. State reconstructs from the Linear map plus belt run state; a
  new session recomputes the frontier from Linear and continues the same
  run. This is the standard belt `/belt:handover` + `/belt:resume` flow.

### Determinism of the judgment-heavy rules (authoring §2)

authoring §2 forbids discretionary instructions, so the resolve rules that
read as judgment calls are reduced to mechanical checks in SKILL.md:

- **Ticket-vs-fog** promotion is a checklist, not a vibe: an item leaves
  *Not yet specified* only when (1) its question is one sentence ending in
  "?", (2) it names no unresolved upstream decision, and (3) at least two
  candidate answers can be listed. Fail any → it stays fog.
- **Map-index bound** — a *Decisions so far* entry is exactly one line:
  `- [<ticket-id>] <≤100-char gist> (<link>)`; anything longer means the
  content belongs in the ticket, not the map.
- **Cycle detection** is a concrete procedure over `relation list` output:
  build the blocks-edges set, run a depth-first back-edge check; a
  reported cycle is the specific edge list, not a judgment.

Irreducible judgment that remains — the *content* of a decision's
resolution and whether a candidate answer is sound — is inherent to a
planning skill and lives with the human (HITL) or the §4 interview, not in
a gate; the design accepts this as outside §2's determinism scope, which
governs control-flow rules, not the human's answers.

### External dependencies and provenance

- linear-cli `>= 2.0.0`, required features: sub-issues via
  `issue create --parent`, the `issue relation` subcommand, the global
  `--workspace` flag, structured `issue query --json`, and raw GraphQL via
  `linear api` (used for map-scoped descendant/graduation queries that
  `issue query`'s label-only filtering cannot express). Declared in README
  beside the existing external-dependency table.
- Deep-HITL resolution skills: `superpowers:brainstorming`,
  `domain-modeling`, `prototype` (all already available in the
  environment) — declared in README per NFR-5.
- MIT attribution to mattpocock/skills wayfinder v1.1 (one line, the
  batch-grill-me precedent), in README and the SKILL.md header.

## Key Decisions

- **Express the resolution loop as one long-lived phase with a `cmd`
  effort-ready gate and `max_retries: 0`.** The gate is the loop
  condition; each `verify` is an iteration. The gate is a `linear api`
  GraphQL query (not `issue query`, which has no `--parent` filter),
  scoped to the map ID from map-ref, true iff some `wf:effort` has ≥1
  `wf:decision` child all closed — the `≥1` clause prevents a childless
  effort from graduating vacuously. Rejected: a fixed set of resolution
  phases (unknown ticket count); a new engine loop construct (violates the
  no-Rust-change constraint and belt's Simplicity-First DSL); a plain
  `issue query` gate (cannot enumerate a parent's children).
- **A committed `map-ref.md` artifact is the scoping key for every gate and
  query.** It records the map issue ID, destination slug, and ticket IDs,
  letting static gate `cmd` strings and the frontier query scope to the
  specific map by descendant traversal rather than workspace-global labels.
  Rejected: label-only scoping (breaks the map gate's "exactly one" and
  leaks other maps' tickets into the frontier once a second map exists).
- **Cross-session continuity reuses `/belt:handover` + `/belt:resume`, no
  dedicated checkpoint phase.** Those commands already operate on any belt
  run and give round-boundary interruption safety for free. Rejected:
  embedding `handover/checkpoint.yml` (that models feature-dev's one-time
  pre-execute reset, not a repeated pause/resume).
- **handoff invokes `/belt:requirements` skill-driven (no static belt
  invoke), pins `--run`, and gates on a run-scoped marker.** The graduated
  effort ID is dynamic (known only at resolve time) and belt invoke args
  are static or run-arg templates, so a `pipeline.yml` invoke cannot carry
  it. The phase is expressed in the pipeline (named, gated) while the
  actual invocation lives in SKILL.md. Because the nested
  `/belt:requirements` `belt-agent init` becomes the newest run, all
  post-invocation wayfinder commands pin `--run <wayfinder-id>`. The gate
  is `file_exists belt://current/handoff-ref.json` (run-scoped), not a
  `docs/requirements/*/requirements.md` glob, which `execute_file_exists`
  (no mtime filter) would satisfy from the pre-existing requirements.md.
  Rejected: a static `invoke: skill: /belt:requirements` (cannot pass the
  dynamic ID); a workspace-global file_exists gate (spurious pass);
  omitting `--run` (targets the wrong run).
- **One run graduates one effort, not the whole map.** This is the
  anti-abandonment lesson applied: wayfinder deliberately slices a foggy
  map into per-effort runs, so each downstream requirements definition is
  bounded. Rejected: resolving the entire map to empty before a single
  handoff (reintroduces the long-run failure mode from the 2026-07-02
  abandonment analysis).
- **Three-level hierarchy (map → effort → decision) with label-based
  classification.** Efforts are the graduation unit; labels
  (`wayfinder-map`, `wf:effort`, `wf:decision` / `wf:research` /
  `wf:deep-hitl`) drive the frontier and gates deterministically.
  Rejected: two levels (no graduation unit for multi-effort maps);
  title-convention classification (fragile, not queryable).
- **Frontier computed client-side, scoped to the map ID.** Open
  `wf:decision` descendants of the map (via `linear api` GraphQL, since
  `issue query` filters by label but not parent) are filtered in the skill
  for unblocked (no open blocker in `relation list`) and unclaimed (no
  assignee). Rejected: server-side-only query (cannot express "unblocked"
  or "descendant-of-map"); label-only global query (leaks other maps'
  tickets — see the map-ref decision).
- **Workspace targeting via explicit `--workspace neko-neko` flags, no
  committed `.linear.toml`.** A repo-level `.linear.toml` would change the
  default workspace for *all* linear-cli use in the belt repo — a
  repo-wide side effect for an experimental skill. Rejected: committing
  `.linear.toml` (persistent, over-broad).
- **Research uses a general-purpose subagent, not a new research skill.**
  belt has no research skill and adding a dependency for the pilot is
  unwarranted; a self-contained subagent prompt (§3) suffices, with the
  orchestrator posting findings back. Rejected: vendoring mattpocock
  `/research` (new dependency, out of scope).
- **Experimental delivery as a plugins/belt skill, not a separate
  plugin.** Marked experimental in the SKILL.md description and README;
  promotion later is just removing the marker. Rejected: a `belt-lab`
  plugin (adds manifest/marketplace sync overhead for a trial).
