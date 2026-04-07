# linear-refresh Standalone Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove belt pipeline from linear-refresh, consolidate linear-cleanup/add into a standalone SKILL.md, and add a Discover step for unlinked external source detection.

**Architecture:** Standalone SKILL.md drives a 5-step workflow (Collect → Discover → Analyze → Approve → Execute). Sub-agents handle I/O parallelization only (Steps 1-2). Reference files hold domain knowledge (analysis guidelines, sub-agent instructions, schemas). Artifacts go to `.linear-refresh/` directory.

**Tech Stack:** Claude Code skills (SKILL.md + references/), linear CLI, slackcli, gh CLI

---

## File Map

### Create

| File | Responsibility |
|------|---------------|
| `examples/skills/linear-refresh/references/cleanup-guidelines.md` | 8-category analysis guidelines (from linear-cleanup Phase 2) |
| `examples/skills/linear-refresh/references/add-guidelines.md` | Detection criteria + disposition rules (from linear-add Phase 2) |
| `examples/skills/linear-refresh/references/discovery-strategy.md` | Query seed generation, noise control, search targets |
| `examples/skills/linear-refresh/references/discover-agent.md` | Sub-agent instructions for Slack/GitHub search |

### Rewrite

| File | Change |
|------|--------|
| `examples/skills/linear-refresh/references/collect-agent.md` | 3 sub-agent types (detail fetch, 1-hop, 2-hop) |
| `examples/skills/linear-refresh/references/collected-context-schema.md` | Add `discovery_sources[]` |
| `examples/skills/linear-refresh/SKILL.md` | Complete rewrite — standalone 5-step workflow |

### Delete

| File | Reason |
|------|--------|
| `examples/skills/linear-refresh/pipeline.yml` | Belt pipeline removed |
| `examples/skills/linear-refresh/belt.toml` | Belt config removed |
| `examples/skills/linear-refresh/linear-cleanup.yml` | Sub-pipeline removed |
| `examples/skills/linear-refresh/linear-add.yml` | Sub-pipeline removed |
| `examples/skills/linear-refresh/references/cleanup-agent.md` | Replaced by cleanup-guidelines.md |
| `examples/skills/linear-refresh/references/add-agent.md` | Replaced by add-guidelines.md |
| `examples/skills/linear-refresh/references/audit-agent.md` | Belt audit removed |
| `examples/skills/linear-refresh/references/approve-format.md` | Absorbed into SKILL.md |
| `examples/skills/linear-refresh/references/ground-truth-audit.md` | Replaced by self-check |
| `examples/skills/linear-refresh/references/execute-agent.md` | Step 5 runs in main agent, no sub-agent needed |
| `examples/skills/linear-cleanup/SKILL.md` | Consolidated into linear-refresh |
| `examples/skills/linear-add/SKILL.md` | Consolidated into linear-refresh |

### Unchanged

| File | Reason |
|------|--------|
| `examples/skills/linear-refresh/references/external-source-exploration.md` | Linked exploration rules unchanged |
| `examples/skills/linear-refresh/references/execution-report.md` | Execution result format unchanged |

---

### Task 1: Create cleanup-guidelines.md

**Files:**
- Create: `examples/skills/linear-refresh/references/cleanup-guidelines.md`
- Source: `examples/skills/linear-cleanup/SKILL.md` (lines 76-198, Phase 2 analysis guidelines)

- [ ] **Step 1: Create cleanup-guidelines.md**

Extract the 8-category analysis guidelines from linear-cleanup SKILL.md Phase 2 and write as a standalone reference file. Remove Phase 1/3/4 execution details — those are now in the main SKILL.md.

