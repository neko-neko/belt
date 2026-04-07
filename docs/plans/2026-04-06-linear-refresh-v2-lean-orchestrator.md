# linear-refresh-v2 Lean Orchestrator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure linear-refresh-v2 so the orchestrator LLM is a pure dispatcher, reducing context from 179.9k to ~45k by routing all data through `.belt/` files and delegating skill loading to sub-agents.

**Architecture:** Decompose the monolithic collect phase into a 4-phase sub-pipeline (`collect.yml`). Split SKILL.md into orchestration-only instructions + 9 per-phase agent reference files. Each pipeline phase dispatches a sub-agent that reads a reference file, loads required skills, processes data via files, and returns only a summary.

**Tech Stack:** belt pipeline YAML, Claude Code skills (markdown), belt lint for validation

**Spec:** `docs/specs/2026-04-06-linear-refresh-v2-lean-orchestrator.md`

---

### Task 1: Create collect sub-pipeline and update pipeline.yml

**Files:**
- Create: `examples/skills/linear-refresh/collect.yml`
- Modify: `examples/skills/linear-refresh/pipeline.yml`

- [ ] **Step 1: Create collect.yml**

Create `examples/skills/linear-refresh/collect.yml`:

```yaml
name: collect
version: 1
description: "Fetch all tickets and explore external sources in 4 stages."
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

- [ ] **Step 2: Update pipeline.yml**

Replace the `collect` phase in `examples/skills/linear-refresh/pipeline.yml`. The full file becomes:

```yaml
name: linear-refresh
version: 1
args:
  force: { type: bool, default: false }

phases:
  - id: collect
    description: "Fetch all tickets and explore external sources (1-hop + 2-hop)."
    uses: ./collect.yml

  - id: cleanup-analysis
    description: "Analyze tickets for structural issues."
    uses: ./linear-cleanup.yml

  - id: add-analysis
    description: "Detect new ticket candidates from external sources."
    uses: ./linear-add.yml

  - id: audit
    description: "Ground Truth audit — verify CollectedContext completeness and Plan quality."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-plan.json"
    validate:
      - "Every In Progress ticket's latest context is reflected in the plan"
      - "No untracked references remain for high-priority tickets"
      - "Deferred signals from external sources are addressed in the plan"
    regate: [collect]
    max_retries: 2

  - id: approve
    description: "Present unified plan for user approval."
    when: "!args.force"
    config:
      skill: "/linear-refresh"
    confirm: true

  - id: execute
    description: "Execute cleanup changes, then add changes."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-result.json"
```

- [ ] **Step 3: Run belt lint to verify pipeline expansion**

Run: `cargo run -p belt --quiet -- lint examples/skills/linear-refresh/pipeline.yml`

Expected: `ok: examples/skills/linear-refresh/pipeline.yml`

This validates:
- `uses: ./collect.yml` resolves correctly
- Sub-pipeline phases expand to `collect/tickets`, `collect/explore-1hop`, `collect/explore-2hop`, `collect/merge`
- `regate: [tickets]` target exists within collect.yml
- `regate: [collect]` on audit resolves to the expanded `collect/tickets` (first sub-phase)
- All phases have `description`
- No duplicate IDs

- [ ] **Step 4: Commit**

```bash
git add examples/skills/linear-refresh/collect.yml examples/skills/linear-refresh/pipeline.yml
git commit -m "refactor(linear-refresh): decompose collect into 4-phase sub-pipeline

Break monolithic collect phase into tickets → explore-1hop →
explore-2hop → merge with per-step belt gates and regate."
```

---

### Task 2: Create collect-tickets-agent reference

**Files:**
- Create: `examples/skills/linear-refresh/references/collect-tickets-agent.md`

- [ ] **Step 1: Create collect-tickets-agent.md**

Create `examples/skills/linear-refresh/references/collect-tickets-agent.md`:

```markdown
# Collect Tickets Agent

Sub-agent instructions for the `collect/tickets` phase.

## Prerequisites

1. Invoke `/linear-cli` skill to load CLI usage patterns.
2. Verify: `which linear && linear --version`

