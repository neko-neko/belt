# linear-refresh Standalone Redesign

**Status**: Draft
**Date**: 2026-04-07

## Summary

Remove the belt pipeline from linear-refresh and return to a standalone SKILL.md design. Consolidate linear-cleanup and linear-add into linear-refresh. Add a new "Discover" step that uses keyword search and ticket reverse-lookup to find external sources not linked from any ticket.

## Problem

Running linear-refresh-v2 (belt pipeline version) on a 65-ticket team revealed two problems:

1. **Operational failure**: The collect phase produced `external_sources: 0` despite tickets containing URLs. The gate (`file_exists: ".belt/collected-context.json"`) passed because the file existed, even though it was effectively empty. Belt's deterministic gates cannot express the quality criteria this workflow needs.

2. **Design gap**: Two Slack messages (`p1775545880691739`, `p1775550564629869`) discussed relevant topics but were not linked from any Linear ticket. The current design only discovers external sources via ticket URLs (Linear → URLs → explore). There is no mechanism to discover unlinked external sources.

### Root Cause: Belt Pipeline is a Poor Fit

The belt pipeline model excels at workflows with clear file-based artifacts, deterministic gate verification, and value in resuming from specific phases (e.g., feature-dev). linear-refresh is fundamentally different:

- **Exploratory**: Quality criteria require LLM judgment, not file-exists checks
- **Heuristic-heavy**: Analysis is contextual pattern matching, not deterministic steps
- **One-shot**: If collect fails, you restart the whole workflow, not resume from a sub-phase
- **Overhead**: The init/next/verify/step ceremony added complexity without proportional value

Evidence of poor fit:
- Lean Orchestrator pattern (179.9k → ~45k context) was a patch for pipeline-induced context bloat
- collect sub-pipeline (4-phase decomposition) was tried and reverted — belt's phase granularity didn't match collect's nature
- The audit phase (regate + validate + max_retries) added ceremony that a simple self-check replaces

The standalone linear-cleanup and linear-add SKILL.md files already drove 4-phase workflows without belt.

## Design

### Workflow

```
Step 1: Collect    — Fetch tickets + explore linked URLs
Step 2: Discover   — Keyword search + ticket reverse-lookup for unlinked sources
Step 3: Analyze    — Cleanup + add analysis in a single pass
Step 4: Approve    — Present unified plan for user approval
Step 5: Execute    — Apply changes via Linear API
```

### Options

| Option | Effect |
|--------|--------|
| `--force` | Skip Step 4 (Approve) |
| `--skip-discovery` | Skip Step 2 (Discover) |
| `--cleanup-only` | Skip add analysis in Step 3 |
| `--add-only` | Skip cleanup analysis in Step 3 |

### File Structure

```
examples/skills/linear-refresh/
├── SKILL.md                              # Workflow + orchestration rules (~120-150 lines)
└── references/
    ├── collect-agent.md                  # Step 1 sub-agent instructions (rewritten)
    ├── discover-agent.md                 # Step 2 sub-agent instructions (NEW)
    ├── cleanup-guidelines.md             # Analysis guidelines (from linear-cleanup Phase 2)
    ├── add-guidelines.md                 # Detection criteria (from linear-add Phase 2)
    ├── collected-context-schema.md       # CollectedContext JSON schema (extended)
    ├── external-source-exploration.md    # URL filtering + summary budgets (existing)
    ├── discovery-strategy.md             # Search query generation strategy (NEW)
    └── execution-report.md              # Execution result format (existing)
```

### Artifacts Directory

`.linear-refresh/` (not `.belt/`) to make belt-independence explicit.

| Artifact | Written by | Read by |
|----------|-----------|---------|
| `.linear-refresh/collected-context.json` | Step 1 + Step 2 | Step 3 |
| `.linear-refresh/plan.json` | Step 3 | Step 4, Step 5 |
| `.linear-refresh/result.json` | Step 5 | — |

### Sub-Agent Strategy

Sub-agents are used **only for I/O parallelization** (Steps 1 and 2). Analysis, approval, and execution run in the main agent.

| Phase | Main agent | Sub-agents |
|-------|-----------|------------|
| Step 1: Collect | Team selection, URL filtering, 2-hop criteria, file write | Ticket detail batches (10 per batch), 1-hop exploration (per cluster), 2-hop exploration |
| Step 2: Discover | Query seed generation, dedup, file append | Slack search (per query group), GitHub search |
| Step 3: Analyze | All analysis | None |
| Step 4: Approve | Plan presentation, user interaction | None |
| Step 5: Execute | All API calls | None |