```markdown
# Cleanup Analysis Guidelines

Analysis guidelines for detecting structural issues in existing Linear tickets.
Each guideline is a heuristic — final judgment is based on contextual understanding
of the ticket landscape, not mechanical rule application.

## Input

- `.linear-refresh/collected-context.json` — all tickets + external sources + discovery sources

## Output

- Cleanup change candidates for inclusion in `.linear-refresh/plan.json`

## Categories

### Parent-child Relationships

| Signal | Judgment |
|--------|---------|
| description mentions "parent ticket", "Epic", etc. | Target as parent candidate |
| Same functional area with Feature/Epic + Bug/Improvement combination | Feature/Epic as parent candidate |
| Multiple tickets reference same PR, one scope contains the other | Containing scope as parent candidate |
| Titles contain Phase N / Step N across multiple tickets | Infer common parent |

### Blocking Relationships

| Signal | Judgment |
|--------|---------|
| description mentions "waiting for completion of", "blocker" | blockedBy candidate |
| External source conversation agrees "after release of X" | blockedBy candidate |
| description describes competing resource in same environment | blockedBy candidate |

### Related (relatedTo)

| Signal | Judgment |
|--------|---------|
| description mentions another ticket but no relation set | relatedTo candidate |
| Multiple tickets discussed in same external source thread | relatedTo candidate |
| Cause → fix, symptom → correction causal relationship | relatedTo candidate |

### Status Inconsistency

| Pattern | Judgment |
|---------|---------|
| archived but completedAt is null | Inconsistency → Done or unarchive |
| In Progress but all referenced PRs/Issues are closed | Done candidate |
| In Progress but blockedBy dependency is unresolved | Report as Blocked |
| Not completed but external source confirms release/deployment done | Done candidate |
| Done but referenced PR is still open | Report as inconsistency |

### Duplicates

| Signal | Judgment |
|--------|---------|
| Same PR/Issue referenced AND same scope | Merge candidate |
| Same feature/bug/request described from different angles | Merge candidate (flag for user judgment) |

### Context Gaps

| Pattern | Judgment |
|---------|---------|
| description mentions external URL but not in attachments | Link addition candidate |
| External source discusses this ticket but no linkage exists | Link addition candidate |
| project unset but sibling tickets (same label) belong to a specific project | Project assignment candidate |

### Title Inaccuracy

| Signal | Judgment |
|--------|---------|
| External source discussion / actual scope diverges from ticket title | Title change candidate |
| Ticket scope evolved since creation, title no longer reflects reality | Title change candidate |

### Missing Due Date

| Signal | Judgment |
|--------|---------|
| External source records an agreed deadline but ticket has no dueDate | Due date candidate |
| description mentions a deadline but dueDate is unset | Due date candidate |
```

- [ ] **Step 2: Verify file was created**

Run: `test -f examples/skills/linear-refresh/references/cleanup-guidelines.md && echo "OK" || echo "MISSING"`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/cleanup-guidelines.md
git commit -m "docs(linear-refresh): add cleanup-guidelines.md reference"
```

---

### Task 2: Create add-guidelines.md

**Files:**
- Create: `examples/skills/linear-refresh/references/add-guidelines.md`
- Source: `examples/skills/linear-add/SKILL.md` (lines 76-159, Phase 2-3 detection criteria + disposition)

- [ ] **Step 1: Create add-guidelines.md**

Extract detection criteria and disposition rules from linear-add SKILL.md Phase 2-3.

```markdown
# Add Detection Guidelines

Detection criteria for identifying items worth attention — new ticket creation,
linking to existing tickets, or explicit skip. Each guideline is a heuristic —
final judgment is based on contextual understanding, not mechanical rule application.

## Input

- `.linear-refresh/collected-context.json` — all tickets + external sources + discovery sources
- Cleanup results from the current analysis pass (for deduplication)

## Output

- Add item candidates for inclusion in `.linear-refresh/plan.json`

## Detection Criteria

The core question: "Is there an unresolved obligation, or an opportunity for intervention?"

### Explicit Obligations

Judgment axis: "Is the ball in our court?"

| Signal | Judgment |
|--------|---------|
| External source records agreed work but no corresponding Linear ticket exists | New ticket candidate |
| External source requests a response/action that remains unanswered | New ticket candidate |
| External source records a clear next step that is untracked | New ticket candidate |
| Ticket description/comments mention "next sprint", "follow-up" work | Follow-up ticket candidate |

### Intervention Opportunities

Not our ball, but intervening could add value.

| Signal | Judgment |
|--------|---------|
| External source discussion has stalled or become circular — a proposal could unblock it | Detection target (judge via disposition) |
| Conversation continues without clear direction | Detection target (judge via disposition) |
| Discussion deadlocked without decision-maker present | Detection target (judge via disposition) |

### Structural Gaps

| Signal | Judgment |
|--------|---------|
| Parent ticket description contains work decomposition but child tickets are missing | Child ticket candidate |
| "Waiting for X" / "after X completes" but X has no Linear ticket | Blocker ticket candidate |

## Exclusion Criteria

Do NOT detect:
- Conversations that are answered and concluded
- Topics under discussion where the ball is not in our court
- Work that is already completed
- Work covered by an existing ticket's scope (duplicate)

## Disposition

Each detected item receives one of:

