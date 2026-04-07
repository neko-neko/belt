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
