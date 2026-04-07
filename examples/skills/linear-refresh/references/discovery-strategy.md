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
