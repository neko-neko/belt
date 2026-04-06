# Collect Agent

Sub-agent instructions for the `collect` phase. Fetches all tickets, explores external
sources (1-hop + conditional 2-hop), and writes `.belt/collected-context.json`.

## Prerequisites

1. Invoke `/linear-cli` skill to load CLI usage patterns.
2. Invoke `/slackcli` skill for Slack thread exploration.
3. Verify: `which linear && linear --version`

## Input

- `team_id` parameter from orchestrator (e.g., "RAKMY")

## Output

- Write: `.belt/collected-context.json`
- Return to orchestrator: count summary only (e.g., "63 tickets, 51 external sources")

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

### Step 3: Fetch Active Ticket Details (parallel)

Filter: status is not Done/Cancelled, OR updated within the last 30 days.

Dispatch parallel sub-agents in batches of 10 tickets. Each sub-agent:
1. Runs `linear issue show {ticket_id}` for each ticket in its batch.
2. Extracts: attachments (URL + title), description URLs (regex `https?://[^\s)]+`), relations (relatedTo, blocks, blockedBy).
3. Returns a JSON array of enriched ticket objects.

### Step 4: 1-Hop External Source Exploration (parallel)

Extract all URLs from `description_urls` and `attachments[].url` across all tickets.

Classify each URL per [external-source-exploration.md](external-source-exploration.md):

| Classification | Action |
|---------------|--------|
| Explore (full) | Slack threads, GitHub issues/PRs with discussion |
| Metadata only | Google Docs/Sheets, Notion — title + accessibility check |
| Skip | Images, screenshots, already-registered attachments |

Group URLs by referring ticket cluster. Dispatch parallel sub-agents per cluster. Each sub-agent:

1. Explores its assigned URLs using available tools (slackcli for Slack, WebFetch as fallback).
2. Applies summary budget per [external-source-exploration.md](external-source-exploration.md):
   - Backlog/Low: 200 chars
   - Todo/Medium: 400 chars
   - In Progress/Urgent/High: 800 chars + raw excerpts
3. Returns one ExternalSource object per URL (see [collected-context-schema.md](collected-context-schema.md)).

### Step 5: 2-Hop Recursive Expansion (conditional, parallel)

Extract `referenced_urls` from 1-hop results where ALL conditions are met:
- Referring ticket is **In Progress** AND **Urgent or High** priority
- `latest_activity_ts` is within **72 hours** of current time

If no URLs qualify, skip to Step 6.

Follow URL types and filtering rules in [external-source-exploration.md](external-source-exploration.md).
Dispatch parallel sub-agents. Mark results with `hop: 2`.

### Step 6: Write CollectedContext

Build the merged structure per [collected-context-schema.md](collected-context-schema.md):

```json
{
  "team_id": "RAKMY",
  "collected_at": "2026-04-07T10:00:00+09:00",
  "tickets": [ ... ],
  "external_sources": [ ... ]
}
```

Write to `.belt/collected-context.json`.

### Step 7: Return Summary

Return ONLY a summary string to the orchestrator. Do NOT return ticket data or source data.

Example: "63 tickets (35 active with details), 51 external sources (48 hop-1 + 3 hop-2), written to .belt/collected-context.json"