| Disposition | Meaning | Execution action |
|-------------|---------|-----------------|
| `create` | New ticket needed | Create ticket with title, description, priority, labels, parent, links, dueDate |
| `link` | Add context to existing ticket | Add comment + relation/attachment to existing ticket |
| `skip` | Not actionable or premature | No action. Record reason in plan. |

### Boundary with Cleanup "Context Addition"

- **Cleanup context addition**: A URL already mentioned in ticket description/comments is missing from attachments → register it
- **Add link**: A newly discovered external discussion is relevant to an existing ticket → add comment + link

Trigger differs (existing reference vs new discovery), but operation is similar. When running in the same analysis pass, deduplication against cleanup results prevents overlap.
```

- [ ] **Step 2: Verify file was created**

Run: `test -f examples/skills/linear-refresh/references/add-guidelines.md && echo "OK" || echo "MISSING"`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/add-guidelines.md
git commit -m "docs(linear-refresh): add add-guidelines.md reference"
```

---

### Task 3: Create discovery-strategy.md

**Files:**
- Create: `examples/skills/linear-refresh/references/discovery-strategy.md`

- [ ] **Step 1: Create discovery-strategy.md**

```markdown
# Discovery Strategy

Rules for generating search queries and controlling noise when discovering
external sources not linked from any Linear ticket.

## Query Seed Generation

Extract seeds from `.linear-refresh/collected-context.json` tickets:

| Seed type | Source | Example |
|-----------|--------|---------|
| Ticket ID | tickets[].id | `RAKMY-98`, `RAKMY-104` |
| Title keywords | tickets[].title (extracted) | `月初ドロップダウン`, `RDS スケーリング` |
| Project name | tickets[].project (deduplicated) | `rakmy` |
| Label names | tickets[].labels (deduplicated) | `bug`, `infrastructure` |
| Assignee names | tickets[].assignee (deduplicated) | — |

### Priority Weighting

| Ticket priority/status | Seeds extracted |
|----------------------|----------------|
| In Progress + Urgent/High | Full: ID + keywords + project + labels |
| In Progress + Medium | ID + keywords |
| Todo / Backlog / Low | ID only |
| Done / Cancelled | Skip (no seeds) |

Rationale: High-priority active tickets have the most value from discovery.
Low-priority tickets contribute IDs only to prevent query explosion.

### Keyword Extraction from Titles

- Split title into meaningful phrases (not individual words)
- Exclude common stop words and generic terms
- Keep domain-specific terms, proper nouns, technical identifiers
- Example: "CS画面 発注先企業ユーザー不在エラー" → `"CS画面 発注先企業"`, `"ユーザー不在エラー"`

## Search Targets

### Slack

Method: `/slackcli` search command

Query construction:
- One search per seed (not combined — Slack search is OR-based within a query)
- Ticket IDs as exact match: `RAKMY-98`
- Keywords as phrase match when multi-word: `"RDS スケーリング"`
- Project/label names as simple terms: `rakmy`

### GitHub

Method: `gh search issues` / `gh search prs`

Query construction:
- Scope to the project repository: `repo:owner/repo`
- Ticket IDs: `repo:rakmy/rakmy_server RAKMY-98`
- Keywords: `repo:rakmy/rakmy_server "RDS scaling"`

### Not Searched

- Google Docs / Notion — no practical full-text search API
- Images, screenshots — not searchable
- Archived Slack channels — may produce stale results

## Noise Control

### Time Window

Search results are limited to the **last 30 days**. Older results are unlikely to represent active obligations.

### Deduplication

Before adding to `discovery_sources[]`:
1. Check URL against `external_sources[].url` from Step 1 — skip if already collected
2. Check URL against other `discovery_sources[].url` — skip if already discovered by another query
3. For Slack: normalize thread URLs (parent message URL = canonical form)

### Relevance Judgment

Each sub-agent judges whether a search result is related to the ticket context:
- **Related**: Content discusses the same feature, bug, system, or stakeholder
- **Unrelated**: Content happens to match a keyword but discusses a different topic
- Discard unrelated results. Do not include them in discovery_sources.

### Result Cap

- Max **30 results per source type** (30 Slack + 30 GitHub)
- On overflow: prioritize by referring ticket priority (Urgent > High > Medium), then by recency
- This cap applies after deduplication and relevance filtering

## Sub-Agent Prompt Structure

> Given the following ticket context, search for related discussions.
>
> **Team:** {team_id}
> **Seeds:** {seed_list}
> **Source:** Slack | GitHub
> **Time window:** last 30 days
> **Exclude URLs:** {already_collected_urls}
>
> For each result, return: url, discovery_query, related_tickets, accessible,
> summary (budget per external-source-exploration.md), latest_activity_ts,
> deferred_signals, source_type
```

