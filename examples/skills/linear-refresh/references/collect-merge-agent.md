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