Sub-agents return data to the main agent (not file handoff). This is the opposite of the belt lean orchestrator pattern — justified because the main agent needs the data for analysis in Step 3.

---

## Step 1: Collect

### Purpose

Fetch all Linear tickets and explore external sources linked from ticket descriptions and attachments. This is the "known data collection" phase — ticket-centric, following existing URLs.

### Flow

```
Main: Team selection + ticket list fetch
  → Sub-agents (parallel): Active ticket detail fetch (10-ticket batches)
  → Main: URL extraction + filtering (per external-source-exploration.md)
  → Sub-agents (parallel): 1-hop linked URL exploration (per cluster)
  → Main: 2-hop criteria evaluation
  → Sub-agents (parallel): 2-hop exploration (qualifying URLs only)
  → Main: Write collected-context.json
```

### Main Agent Responsibilities

1. **Team selection**: `linear team list` → 1 team: auto-select / multiple: ask user / 0: error
2. **Ticket list fetch**: `linear issue list --team {id} --sort priority --all-states --all-assignees --limit 0 --no-pager`
3. **URL extraction + filtering**: Extract URLs from sub-agent results, classify per `external-source-exploration.md` (explore / metadata-only / skip)
4. **2-hop criteria**: Only expand for In Progress + Urgent/High + activity within 72 hours
5. **File write**: Merge all data into `.linear-refresh/collected-context.json` per `collected-context-schema.md`

### Sub-Agent Responsibilities

**Detail fetch batches** (parallel, 10 tickets each):
- Run `linear issue show {ticket_id}` for each ticket
- Extract: attachments, description_urls, relations
- Return: JSON array of enriched ticket objects

**1-hop exploration** (parallel, per ticket cluster):
- Load `/slackcli` for Slack thread exploration
- WebFetch as fallback
- Summary budgets: Backlog/Low: 200 chars, Todo/Medium: 400 chars, In Progress/High: 800 chars + raw excerpts
- Return: ExternalSource object array

**2-hop exploration** (parallel):
- Explore `referenced_urls` from 1-hop results
- Mark with `hop: 2`
- Return: ExternalSource object array

### collect-agent.md Role

Instructions for three types of I/O sub-agents (detail fetch, 1-hop, 2-hop). Each sub-agent section specifies: prerequisites (skills to load), input, procedure, return format.

---

## Step 2: Discover

### Purpose

Find external sources not linked from any ticket. Uses ticket context as search seeds to discover unlinked Slack messages, GitHub Issues/PRs, and other discussions.

### Flow

```
Main: Generate query seeds from collected-context.json
  → Main: Assemble search queries (per discovery-strategy.md)
  → Sub-agents (parallel): Slack search / GitHub search
  → Main: Filter results (dedup against Step 1 sources)
  → Main: Append to collected-context.json as discovery_sources
```

### Query Seed Generation

Extract from collected-context.json tickets:

| Seed type | Source | Example |
|-----------|--------|---------|
| Ticket ID | tickets[].id | `RAKMY-98`, `RAKMY-104` |
| Title keywords | tickets[].title | `月初ドロップダウン`, `RDS スケーリング` |
| Project name | tickets[].project (deduplicated) | `rakmy` |
| Label names | tickets[].labels (deduplicated) | `bug`, `infrastructure` |
| Assignee names | tickets[].assignee (deduplicated) | — |

**Priority weighting**: Seeds from In Progress / Urgent / High tickets get full extraction (ID + keywords + project + labels). Backlog / Low tickets contribute ID only (to prevent query explosion).

### Search Targets

| Source | Method | Query example |
|--------|--------|---------------|
| Slack | `/slackcli` search | `RAKMY-98`, `月初ドロップダウン`, `RDS スケーリング` |
| GitHub | `gh search issues` / `gh search prs` | `repo:rakmy/rakmy_server RAKMY-98` |

**Not searched**: Google Docs, Notion (no practical full-text search API), images, screenshots.

### Noise Control