- [ ] **Step 2: Verify file was created**

Run: `test -f examples/skills/linear-refresh/references/discovery-strategy.md && echo "OK" || echo "MISSING"`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/discovery-strategy.md
git commit -m "docs(linear-refresh): add discovery-strategy.md reference"
```

---

### Task 4: Create discover-agent.md

**Files:**
- Create: `examples/skills/linear-refresh/references/discover-agent.md`

- [ ] **Step 1: Create discover-agent.md**

```markdown
# Discover Agent

Sub-agent instructions for Step 2 (Discover). Searches Slack and GitHub for
external sources not linked from any Linear ticket.

## Sub-Agent Types

Two types of sub-agents are dispatched in parallel by the main agent.

---

### Slack Search Agent

#### Prerequisites

1. Invoke `/slackcli` skill to load Slack CLI usage patterns.

#### Input

- `seeds`: List of search queries (ticket IDs, keywords, project names)
- `exclude_urls`: URLs already collected in Step 1 (for deduplication)
- `time_window`: "last 30 days"
- `ticket_context`: Summary of related tickets (for relevance judgment)

#### Procedure

1. For each seed, run Slack search via slackcli.
2. For each result:
   a. Check if URL is in `exclude_urls` → skip if yes.
   b. Judge relevance: does this message/thread relate to the ticket context?
   c. If related: explore the full thread, extract summary + deferred signals.
   d. Apply summary budget per [external-source-exploration.md](external-source-exploration.md).
3. Deduplicate results (normalize Slack thread URLs to parent message form).
4. If results exceed 30, prioritize by ticket priority + recency.

#### Return Format

Return a JSON array of DiscoverySource objects:

```json
[
  {
    "url": "https://workspace.slack.com/archives/CHAN/p1234567890",
    "discovery_query": "RAKMY-98",
    "related_tickets": ["RAKMY-98", "RAKMY-97"],
    "accessible": true,
    "summary": "Thread discusses month-start dropdown implementation...",
    "latest_activity_ts": "2026-04-05T14:30:00+09:00",
    "deferred_signals": ["will follow up after deploy"],
    "source_type": "slack_thread"
  }
]
```

---

### GitHub Search Agent

#### Prerequisites

None (gh CLI is available by default).

#### Input

- `seeds`: List of search queries
- `repo`: Target repository (e.g., "rakmy/rakmy_server")
- `exclude_urls`: URLs already collected in Step 1
- `ticket_context`: Summary of related tickets

#### Procedure

1. For each seed, run `gh search issues` and `gh search prs` scoped to `repo`.
2. For each result:
   a. Check if URL is in `exclude_urls` → skip if yes.
   b. Judge relevance: does this issue/PR relate to the ticket context?
   c. If related: fetch comments/discussion, extract summary + deferred signals.
   d. Apply summary budget per [external-source-exploration.md](external-source-exploration.md).
3. Deduplicate (same issue via different queries).
4. If results exceed 30, prioritize by ticket priority + recency.

#### Return Format

Same JSON array of DiscoverySource objects as Slack, with `source_type` as
`"github_issue"` or `"github_pr"`.
```

- [ ] **Step 2: Verify file was created**

Run: `test -f examples/skills/linear-refresh/references/discover-agent.md && echo "OK" || echo "MISSING"`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/discover-agent.md
git commit -m "docs(linear-refresh): add discover-agent.md reference"
```

---

### Task 5: Rewrite collect-agent.md

**Files:**
- Modify: `examples/skills/linear-refresh/references/collect-agent.md`

- [ ] **Step 1: Rewrite collect-agent.md**

Replace the entire file. The old version was a single monolithic agent instruction. The new version defines 3 sub-agent types (detail fetch, 1-hop exploration, 2-hop exploration) that the main agent dispatches.

```markdown
# Collect Agent

Sub-agent instructions for Step 1 (Collect). Three types of I/O sub-agents
are dispatched in parallel by the main agent.

## Sub-Agent Types

---

### Detail Fetch Agent

Fetches ticket details in batches of 10.

#### Prerequisites

1. Invoke `/linear-cli` skill to load CLI usage patterns.

#### Input

- `ticket_ids`: List of ticket IDs to fetch (max 10 per batch)

#### Procedure

1. For each ticket ID, run `linear issue show {ticket_id}`.
2. Extract from each result:
   - `attachments`: `[{ "url": "string", "title": "string" }]`
   - `description_urls`: URLs extracted via regex `https?://[^\s)]+`
   - `relations`: `{ "relatedTo": [], "blocks": [], "blockedBy": [] }`
