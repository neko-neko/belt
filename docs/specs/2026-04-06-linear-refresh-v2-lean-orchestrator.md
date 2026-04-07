# linear-refresh-v2 Lean Orchestrator Design

**Status**: Draft
**Date**: 2026-04-06

## Summary

Restructure the linear-refresh-v2 skill so the orchestrator LLM acts as a pure dispatcher, never holding ticket data or external source content in its context. Data flows exclusively through `.belt/` files, and domain skills are loaded only by sub-agents that need them.

## Problem

Running linear-refresh-v2 on a 35-ticket team consumed 179.9k tokens of orchestrator context. The root causes:

1. **Data relay**: Sub-agent results (ticket details, external source summaries) flow back to the orchestrator, which assembles `collected-context.json` in-context
2. **Skill stacking**: 5 SKILL.md files (belt-agent, linear-cli, slackcli, linear-cleanup, linear-add) are loaded into the orchestrator's context
3. **Monolithic collect**: The collect phase packs tickets fetch, detail fetch, 1-hop exploration, 2-hop expansion, and merge into a single phase with no belt-level checkpoints

This contradicts belt's design: phases loosely coupled via files.

## Design

### Principle: Orchestrator Holds No Data

The orchestrator's responsibilities are strictly:

1. Advance belt-agent phases (init / next / verify / step)
2. Dispatch sub-agents with file I/O instructions
3. Present the unified plan at the approve phase (the only phase where the orchestrator reads data)

The orchestrator does NOT: hold ticket data, read collected-context.json, load domain skills (linear-cli, slackcli, linear-cleanup, linear-add).

### collect Sub-Pipeline Decomposition

Break the monolithic collect phase into a 4-phase sub-pipeline so belt gates verify each intermediate artifact.

**pipeline.yml change:**

```yaml
phases:
  - id: collect
    description: "Fetch all tickets and explore external sources."
    uses: ./collect.yml  # was: inline phase with gate
  # ... remaining phases unchanged
```

**collect.yml (new):**

```yaml
name: collect
version: 1
phases:
  - id: tickets
    description: "Fetch ticket list and details from Linear."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/partial/tickets.json"

  - id: explore-1hop
    description: "1-hop external source exploration."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/partial/sources-1hop.json"

  - id: explore-2hop
    description: "2-hop recursive expansion for high-priority tickets."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/partial/sources-2hop.json"

  - id: merge
    description: "Merge partial files into collected-context.json."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/collected-context.json"
    regate: [tickets]
```

**Design decisions:**

- `explore-2hop` has no `when:` guard. When no tickets qualify for 2-hop expansion, the agent writes an empty array `[]` to `sources-2hop.json`. This avoids the merge agent needing to handle file-absent vs file-empty cases.
- `regate: [tickets]` on merge only. Explore failures retry at explore level; merge failure implies data integrity issues requiring a full recollect.

### Sub-Agent Dispatch Pattern

Each phase dispatches a sub-agent with:

1. A reference file path (the agent reads it for instructions)
2. Minimal parameters (team_id, hop level, etc.)
3. The expectation that the agent writes its output to `.belt/` files

The agent returns only a status summary (1-2 lines). The orchestrator never receives data payloads.

**Example — collect/tickets:**

```
Orchestrator dispatches Agent:
  "Read references/collect-tickets-agent.md and execute for team {team_id}.
   Return only a count summary."

Agent internally:
  1. Read(references/collect-tickets-agent.md)
  2. Skill(linear-cli) — loaded in agent context only
  3. linear issue list → fetch details (parallel sub-sub-agents)
  4. Write(.belt/partial/tickets.json)
  5. Returns: "60 tickets fetched, 35 active with details"
```

**Example — collect/explore-1hop:**

```
Orchestrator dispatches Agent:
  "Read references/collect-explore-agent.md and execute 1-hop exploration.
   Tickets are in .belt/partial/tickets.json.
   Return only a count summary."

Agent internally:
  1. Read(references/collect-explore-agent.md)
  2. Skill(slackcli)
  3. Read(.belt/partial/tickets.json) — extracts URLs, clusters
  4. Parallel sub-sub-agents write per-cluster results
  5. Aggregates into Write(.belt/partial/sources-1hop.json)
  6. Returns: "63 external sources explored across 5 clusters"
```

**approve phase is the exception:**

The orchestrator directly reads `.belt/refresh-plan.json` (~32 lines) and presents it to the user. This is the only phase where the orchestrator touches data, justified because user interaction requires it and the file is small.

### SKILL.md Split

**Current:** 1 file (114 lines) containing phase map + all domain instructions + HARD-GATE.

**After:** SKILL.md contains orchestration instructions only (~40 lines). Domain knowledge moves to reference files read by sub-agents.

```
examples/skills/linear-refresh/
├── SKILL.md                             # orchestration only
├── pipeline.yml                         # updated: collect uses sub-pipeline
├── collect.yml                          # NEW: collect sub-pipeline
├── belt.toml
├── linear-cleanup.yml                   # unchanged
├── linear-add.yml                       # unchanged
└── references/
    ├── collect-tickets-agent.md          # NEW
    ├── collect-explore-agent.md          # NEW (shared 1hop/2hop)
    ├── collect-merge-agent.md            # NEW
    ├── cleanup-agent.md                  # NEW
    ├── add-agent.md                      # NEW
    ├── audit-agent.md                    # NEW
    ├── execute-agent.md                  # NEW
    ├── approve-format.md                 # NEW
    ├── collected-context-schema.md       # existing (referenced by merge/explore agents)
    ├── external-source-exploration.md    # existing (referenced by explore agents)
    ├── execution-report.md              # existing (referenced by execute agent)
    └── ground-truth-audit.md            # existing (referenced by audit agent)
```