## Input

- `team_id` parameter from orchestrator (e.g., "RAKMY")

## Output

- Write: `.belt/partial/tickets.json`
- Return to orchestrator: count summary only (e.g., "60 tickets fetched, 35 active with details")

## Procedure

### Step 1: Team Selection

If `team_id` is not provided:
1. Run `linear team list` to list available teams.
2. If 1 team → auto-select. If multiple → ask user. If 0 → error.

### Step 2: Fetch All Tickets

Run: `linear issue list --team {team_id} --sort priority --all-states --all-assignees --limit 0 --no-pager`

Record for each ticket:
- id, title, status, priority, labels, project, parentId, assignee
- completedAt, archivedAt

### Step 3: Fetch Active Ticket Details

Filter: status is not Done/Cancelled, OR updated within the last 30 days.

Dispatch parallel sub-agents in batches of 10 tickets. Each sub-agent:
1. Runs `linear issue show {ticket_id}` for each ticket in its batch.
2. Extracts: attachments (URL + title), description URLs (regex `https?://[^\s)]+`), relations (relatedTo, blocks, blockedBy).
3. Returns a JSON array of enriched ticket objects.

Aggregate all batch results into a single array.

### Step 4: Write Output

Write the aggregated ticket array to `.belt/partial/tickets.json`:

```json
{
  "team_id": "RAKMY",
  "fetched_at": "2026-04-06T20:00:00+09:00",
  "tickets": [
    {
      "id": "RAKMY-98",
      "title": "...",
      "status": "In Review",
      "priority": "High",
      "labels": ["Feature"],
      "project": null,
      "parentId": null,
      "assignee": "SN",
      "completedAt": null,
      "archivedAt": null,
      "attachments": [{"url": "...", "title": "..."}],
      "description_urls": ["https://..."],
      "relations": {"relatedTo": [], "blocks": [], "blockedBy": []}
    }
  ]
}
```

### Step 5: Return Summary

Return ONLY a summary string to the orchestrator. Do NOT return ticket data.

Example: "60 tickets fetched (35 active with details), written to .belt/partial/tickets.json"
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/linear-refresh/references/collect-tickets-agent.md
git commit -m "docs(linear-refresh): add collect-tickets-agent reference"
```

---

### Task 3: Create collect-explore-agent reference

**Files:**
- Create: `examples/skills/linear-refresh/references/collect-explore-agent.md`

- [ ] **Step 1: Create collect-explore-agent.md**

Create `examples/skills/linear-refresh/references/collect-explore-agent.md`:

```markdown
# Collect Explore Agent

Sub-agent instructions for `collect/explore-1hop` and `collect/explore-2hop` phases.
This single reference serves both phases — behavior varies by the `hop` parameter.

## Prerequisites

1. Invoke `/slackcli` skill for Slack thread exploration.

## Input

- `hop` parameter: `1` (1-hop) or `2` (2-hop)
- File: `.belt/partial/tickets.json` (from collect/tickets phase)
- If hop=2: File `.belt/partial/sources-1hop.json` (for `referenced_urls` extraction)

## Output

- Write: `.belt/partial/sources-{1hop|2hop}.json`
- Return to orchestrator: count summary only

## Procedure

### Step 1: Load Tickets and Extract URLs

Read `.belt/partial/tickets.json`.

**If hop=1:**
Extract all URLs from `description_urls` and `attachments[].url` across all tickets.

**If hop=2:**
Read `.belt/partial/sources-1hop.json`. Extract `referenced_urls` from sources where:
- The referring ticket is **In Progress** AND **Urgent or High** priority
- `latest_activity_ts` is within **72 hours** of current time

If no URLs qualify for 2-hop, write an empty array and return.

### Step 2: Filter URLs

Classify each URL per [external-source-exploration.md](external-source-exploration.md):

| Classification | Action |
|---------------|--------|
| Explore (full) | Slack threads, GitHub issues/PRs with discussion |
| Metadata only | Google Docs/Sheets, Notion — title + accessibility check |
| Skip | Images, screenshots, already-registered attachments |