| Control | Method |
|---------|--------|
| Time window | Last 30 days only |
| Dedup | Exclude URLs already in `external_sources[]` from Step 1 |
| Relevance | Sub-agents judge "is this related to these tickets?" — discard if unrelated |
| Cap | Max 30 results per source type. Prioritize by ticket priority + recency on overflow |

### Sub-Agent Responsibilities

**Slack search** (parallel, per query group):
- Load `/slackcli`
- Execute search queries, explore matching messages/threads
- Exclude URLs already collected in Step 1
- Apply summary budgets per `external-source-exploration.md`
- Return: DiscoverySource object array

**GitHub search** (parallel):
- Run `gh search issues` / `gh search prs`
- Summarize relevant comment/discussion sections
- Return: DiscoverySource object array

### Schema Extension

`collected-context-schema.md` gains a `discovery_sources[]` field:

```json
{
  "team_id": "string",
  "tickets": [],
  "external_sources": [],
  "discovery_sources": [
    {
      "url": "string",
      "discovery_query": "string",
      "related_tickets": ["string"],
      "accessible": true,
      "summary": "string",
      "latest_activity_ts": "string (ISO 8601)",
      "deferred_signals": ["string"],
      "source_type": "slack_message | slack_thread | github_issue | github_pr"
    }
  ]
}
```

**Why separate from external_sources**: Different provenance (ticket URL vs search query), different confidence level (linked = confirmed relationship, discovered = inferred), and traceability for debugging.

### discovery-strategy.md Content

- Seed generation rules (priority weighting, extraction targets)
- Per-source search query assembly
- Noise control parameters (time window, cap, dedup rules)
- Relevance judgment criteria for sub-agents

---

## Step 3: Analyze

### Purpose

Run cleanup analysis and add analysis in a single pass over collected-context.json. The main agent reads cleanup-guidelines.md and add-guidelines.md as inline references.

### Flow

```
Main: Read collected-context.json
  → Main: Read cleanup-guidelines.md, run cleanup analysis
  → Main: Read add-guidelines.md, run add analysis (referencing cleanup results for dedup)
  → Main: Self-check
  → Main: Write plan.json
```

### Why No Sub-Agents

- Cleanup and add analysis share the same input (collected-context.json)
- Add analysis references cleanup results for deduplication (sequential dependency)
- Analysis requires cross-ticket contextual understanding (not parallelizable)

### Cleanup Analysis

Per `cleanup-guidelines.md` (migrated from linear-cleanup SKILL.md Phase 2):

| Category | Detection target |
|----------|-----------------|
| Parent-child | description/scope containment → parent candidates |
| Blocking | "waiting for", "blocker" mentions, external source agreements |
| Related (relatedTo) | Cross-ticket mentions, same-thread discussion, causal relationships |
| Status inconsistency | archived+incomplete, PR closed but In Progress, blockedBy target Done |
| Duplicates | Same PR reference + same scope |
| Context gaps | Unregistered URLs, missing project assignment |
| Title inaccuracy | Divergence from external source discussions |
| Due date missing | Agreed deadlines not reflected |

### Add Analysis

Per `add-guidelines.md` (migrated from linear-add SKILL.md Phase 2):

| Detection axis | Target |
|---------------|--------|
| Explicit obligations | Agreed work without ticket, unanswered requests, untracked next steps |
| Intervention opportunities | Stalled discussions, directionless conversations, deadlocked without decision-maker |
| Structural gaps | Incomplete work decomposition, unregistered blockers |

Each candidate gets a disposition: `create` / `link` / `skip`.

### Discovery Sources Handling

| Source | Treatment |
|--------|-----------|
| `external_sources` (linked) | High confidence. Ticket relationship is confirmed. |
| `discovery_sources` (searched) | Medium confidence. Relevance judged by sub-agent but noise is possible. Rationale gets `[discovered]` tag for visibility at Approve. |

### Self-Check (Replaces Belt Audit)

After analysis completes, the main agent runs a one-pass self-check:

| Check | Action |
|-------|--------|
| In Progress coverage | Every In Progress ticket was analyzed at least once |
| Discovery utilization | Deferred signals from discovery_sources are reflected in the plan |
| Empty result sanity | If external_sources + discovery_sources are both 0 but tickets have URLs, report the anomaly to the user |

**Differences from belt audit:**
- No regate (collect re-execution). If problems are found, report to user for judgment.
- No max_retries loop. Self-check runs once.
- No step --confirm ceremony. The self-check is the agent's own quality control.