**SKILL.md phase map:**

| Phase | Action | Agent Reference |
|-------|--------|-----------------|
| collect/tickets | Dispatch tickets-agent | references/collect-tickets-agent.md |
| collect/explore-1hop | Dispatch explore-agent (hop=1) | references/collect-explore-agent.md |
| collect/explore-2hop | Dispatch explore-agent (hop=2) | references/collect-explore-agent.md |
| collect/merge | Dispatch merge-agent | references/collect-merge-agent.md |
| cleanup-analysis/analyze | Dispatch cleanup-agent | references/cleanup-agent.md |
| add-analysis/analyze | Dispatch add-agent | references/add-agent.md |
| audit | Dispatch audit-agent | references/audit-agent.md |
| approve | Orchestrator reads refresh-plan.json | references/approve-format.md |
| execute | Dispatch execute-agent | references/execute-agent.md |

**HARD-GATE removal:** The current HARD-GATE requires the orchestrator to load `/linear-cli` and `/slackcli`. In the new design, these are loaded by sub-agents. The HARD-GATE is removed from the orchestrator's SKILL.md and replaced by prerequisites in the agent reference files.

### Skill Loading Delegation

| Skill | Loaded by | Not loaded by |
|-------|-----------|---------------|
| `/belt-agent` | Orchestrator | — |
| `/linear-cli` | collect-tickets-agent, execute-agent | Orchestrator |
| `/slackcli` | collect-explore-agent | Orchestrator |
| `/linear-cleanup` | cleanup-agent | Orchestrator |
| `/linear-add` | add-agent | Orchestrator |

### Sub-Agent Internal Aggregation

The collect-explore-agent dispatches parallel sub-sub-agents (one per URL cluster). These sub-sub-agents return per-URL ExternalSource objects to the explore-agent, which aggregates them into `sources-{1hop|2hop}.json`.

This is acceptable because:

1. The explore-agent's context is disposable (scoped to one phase)
2. Each sub-sub-agent returns a single ExternalSource entry (bounded by summary budget: 200/400/800 chars)
3. For 63 sources at ~400 chars average, the aggregation adds ~25k tokens — within a single-phase agent's budget

The partial-file pattern is NOT applied recursively inside explore-agent. Over-engineering internal structure adds complexity without proportional benefit since the agent context is discarded after the phase.

### Context Budget Estimate

**Orchestrator (after):**

| Item | Estimated tokens |
|------|-----------------|
| System prompt + CLAUDE.md | ~30k (environment, not reducible) |
| belt-agent SKILL.md | ~3k |
| linear-refresh-v2 SKILL.md (reduced) | ~2k |
| Per-phase belt-agent JSON + agent summary | ~1-2k per phase |
| approve: refresh-plan.json read | ~3k |
| **Total** | **~45k** |

**vs. Current: 179.9k** — approximately 75% reduction in orchestrator context.

**Scalability:** Ticket count (35 → 200) increases sub-agent context, not orchestrator context.

## File Changes

### Modified

| File | Change |
|------|--------|
| `examples/skills/linear-refresh/SKILL.md` | Reduce to orchestration-only instructions |
| `examples/skills/linear-refresh/pipeline.yml` | collect phase: `uses: ./collect.yml` |

### New

| File | Purpose |
|------|---------|
| `examples/skills/linear-refresh/collect.yml` | collect sub-pipeline (4 phases) |
| `references/collect-tickets-agent.md` | tickets sub-agent instructions |
| `references/collect-explore-agent.md` | 1hop/2hop shared exploration instructions |
| `references/collect-merge-agent.md` | merge sub-agent instructions |
| `references/cleanup-agent.md` | cleanup-analysis sub-agent instructions |
| `references/add-agent.md` | add-analysis sub-agent instructions |
| `references/audit-agent.md` | audit sub-agent instructions |
| `references/execute-agent.md` | execute sub-agent instructions |
| `references/approve-format.md` | approve phase display format |

### Unchanged

| File | Reason |
|------|--------|
| `linear-cleanup.yml` | Sub-pipeline definition unchanged |
| `linear-add.yml` | Sub-pipeline definition unchanged |
| `belt.toml` | Path resolution unchanged |
| `references/collected-context-schema.md` | Schema unchanged |
| `references/external-source-exploration.md` | Exploration rules unchanged |
| `references/execution-report.md` | Report format unchanged |
| `references/ground-truth-audit.md` | Audit questions unchanged |
| `crates/belt-core/**` | No engine changes (Approach A) |

### Out of Scope

| Item | Reason |
|------|--------|
| `/linear-cleanup` standalone SKILL.md | Used by sub-agent via Skill(); also used standalone |
| `/linear-add` standalone SKILL.md | Same |
| `/linear-cli`, `/slackcli` SKILL.md | Tool definitions, unchanged |
| dotfiles old linear-refresh | Deprecated, left as-is |
| belt engine context isolation | Deferred to Approach B if needed |