Deduplicate against already-explored URLs (for hop=2, exclude all hop=1 URLs).

### Step 3: Cluster and Dispatch

Group URLs by referring ticket cluster (related tickets that share a feature area).
Dispatch parallel sub-agents per cluster. Each sub-agent:

1. Explores its assigned URLs using available tools (slackcli for Slack, WebFetch as fallback).
2. Applies summary budget per [external-source-exploration.md](external-source-exploration.md):
   - Backlog/Low: 200 chars
   - Todo/Medium: 400 chars
   - In Progress/Urgent/High: 800 chars + raw excerpts
3. Returns one ExternalSource object per URL:

```json
{
  "url": "https://...",
  "ticket_id": "RAKMY-98",
  "hop": 1,
  "accessible": true,
  "summary": "...",
  "referenced_urls": ["https://..."],
  "latest_activity_ts": "2026-04-06T12:00:00Z",
  "deferred_signals": ["confirming"],
  "source_type": "slack_thread"
}
```

### Step 4: Aggregate and Write

Collect all ExternalSource objects from sub-agents into a single array.
Write to `.belt/partial/sources-{1hop|2hop}.json`:

```json
[
  { "url": "...", "ticket_id": "...", "hop": 1, ... },
  { "url": "...", "ticket_id": "...", "hop": 1, ... }
]
```

**Note:** Aggregation within this agent is acceptable. The agent's context is disposable
(scoped to one phase). Each sub-agent returns a single ExternalSource entry, bounded
by summary budget. See spec: "Sub-Agent Internal Aggregation" section.

### Step 5: Return Summary

Return ONLY a summary string. Do NOT return source data.

Example: "63 external sources explored across 5 clusters, written to .belt/partial/sources-1hop.json"
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/linear-refresh/references/collect-explore-agent.md
git commit -m "docs(linear-refresh): add collect-explore-agent reference (1hop/2hop shared)"
```

---

### Task 4: Create collect-merge-agent reference

**Files:**
- Create: `examples/skills/linear-refresh/references/collect-merge-agent.md`

- [ ] **Step 1: Create collect-merge-agent.md**

Create `examples/skills/linear-refresh/references/collect-merge-agent.md`:

```markdown
# Collect Merge Agent

Sub-agent instructions for the `collect/merge` phase.

## Prerequisites

None. This agent performs pure file I/O — no external tools or skills required.

## Input

- `.belt/partial/tickets.json`
- `.belt/partial/sources-1hop.json`
- `.belt/partial/sources-2hop.json`

## Output

- Write: `.belt/collected-context.json`
- Return to orchestrator: count summary only

## Procedure

### Step 1: Read Partial Files

Read all three input files.

### Step 2: Merge into CollectedContext

Build the merged structure per [collected-context-schema.md](collected-context-schema.md):

```json
{
  "team_id": "<from tickets.json>",
  "collected_at": "<current ISO 8601 timestamp>",
  "tickets": "<tickets array from tickets.json>",
  "external_sources": "<concat sources-1hop.json + sources-2hop.json>"
}
```

### Step 3: Write Output

Write to `.belt/collected-context.json`.

### Step 4: Return Summary

Return ONLY a summary string.

Example: "Merged: 35 tickets, 66 external sources (63 hop-1 + 3 hop-2), written to .belt/collected-context.json"
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/linear-refresh/references/collect-merge-agent.md
git commit -m "docs(linear-refresh): add collect-merge-agent reference"
```

---

### Task 5: Create cleanup-agent and add-agent references

**Files:**
- Create: `examples/skills/linear-refresh/references/cleanup-agent.md`
- Create: `examples/skills/linear-refresh/references/add-agent.md`

- [ ] **Step 1: Create cleanup-agent.md**

Create `examples/skills/linear-refresh/references/cleanup-agent.md`:

```markdown
# Cleanup Agent

Sub-agent instructions for the `cleanup-analysis/analyze` phase.

## Prerequisites

1. Invoke `/linear-cleanup` skill to load analysis guidelines.

## Input

- File: `.belt/collected-context.json`

## Output

- Write: `.belt/plan-a.json`
- Return to orchestrator: count summary only

