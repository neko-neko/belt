# Execute Agent

Sub-agent instructions for the `execute` phase.

## Prerequisites

1. Invoke `/linear-cli` skill for ticket manipulation commands.

## Input

- File: `.belt/refresh-plan.json`
- Reference: [execution-report.md](execution-report.md) for output format and execution order

## Output

- Write: `.belt/refresh-result.json`
- Return to orchestrator: execution result summary only

## Procedure

### Step 1: Read Plan

Read `.belt/refresh-plan.json` and `execution-report.md`.

### Step 2: Execute Cleanup Changes

Follow the strict execution order from execution-report.md:

1. **Parent-child relationships** (sequential)
2. **Parallel:** blockedBy, relatedTo, status changes, project assignment, context additions
3. **Duplicate merges** — set Done + duplicateOf (sequential)

For each change, use `linear issue update` or `linear issue comment` as appropriate.

### Step 3: Execute Add Changes

1. **Create** new tickets via `linear issue create`
2. **Link** to existing tickets via `linear issue comment` or `linear issue update`

### Step 4: Error Handling

Per execution-report.md:
- Individual failures: skip and continue, add to failures list
- Rate limits: wait and retry (max 3)
- Deleted/archived tickets: skip
- Circular parent-child: skip
- Cleanup failures do NOT block Add execution

### Step 5: Write Result

Write `.belt/refresh-result.json` per the schema in execution-report.md.

### Step 6: Return Summary

Return ONLY: "Executed: 13/14 cleanup (1 failed), 4/4 add. Written to .belt/refresh-result.json"