3. Return enriched ticket objects.

#### Return Format

```json
[
  {
    "id": "RAKMY-81",
    "attachments": [{ "url": "https://...", "title": "..." }],
    "description_urls": ["https://github.com/...", "https://admin.rakmy.jp/..."],
    "relations": { "relatedTo": ["RAKMY-82"], "blocks": [], "blockedBy": [] }
  }
]
```

---

### 1-Hop Exploration Agent

Explores external URLs linked from ticket descriptions and attachments.

#### Prerequisites

1. Invoke `/slackcli` skill for Slack thread exploration.

#### Input

- `urls`: List of URLs to explore (pre-filtered by main agent)
- `ticket_context`: For each URL, the referring ticket's ID, title, and priority
- `classification`: Per-URL classification (explore / metadata-only) from main agent

#### Procedure

1. For each URL classified as "explore":
   a. Use `/slackcli` for Slack URLs, WebFetch for others.
   b. Summarize content within budget:
      - Backlog/Low priority ticket: 200 chars
      - Todo/Medium: 400 chars
      - In Progress/Urgent/High: 800 chars + raw excerpts
   c. Extract `referenced_urls` (URLs mentioned in the fetched content).
   d. Detect `deferred_signals` per [external-source-exploration.md](external-source-exploration.md).
2. For each URL classified as "metadata-only":
   a. Check title + accessibility only.
3. Return ExternalSource objects.

#### Return Format

```json
[
  {
    "url": "https://rakmy.slack.com/archives/CHAN/p123",
    "ticket_id": "RAKMY-81",
    "hop": 1,
    "accessible": true,
    "summary": "Thread discusses...",
    "referenced_urls": ["https://github.com/..."],
    "latest_activity_ts": "2026-04-05T10:00:00+09:00",
    "deferred_signals": ["will update after testing"],
    "source_type": "slack_thread"
  }
]
```

---

### 2-Hop Exploration Agent

Expands `referenced_urls` from 1-hop results under strict conditions.

#### Prerequisites

1. Invoke `/slackcli` skill for Slack thread exploration.

#### Input

- `urls`: List of URLs to explore (pre-filtered by main agent per 2-hop criteria)
- `ticket_context`: Referring ticket info from the 1-hop source

#### 2-Hop Criteria (applied by main agent before dispatch)

ALL conditions must be met for a URL to qualify:
- Referring ticket is **In Progress** AND **Urgent or High** priority
- `latest_activity_ts` from 1-hop is within **72 hours** of current time

URL types to follow: GitHub PR/Issue comments, cross-references, Slack threads, dev URLs.
URL types to skip: Already explored in 1-hop, static documents, images, 3rd+ hop URLs.

#### Procedure

Same as 1-hop exploration agent, but mark all results with `hop: 2`.

#### Return Format

Same as 1-hop, with `"hop": 2`.
```

- [ ] **Step 2: Verify rewrite**

Run: `grep -c "Sub-Agent Types" examples/skills/linear-refresh/references/collect-agent.md`
Expected: `1`

Run: `grep -c "Detail Fetch Agent" examples/skills/linear-refresh/references/collect-agent.md`
Expected: `1`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/collect-agent.md
git commit -m "docs(linear-refresh): rewrite collect-agent.md for 3 sub-agent types"
```

---

### Task 6: Update collected-context-schema.md

**Files:**
- Modify: `examples/skills/linear-refresh/references/collected-context-schema.md`

- [ ] **Step 1: Rewrite collected-context-schema.md**

Add `discovery_sources[]` field and update the top-level structure.

```markdown
# CollectedContext Schema

JSON schema for the `.linear-refresh/collected-context.json` artifact produced
by Step 1 (Collect) and extended by Step 2 (Discover).

## Structure

```json
{
  "team_id": "string",
  "tickets": [],
  "external_sources": [],
  "discovery_sources": []
}
```

## tickets[]

Each ticket entry contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Linear issue identifier (e.g., "RAKMY-20") |
| `title` | string | Issue title |
| `status` | string | Current status (Backlog, Todo, In Progress, Done, Cancelled) |
| `priority` | string | Priority level (Urgent, High, Medium, Low, No priority) |
| `labels` | string[] | Label names |
| `project` | string \| null | Project name if assigned |
| `parentId` | string \| null | Parent issue ID |
| `assignee` | string \| null | Assignee name |
| `completedAt` | string \| null | ISO 8601 completion timestamp |
| `archivedAt` | string \| null | ISO 8601 archive timestamp |
| `attachments` | object[] | `[{ "url": "string", "title": "string" }]` |
| `description_urls` | string[] | URLs extracted from description via `https?://[^\s)]+` |
| `relations` | object | `{ "relatedTo": [], "blocks": [], "blockedBy": [] }` |