## Procedure

### Step 1: Load Guidelines

Invoke `/linear-cleanup` skill. Follow its Phase 2 (Merge + Analyze) analysis guidelines.

### Step 2: Read CollectedContext

Read `.belt/collected-context.json`.

### Step 3: Analyze

Detect change candidates across these categories:
- Parent-child relationships
- Blocking relationships (blockedBy)
- Related tickets (relatedTo)
- Status inconsistencies
- Duplicate tickets
- Context gaps (missing attachments, comments)

### Step 4: Write Plan A

Write `.belt/plan-a.json` following the linear-cleanup Phase 3 plan format:

```json
{
  "type": "cleanup",
  "summary": {
    "tickets_analyzed": 35,
    "external_sources_explored": 66,
    "changes_detected": 14
  },
  "changes": [
    {
      "id": "C-01",
      "category": "related",
      "ticket": "ISSUE-XX",
      "action": "add_relation",
      "target": "ISSUE-YY",
      "relation_type": "relatedTo",
      "rationale": "..."
    }
  ]
}
```

### Step 5: Return Summary

Return ONLY: "14 cleanup changes detected, written to .belt/plan-a.json"
```

- [ ] **Step 2: Create add-agent.md**

Create `examples/skills/linear-refresh/references/add-agent.md`:

```markdown
# Add Agent

Sub-agent instructions for the `add-analysis/analyze` phase.

## Prerequisites

1. Invoke `/linear-add` skill to load detection criteria.

## Input

- File: `.belt/collected-context.json`
- File: `.belt/plan-a.json` (for deduplication against cleanup changes)

## Output

- Write: `.belt/plan-b.json`
- Return to orchestrator: count summary only

## Procedure

### Step 1: Load Guidelines

Invoke `/linear-add` skill. Follow its Phase 2 (Analyze) detection criteria.

### Step 2: Read Inputs

Read `.belt/collected-context.json` and `.belt/plan-a.json`.

### Step 3: Analyze

Detect new ticket candidates from external sources. For each candidate, classify as:
- `create` — new ticket needed
- `link` — add context to existing ticket (comment/attachment)
- `skip` — not actionable or premature

Exclude items already covered by Plan A changes (deduplication).

### Step 4: Write Plan B

Write `.belt/plan-b.json` following the linear-add Phase 3 plan format:

```json
{
  "type": "add",
  "summary": {
    "items_detected": 6,
    "create": 2,
    "link": 2,
    "skip": 2
  },
  "items": [
    {
      "id": "A-01",
      "disposition": "create",
      "title": "...",
      "priority": "Medium",
      "status": "Backlog",
      "rationale": "...",
      "relations": {"parent": "ISSUE-XX", "blockedBy": ["ISSUE-YY"]}
    }
  ]
}
```

### Step 5: Return Summary

Return ONLY: "6 items detected (2 create, 2 link, 2 skip), written to .belt/plan-b.json"
```

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/cleanup-agent.md examples/skills/linear-refresh/references/add-agent.md
git commit -m "docs(linear-refresh): add cleanup-agent and add-agent references"
```

---

### Task 6: Create audit-agent, execute-agent, and approve-format references

**Files:**
- Create: `examples/skills/linear-refresh/references/audit-agent.md`
- Create: `examples/skills/linear-refresh/references/execute-agent.md`
- Create: `examples/skills/linear-refresh/references/approve-format.md`

- [ ] **Step 1: Create audit-agent.md**

Create `examples/skills/linear-refresh/references/audit-agent.md`:

```markdown
# Audit Agent

Sub-agent instructions for the `audit` phase.

## Prerequisites

1. Invoke `/slackcli` skill (for remediation single-shot exploration if needed).

## Input

- File: `.belt/collected-context.json`
- File: `.belt/plan-a.json`
- File: `.belt/plan-b.json`
- Reference: [ground-truth-audit.md](ground-truth-audit.md) for Q1-Q3 audit questions

## Output

- Write: `.belt/refresh-plan.json`
- Return to orchestrator: audit result summary only

## Procedure

### Step 1: Read Inputs

Read all three data files and `ground-truth-audit.md`.

### Step 2: Run Audit

For each **In Progress** and **In Review** ticket, answer the 3 audit questions from ground-truth-audit.md:

- **Q1 (Implementation Context):** Latest specs/decisions reflected in Plan A/B?
- **Q2 (Recent Activity):** Deferred signals from last 72h addressed?
- **Q3 (Untracked References):** Any unexplored referenced_urls remaining?

### Step 3: Remediation (if needed)

If any question reveals a gap requiring additional exploration:

1. Run single-shot exploration for specific URL(s) only.
2. Update the relevant plan (Plan A or Plan B) with new findings.
3. Re-audit affected tickets.

This loop is bounded by `max_retries: 2` on the audit phase in pipeline.yml.

### Step 4: Generate Unified Plan

Merge Plan A and Plan B into `.belt/refresh-plan.json`:

```json
{
  "summary": {
    "total_tickets": 35,
    "cleanup_changes": 14,
    "add_detections": {"create": 2, "link": 2, "skip": 2},
    "external_sources": {"explored": 63, "skipped": 3, "failed": 0}
  },
  "cleanup": [],
  "add": []
}
```

### Step 5: Return Summary

Return ONLY: "Audit passed. Unified plan: 14 cleanup + 6 add items, written to .belt/refresh-plan.json"

If remediation was performed: "Audit: 1 remediation cycle. 2 gaps filled. Unified plan: ..."
```

- [ ] **Step 2: Create execute-agent.md**

Create `examples/skills/linear-refresh/references/execute-agent.md`:

```markdown
# Execute Agent

Sub-agent instructions for the `execute` phase.

## Prerequisites

1. Invoke `/linear-cli` skill for ticket manipulation commands.

## Input

- File: `.belt/refresh-plan.json`
- Reference: [execution-report.md](execution-report.md) for output format and execution order

## Output

- Write: `.belt/refresh-result.json`
- Return to orchestrator: execution result summary only

## Procedure

### Step 1: Read Plan

Read `.belt/refresh-plan.json` and `execution-report.md`.

### Step 2: Execute Cleanup Changes

Follow the strict execution order from execution-report.md:

1. **Parent-child relationships** (sequential)
2. **Parallel:** blockedBy, relatedTo, status changes, project assignment, context additions
3. **Duplicate merges** — set Done + duplicateOf (sequential)

For each change, use `linear issue update` or `linear issue comment` as appropriate.

### Step 3: Execute Add Changes

1. **Create** new tickets via `linear issue create`
2. **Link** to existing tickets via `linear issue comment` or `linear issue update`

### Step 4: Error Handling

Per execution-report.md:
- Individual failures: skip and continue, add to failures list
- Rate limits: wait and retry (max 3)
- Deleted/archived tickets: skip
- Circular parent-child: skip
- Cleanup failures do NOT block Add execution

### Step 5: Write Result

Write `.belt/refresh-result.json` per the schema in execution-report.md.

### Step 6: Return Summary

Return ONLY: "Executed: 13/14 cleanup (1 failed), 4/4 add. Written to .belt/refresh-result.json"
```

- [ ] **Step 3: Create approve-format.md**

Create `examples/skills/linear-refresh/references/approve-format.md`:

```markdown
# Approve Phase Format

Display format for the orchestrator to present the unified plan to the user.

## Input

The orchestrator reads `.belt/refresh-plan.json` directly (this is the only phase
where the orchestrator reads data files).

## Display Format

Present the plan using this structure:

### Header

```
## Linear Refresh Plan

