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