## external_sources[]

Sources discovered via ticket URLs (Step 1). Each entry:

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Explored URL |
| `ticket_id` | string | Referring ticket ID |
| `hop` | number | 1 = from ticket URL, 2 = from referenced_urls in hop 1 |
| `accessible` | boolean | Whether the URL was successfully fetched |
| `summary` | string | Content summary (budget: 200/400/800 chars by priority) |
| `referenced_urls` | string[] | URLs mentioned in the fetched content |
| `latest_activity_ts` | string | ISO 8601 timestamp of last activity |
| `deferred_signals` | string[] | Detected deferred commitment patterns |
| `source_type` | string | One of: slack_thread, github_issue, github_pr, github_comment, document |

## discovery_sources[]

Sources discovered via keyword search / ticket reverse-lookup (Step 2).
Separate from external_sources because provenance and confidence differ.

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Discovered URL |
| `discovery_query` | string | The search query that found this source |
| `related_tickets` | string[] | Ticket IDs this source is judged to relate to |
| `accessible` | boolean | Whether the URL was successfully fetched |
| `summary` | string | Content summary (budget per external-source-exploration.md) |
| `latest_activity_ts` | string | ISO 8601 timestamp of last activity |
| `deferred_signals` | string[] | Detected deferred commitment patterns |
| `source_type` | string | One of: slack_message, slack_thread, github_issue, github_pr |
```

- [ ] **Step 2: Verify discovery_sources field exists**

Run: `grep -c "discovery_sources" examples/skills/linear-refresh/references/collected-context-schema.md`
Expected: at least `4` (header, structure, section title, table)

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/references/collected-context-schema.md
git commit -m "docs(linear-refresh): add discovery_sources to collected-context-schema"
```

---

### Task 7: Rewrite SKILL.md

**Files:**
- Modify: `examples/skills/linear-refresh/SKILL.md`

- [ ] **Step 1: Rewrite SKILL.md**

Complete rewrite — standalone 5-step workflow, no belt pipeline references.