**Team:** {team_id} ({total_tickets} tickets analyzed, {external_sources.explored} external sources explored)
```

### Cleanup Section

Group changes by category with tables:

| Category | Table Columns |
|----------|--------------|
| Parent-child (parent) | Ticket, Parent, Rationale |
| Blocking (blocking) | Ticket, blockedBy, Rationale |
| Related (related) | Ticket, relatedTo, Rationale |
| Context addition (context_addition) | ID, Ticket, Content |
| Project assignment (project) | ID, Ticket, Project, Rationale |
| Due date (due_date) | ID, Ticket, Due Date, Rationale |
| Title change (title) | ID, Ticket, Current → New |
| Duplicate (duplicate) | ID, Ticket, duplicateOf, Rationale |

Omit categories with 0 items.

### Add Section

Group items by disposition:

| Disposition | Table Columns |
|------------|--------------|
| create | ID, Title, Priority, Status |
| link | ID, Ticket, Content |
| skip | ID, Content, Reason |

### Approval Prompt

```
This plan will execute {cleanup_count} cleanup changes and {add_create + add_link} add actions.
Approve? (ok / modify / cancel)
```

## Approval Flow

- `ok` / approve → proceed to execute phase
- Modification request → update `.belt/refresh-plan.json` and re-present
- `cancel` → exit pipeline
- `--force` mode: display plan but skip waiting, proceed immediately
```

- [ ] **Step 4: Commit**

```bash
git add examples/skills/linear-refresh/references/audit-agent.md examples/skills/linear-refresh/references/execute-agent.md examples/skills/linear-refresh/references/approve-format.md
git commit -m "docs(linear-refresh): add audit, execute, and approve-format references"
```

---

### Task 7: Rewrite SKILL.md to orchestration-only

**Files:**
- Modify: `examples/skills/linear-refresh/SKILL.md`

- [ ] **Step 1: Rewrite SKILL.md**

Replace the entire content of `examples/skills/linear-refresh/SKILL.md` with:

```markdown
---
name: linear-refresh-v2
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Invoke `/belt-agent` for protocol details.

## Role

You are the **orchestrator**. Your responsibilities:

1. Advance belt-agent phases (init → next → verify → step)
2. Dispatch sub-agents per phase with a reference file path
3. Present the unified plan at the approve phase

You do NOT:
- Hold ticket data or external source content
- Read `.belt/collected-context.json`, `.belt/plan-a.json`, or `.belt/plan-b.json`
- Load domain skills (`/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add`)

Sub-agents handle all data processing. They read reference files for instructions, load required skills, and write output to `.belt/` files. You receive only a summary string.

## Phase Map

| Phase | Dispatch | Reference |
|-------|----------|-----------|
| collect/tickets | Agent: fetch ticket list and details | [collect-tickets-agent.md](references/collect-tickets-agent.md) |
| collect/explore-1hop | Agent: 1-hop external source exploration | [collect-explore-agent.md](references/collect-explore-agent.md) (hop=1) |
| collect/explore-2hop | Agent: 2-hop recursive expansion | [collect-explore-agent.md](references/collect-explore-agent.md) (hop=2) |
| collect/merge | Agent: merge partial files | [collect-merge-agent.md](references/collect-merge-agent.md) |
| cleanup-analysis/analyze | Agent: detect structural issues | [cleanup-agent.md](references/cleanup-agent.md) |
| add-analysis/analyze | Agent: detect new ticket candidates | [add-agent.md](references/add-agent.md) |
| audit | Agent: ground truth audit + unified plan | [audit-agent.md](references/audit-agent.md) |
| approve | **Orchestrator**: read refresh-plan.json, present | [approve-format.md](references/approve-format.md) |
| execute | Agent: run cleanup then add changes | [execute-agent.md](references/execute-agent.md) |

## Dispatch Pattern

For each phase (except approve):

1. `belt-agent next` — get current phase info
2. Read the phase's reference file path from the table above
3. Dispatch Agent with prompt:

   > Read `{reference_path}` and execute.
   > Team: {team_id}. [Phase-specific parameters if any.]
   > Return only a count summary.

4. Agent completes → receive summary string
5. `belt-agent verify` → `belt-agent step` → next phase

## approve Phase (Orchestrator Direct)

The only phase where the orchestrator reads data:

1. `belt-agent next` — get approve phase info
2. Read `.belt/refresh-plan.json` (~30 lines)
3. Read [approve-format.md](references/approve-format.md) for display layout
4. Present formatted plan to user
5. Handle approval / modification / cancellation
6. `belt-agent step --confirm`

## Output Files

| File | Produced by |
|------|------------|
| `.belt/partial/tickets.json` | collect-tickets-agent |
| `.belt/partial/sources-1hop.json` | collect-explore-agent (hop=1) |
| `.belt/partial/sources-2hop.json` | collect-explore-agent (hop=2) |
| `.belt/collected-context.json` | collect-merge-agent |
| `.belt/plan-a.json` | cleanup-agent |
| `.belt/plan-b.json` | add-agent |
| `.belt/refresh-plan.json` | audit-agent |
| `.belt/refresh-result.json` | execute-agent |

## Red Flags

**Never:**
- Return ticket data from sub-agents to the orchestrator
- Load `/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add` in the orchestrator
- Read `.belt/collected-context.json` in the orchestrator
- Execute Linear API calls from the orchestrator
- Explore beyond 2 hops

**Always:**
- Dispatch one agent per phase
- Pass reference file path in the agent prompt
- Verify sub-agent wrote the expected gate file before running `belt-agent verify`
```