### Output

`.linear-refresh/plan.json`:

```json
{
  "summary": {
    "total_tickets": 65,
    "active_tickets": 37,
    "external_sources": 51,
    "discovery_sources": 8,
    "cleanup_changes": 14,
    "add_items": { "create": 2, "link": 3, "skip": 1 }
  },
  "cleanup": [
    {
      "id": "C-01",
      "category": "related",
      "ticket": "RAKMY-102",
      "action": "add_relation",
      "target": "RAKMY-103",
      "rationale": "RDS scaling — writer boost / reader xlarge, same Slack thread"
    }
  ],
  "add": [
    {
      "id": "A-01",
      "disposition": "create",
      "title": "...",
      "priority": "Medium",
      "rationale": "[discovered] Discussed in Slack #general ..."
    }
  ]
}
```

---

## Step 4: Approve

### Purpose

Present the unified plan to the user and get approval before execution.

### Display Format

```markdown
## Linear Refresh Plan

**Team:** {team_id} ({total_tickets} tickets, {external_sources} linked sources, {discovery_sources} discovered sources)

### Cleanup ({cleanup_changes} items)

#### Parent-child (N items)
| ID | Parent | Child | Rationale |

#### Related (N items)
| ID | Ticket | relatedTo | Rationale |

#### Blocking (N items)
| ID | Ticket | blockedBy | Rationale |

#### Status (N items)
| ID | Ticket | Current → New | Rationale |

#### Project assignment (N items)
| ID | Ticket | Project | Rationale |

#### Context addition (N items)
| ID | Ticket | Content | Rationale |

#### Title change (N items)
| ID | Ticket | Current → New | Rationale |

#### Due date (N items)
| ID | Ticket | Due Date | Rationale |

#### Duplicates (N items)
| ID | Ticket | duplicateOf | Rationale |

(Categories with 0 items are omitted)

### Add ({create + link} items)

#### Create (N items)
| ID | Title | Priority | Rationale |

#### Link (N items)
| ID | Ticket | Content | Rationale |

#### Skip (N items)
| ID | Content | Reason |

---
This plan will execute {cleanup_count} cleanup changes and {add_count} add actions.
Approve? (ok / modify / cancel)
```

### Approval Flow

- `ok` / approve → proceed to Step 5
- ID-based modification request → update plan.json and re-present
- `cancel` → exit
- `--force`: display plan but skip approval, proceed immediately

---

## Step 5: Execute

### Purpose

Apply the approved plan via Linear API.

### Execution Order

1. **Parent-child relationships** (sequential) — prerequisite for other relations
2. **Parallel**: blockedBy, relatedTo, status changes, project assignment, context additions, title changes, due dates
3. **Duplicate merges** (sequential) — Done + duplicateOf last
4. **Add: create** — new tickets first
5. **Add: link** — comments/relations to existing tickets

### Error Handling

| Error | Response |
|-------|----------|
| Individual ticket API error | Skip and continue, add to failures |
| Rate limit | Wait and retry (max 3) |
| Deleted/archived ticket | Skip, add to failures |
| Circular parent-child | Skip, add to failures |
| Cleanup failure | Does NOT block add execution |

### Output

`.linear-refresh/result.json` per `execution-report.md` format.

### Result Display

```markdown
## Refresh Result

### Cleanup
✓ Success: N items
✗ Failed: N items
- RAKMY-XX: reason

### Add
✓ Created: N tickets
✓ Linked: N tickets
✗ Failed: N items

### Changed Tickets
| Ticket | Type | Change |
```

---

## SKILL.md Structure

```
---
name: linear-refresh
description: >-
  Linearチームのチケット棚卸し・構造整理・新規検出を一気通貫で実行するスキル。
  チケットに紐付いた外部リンクの探索に加え、キーワード検索とチケット逆引きで
  未紐付きの外部ソースも発見する。
argument-hint: "[--force] [--skip-discovery] [--cleanup-only] [--add-only]"
---

# Linear Refresh

(summary)

## Options
## Prerequisites
## Workflow Overview
## Step 1: Collect
## Step 2: Discover
## Step 3: Analyze
## Step 4: Approve
## Step 5: Execute
## Red Flags
## Artifacts
```