```markdown
---
name: linear-refresh
description: >-
  Linearチームのチケット棚卸し・構造整理・新規検出を一気通貫で実行するスキル。
  チケットに紐付いた外部リンクの探索に加え、キーワード検索とチケット逆引きで
  未紐付きの外部ソースも発見する。
argument-hint: "[--force] [--skip-discovery] [--cleanup-only] [--add-only]"
---

# Linear Refresh

Linearチームのチケット棚卸し・構造整理・新規検出を一気通貫で実行する。

## Options

| Option | Effect |
|--------|--------|
| `--force` | Skip Step 4 (Approve) — display plan but proceed immediately |
| `--skip-discovery` | Skip Step 2 (Discover) — linked sources only |
| `--cleanup-only` | Skip add analysis in Step 3 |
| `--add-only` | Skip cleanup analysis in Step 3 |

## Prerequisites

- `linear` CLI available: `which linear && linear --version`
- `/slackcli` skill available (for Slack exploration)
- Team selection: `linear team list` → 1 team: auto-select / multiple: ask user / 0: error

## Workflow

```
Step 1: Collect    — Fetch tickets + explore linked URLs
Step 2: Discover   — Keyword search + ticket reverse-lookup (skipped with --skip-discovery)
Step 3: Analyze    — Cleanup + add analysis in a single pass
Step 4: Approve    — Present unified plan for user approval (skipped with --force)
Step 5: Execute    — Apply changes via Linear API
```

**Announce at start:** 「Linear Refresh を開始します。Step 1: Collect」

## Step 1: Collect

Fetch all tickets and explore external sources linked from ticket descriptions/attachments.

1. Invoke `/linear-cli` and `/slackcli` skills.
2. Fetch ticket list: `linear issue list --team {id} --sort priority --all-states --all-assignees --limit 0 --no-pager`
3. Dispatch **parallel sub-agents** for active ticket detail fetch (10-ticket batches).
   → Sub-agent instructions: [collect-agent.md](references/collect-agent.md) "Detail Fetch Agent"
4. Extract URLs from results. Classify per [external-source-exploration.md](references/external-source-exploration.md).
5. Dispatch **parallel sub-agents** for 1-hop URL exploration (per ticket cluster).
   → Sub-agent instructions: [collect-agent.md](references/collect-agent.md) "1-Hop Exploration Agent"
6. Evaluate 2-hop criteria: In Progress + Urgent/High + activity within 72h.
7. If qualifying URLs exist, dispatch **parallel sub-agents** for 2-hop exploration.
   → Sub-agent instructions: [collect-agent.md](references/collect-agent.md) "2-Hop Exploration Agent"
8. Merge all results into `.linear-refresh/collected-context.json` per [collected-context-schema.md](references/collected-context-schema.md).

## Step 2: Discover

Find external sources not linked from any ticket via keyword search and ticket reverse-lookup.

**Skipped when `--skip-discovery` is set.** Write empty `discovery_sources: []` and proceed.

1. Generate query seeds from collected-context.json per [discovery-strategy.md](references/discovery-strategy.md).
2. Dispatch **parallel sub-agents** for Slack search and GitHub search.
   → Sub-agent instructions: [discover-agent.md](references/discover-agent.md)
3. Filter results: deduplicate against Step 1 sources, discard unrelated.
4. Append to `.linear-refresh/collected-context.json` as `discovery_sources[]`.

## Step 3: Analyze

Run cleanup and add analysis in a single pass. Read collected-context.json once.

1. Read [cleanup-guidelines.md](references/cleanup-guidelines.md). Detect change candidates across 8 categories.
   **Skipped when `--add-only` is set.**
2. Read [add-guidelines.md](references/add-guidelines.md). Detect items with `create`/`link`/`skip` disposition.
   Reference cleanup results for deduplication.
   **Skipped when `--cleanup-only` is set.**
3. For `discovery_sources` items: apply same analysis but tag rationale with `[discovered]`.
4. **Self-check:**
   - Every In Progress ticket was analyzed at least once.
   - Deferred signals from discovery_sources are reflected in the plan.
   - If external_sources + discovery_sources are both 0 but tickets have URLs, report anomaly to user.
5. Write `.linear-refresh/plan.json`.

## Step 4: Approve

Present the unified plan for user approval.

**Skipped when `--force` is set** (plan is displayed but approval is not awaited).

Display format:

```
## Linear Refresh Plan

**Team:** {team_id} ({total_tickets} tickets, {external_sources} linked, {discovery_sources} discovered)

### Cleanup ({N} items)
(Group by category: parent-child, related, blocking, status, project, context, title, due date, duplicates. Omit empty categories.)

### Add ({N} items)
(Group by disposition: create, link, skip.)

---
Approve? (ok / modify / cancel)
```

- `ok` → Step 5
- ID-based modification → update plan.json, re-present
- `cancel` → exit

## Step 5: Execute

Apply the approved plan via Linear API.

1. **Cleanup** (strict order):
   a. Parent-child relationships (sequential)
   b. Parallel: blockedBy, relatedTo, status, project, context, title, due dates
   c. Duplicate merges — Done + duplicateOf (sequential, last)
2. **Add** (strict order):
   a. Create new tickets
   b. Link to existing tickets (comments + relations)
3. Error handling: skip individual failures, retry rate limits (max 3), continue on cleanup failure.
   → Result format: [execution-report.md](references/execution-report.md)
4. Write `.linear-refresh/result.json`. Display result summary.

## Red Flags

**Never:**
- Execute changes without an approved plan (unless `--force`)
- Delete or archive tickets (cleanup closes duplicates only)
- Rewrite ticket descriptions (context additions use comments/attachments)
- Explore beyond 2 hops in Step 1 (infinite expansion prevention)
- Search beyond 30 days in Step 2

**Always:**
- Invoke `/linear-cli` and `/slackcli` before Step 1
- Respect summary budgets (200/400/800 chars by priority)
- Record deferred signals from external sources
- Tag `[discovered]` on rationale for discovery-sourced items
- Report all execution failures in result JSON

## Artifacts

| File | Written by | Purpose |
|------|-----------|---------|
| `.linear-refresh/collected-context.json` | Step 1 + Step 2 | All tickets, linked sources, discovered sources |
| `.linear-refresh/plan.json` | Step 3 | Unified cleanup + add plan |
| `.linear-refresh/result.json` | Step 5 | Execution results |
```

- [ ] **Step 2: Verify SKILL.md structure**

Run: `grep -c "^## Step" examples/skills/linear-refresh/SKILL.md`
Expected: `5`

Run: `grep "^name:" examples/skills/linear-refresh/SKILL.md`
Expected: `name: linear-refresh`

