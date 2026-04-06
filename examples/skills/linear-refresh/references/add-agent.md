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