Estimated size: ~120-150 lines. Analysis guideline details (cleanup: ~250 lines, add: ~200 lines) are in reference files, keeping the SKILL.md lean.

---

## Deleted Files

| File | Reason |
|------|--------|
| `examples/skills/linear-refresh/pipeline.yml` | Belt pipeline removed |
| `examples/skills/linear-refresh/belt.toml` | Belt config removed |
| `examples/skills/linear-refresh/linear-cleanup.yml` | Sub-pipeline removed |
| `examples/skills/linear-refresh/linear-add.yml` | Sub-pipeline removed |
| `examples/skills/linear-refresh/references/cleanup-agent.md` | → `cleanup-guidelines.md` |
| `examples/skills/linear-refresh/references/add-agent.md` | → `add-guidelines.md` |
| `examples/skills/linear-refresh/references/audit-agent.md` | Belt audit removed |
| `examples/skills/linear-refresh/references/approve-format.md` | Absorbed into SKILL.md Step 4 |
| `examples/skills/linear-refresh/references/ground-truth-audit.md` | Replaced by self-check in Step 3 |
| `examples/skills/linear-refresh/references/execute-agent.md` | Step 5 runs in main agent, no sub-agent needed |
| `examples/skills/linear-cleanup/` | Consolidated into linear-refresh |
| `examples/skills/linear-add/` | Consolidated into linear-refresh |

## Modified Files

| File | Change |
|------|--------|
| `examples/skills/linear-refresh/SKILL.md` | Complete rewrite — standalone workflow |
| `examples/skills/linear-refresh/references/collect-agent.md` | Rewrite for sub-agent I/O instructions (3 sub-agent types) |
| `examples/skills/linear-refresh/references/collected-context-schema.md` | Add `discovery_sources[]` field |

## New Files

| File | Purpose |
|------|---------|
| `examples/skills/linear-refresh/references/discover-agent.md` | Step 2 sub-agent instructions (Slack search, GitHub search) |
| `examples/skills/linear-refresh/references/cleanup-guidelines.md` | Analysis guidelines (from linear-cleanup Phase 2) |
| `examples/skills/linear-refresh/references/add-guidelines.md` | Detection criteria (from linear-add Phase 2) |
| `examples/skills/linear-refresh/references/discovery-strategy.md` | Query seed generation, noise control, search targets |

## Design Decisions

### Why remove belt pipeline

Belt's deterministic state machine (phases → file-exists gates → step) adds ceremony without proportional value for an exploratory, heuristic-heavy workflow. The standalone SKILL.md approach is simpler, cheaper (no init/next/verify/step overhead), and allows the main agent to hold analysis context continuously. Belt remains the right tool for deterministic multi-phase workflows like feature-dev.

### Why consolidate cleanup and add

Both require the same collected-context input. Running them as separate sub-agents (belt version) added ~117k tokens and 4 minutes of overhead for skill loading, context reading, and agent dispatch. A single-pass analysis reads the data once and avoids duplication. Standalone linear-cleanup and linear-add SKILL.md files are removed — the `--cleanup-only` and `--add-only` options replace standalone invocation.

### Why separate external_sources and discovery_sources

Different provenance (ticket URL vs search query), different confidence levels (linked = confirmed, discovered = inferred), and traceability for debugging. The `[discovered]` tag in rationale helps users distinguish at approval time.

### Why self-check instead of belt audit

The belt audit phase used regate (re-run collect) + validate (LLM criteria) + max_retries (loop). This required belt's step --confirm ceremony and added complexity. A one-pass self-check at the end of analysis achieves the same quality goal: verify In Progress coverage, discovery utilization, and empty-result sanity. If issues are found, report to user instead of automatic remediation loops.

### Why sub-agents return data to main (not file handoff)

The belt lean orchestrator used file handoff to keep the orchestrator's context lean. In the standalone design, the main agent needs collected data for analysis in Step 3 anyway. Returning data directly avoids the write-read indirection. File artifacts (collected-context.json, plan.json) exist for debugging and resumability, not for inter-phase communication.

## Related

- [linear-refresh belt migration spec](2026-04-06-linear-refresh-belt-migration.md) — original belt migration design
- [lean orchestrator spec](2026-04-06-linear-refresh-v2-lean-orchestrator.md) — context optimization (now superseded)
- [BELT-20](https://linear.app/neko-neko/issue/BELT-20) — belt redesign parent epic