Run: `grep -c "belt-agent\|pipeline\.yml\|belt\.toml" examples/skills/linear-refresh/SKILL.md`
Expected: `0` (no belt references)

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-refresh/SKILL.md
git commit -m "refactor(linear-refresh): rewrite as standalone SKILL.md

Remove belt pipeline, consolidate cleanup/add, add Discover step."
```

---

### Task 8: Delete old files

**Files:**
- Delete: 11 files across 3 directories

- [ ] **Step 1: Delete belt pipeline files**

```bash
rm examples/skills/linear-refresh/pipeline.yml
rm examples/skills/linear-refresh/belt.toml
rm examples/skills/linear-refresh/linear-cleanup.yml
rm examples/skills/linear-refresh/linear-add.yml
```

- [ ] **Step 2: Delete replaced reference files**

```bash
rm examples/skills/linear-refresh/references/cleanup-agent.md
rm examples/skills/linear-refresh/references/add-agent.md
rm examples/skills/linear-refresh/references/audit-agent.md
rm examples/skills/linear-refresh/references/approve-format.md
rm examples/skills/linear-refresh/references/ground-truth-audit.md
rm examples/skills/linear-refresh/references/execute-agent.md
```

- [ ] **Step 3: Delete standalone skill directories**

```bash
rm -r examples/skills/linear-cleanup/
rm -r examples/skills/linear-add/
```

- [ ] **Step 4: Verify deletions**

Run: `ls examples/skills/linear-refresh/`
Expected:
```
SKILL.md
references/
```

Run: `ls examples/skills/linear-refresh/references/`
Expected (8 files):
```
add-guidelines.md
cleanup-guidelines.md
collect-agent.md
collected-context-schema.md
discover-agent.md
discovery-strategy.md
execution-report.md
external-source-exploration.md
```

Run: `test -d examples/skills/linear-cleanup && echo "EXISTS" || echo "DELETED"`
Expected: `DELETED`

Run: `test -d examples/skills/linear-add && echo "EXISTS" || echo "DELETED"`
Expected: `DELETED`

- [ ] **Step 5: Commit**

```bash
git add -A examples/skills/linear-refresh/ examples/skills/linear-cleanup/ examples/skills/linear-add/
git commit -m "refactor(linear-refresh): delete belt pipeline and standalone cleanup/add

Remove pipeline.yml, belt.toml, sub-pipelines, replaced references,
and standalone linear-cleanup/linear-add skill directories."
```

---

### Task 9: Final Validation

- [ ] **Step 1: Verify file structure**

Run: `find examples/skills/linear-refresh -type f | sort`

Expected output (9 files: 1 SKILL.md + 8 references):
```
examples/skills/linear-refresh/SKILL.md
examples/skills/linear-refresh/references/add-guidelines.md
examples/skills/linear-refresh/references/cleanup-guidelines.md
examples/skills/linear-refresh/references/collect-agent.md
examples/skills/linear-refresh/references/collected-context-schema.md
examples/skills/linear-refresh/references/discover-agent.md
examples/skills/linear-refresh/references/discovery-strategy.md
examples/skills/linear-refresh/references/execution-report.md
examples/skills/linear-refresh/references/external-source-exploration.md
```

- [ ] **Step 2: Verify no belt references remain**

Run: `grep -r "belt-agent\|pipeline\.yml\|belt\.toml\|\.belt/" examples/skills/linear-refresh/`
Expected: no output (0 matches)

- [ ] **Step 3: Verify all reference file links in SKILL.md resolve**

Run: `grep -oP '\[.*?\]\(references/\K[^)]+' examples/skills/linear-refresh/SKILL.md | while read f; do test -f "examples/skills/linear-refresh/references/$f" && echo "OK: $f" || echo "BROKEN: $f"; done`

Expected: all OK, no BROKEN

- [ ] **Step 4: Verify no orphan references (referenced by nothing)**

Run: `for f in examples/skills/linear-refresh/references/*.md; do name=$(basename "$f"); grep -q "$name" examples/skills/linear-refresh/SKILL.md examples/skills/linear-refresh/references/*.md && echo "USED: $name" || echo "ORPHAN: $name"; done`

Expected: all USED, no ORPHAN

- [ ] **Step 5: Verify standalone skill directories are gone**

Run: `ls examples/skills/ | sort`
Expected: `linear-refresh` only (no `linear-cleanup`, no `linear-add`)
Note: other skill directories (feature-dev, smoke-test, etc.) may also appear — just verify cleanup/add are absent.
