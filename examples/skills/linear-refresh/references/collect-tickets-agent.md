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