- [ ] **Step 2: Run belt lint**

Run: `cargo run -p belt --quiet -- lint examples/skills/linear-refresh/pipeline.yml`

Expected: `ok: examples/skills/linear-refresh/pipeline.yml`

SKILL.md changes don't affect lint — this confirms the pipeline is still valid.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/SKILL.md
git commit -m "refactor(linear-refresh): rewrite SKILL.md to orchestration-only

Orchestrator dispatches sub-agents per phase, never holds data.
Domain skills loaded by sub-agents only. Context: 179.9k → ~45k."
```

---

### Task 8: Final verification

**Files:**
- None modified (verification only)

- [ ] **Step 1: Verify file structure**

Run: `find examples/skills/linear-refresh -type f | sort`

Expected:
```
examples/skills/linear-refresh/SKILL.md
examples/skills/linear-refresh/belt.toml
examples/skills/linear-refresh/collect.yml
examples/skills/linear-refresh/linear-add.yml
examples/skills/linear-refresh/linear-cleanup.yml
examples/skills/linear-refresh/pipeline.yml
examples/skills/linear-refresh/references/add-agent.md
examples/skills/linear-refresh/references/approve-format.md
examples/skills/linear-refresh/references/audit-agent.md
examples/skills/linear-refresh/references/cleanup-agent.md
examples/skills/linear-refresh/references/collect-explore-agent.md
examples/skills/linear-refresh/references/collect-merge-agent.md
examples/skills/linear-refresh/references/collect-tickets-agent.md
examples/skills/linear-refresh/references/collected-context-schema.md
examples/skills/linear-refresh/references/execution-report.md
examples/skills/linear-refresh/references/external-source-exploration.md
examples/skills/linear-refresh/references/ground-truth-audit.md
examples/skills/linear-refresh/references/execute-agent.md
```

- [ ] **Step 2: Verify belt lint passes**

Run: `cargo run -p belt --quiet -- lint examples/skills/linear-refresh/pipeline.yml`

Expected: `ok: examples/skills/linear-refresh/pipeline.yml`

- [ ] **Step 3: Verify all reference files are reachable from SKILL.md**

Check that every reference path in SKILL.md's Phase Map table resolves to an existing file:

Run: `for f in references/collect-tickets-agent.md references/collect-explore-agent.md references/collect-merge-agent.md references/cleanup-agent.md references/add-agent.md references/audit-agent.md references/approve-format.md references/execute-agent.md; do test -f "examples/skills/linear-refresh/$f" && echo "OK: $f" || echo "MISSING: $f"; done`

Expected: all OK

- [ ] **Step 4: Verify cross-references in agent reference files**

Check that agent reference files that reference other reference files point to existing files:

Run: `grep -roh '\[.*\](.*\.md)' examples/skills/linear-refresh/references/*-agent.md | grep -oP '\(.*?\)' | tr -d '()' | sort -u | while read f; do test -f "examples/skills/linear-refresh/references/$f" && echo "OK: $f" || echo "MISSING: $f"; done`

Expected: all OK (collected-context-schema.md, external-source-exploration.md, ground-truth-audit.md, execution-report.md)
