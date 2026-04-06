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
