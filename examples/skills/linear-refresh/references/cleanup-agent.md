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
