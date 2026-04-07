# Execution Report

Format for the `.linear-refresh/result.json` artifact produced by Step 5 (Execute).

## JSON Structure

```json
{
  "cleanup": {
    "success": 0,
    "failed": 0,
    "failures": [
      {
        "ticket_id": "ISSUE-XX",
        "action": "set parent",
        "error": "reason"
      }
    ]
  },
  "add": {
    "created": 0,
    "linked": 0,
    "failed": 0,
    "failures": [
      {
        "item": "description",
        "action": "create",
        "error": "reason"
      }
    ]
  },
  "changes": [
    {
      "ticket_id": "ISSUE-XX",
      "type": "cleanup | create | link",
      "description": "what changed"
    }
  ]
}
```

## Execution Order

1. **Cleanup** (strict order):
   1. Parent-child relationship setup.
   2. Parallel: blockedBy, relatedTo, status changes, project assignment, context additions.
   3. Duplicate merges (set Done + duplicateOf).

2. **Add** (strict order):
   1. Create new tickets.
   2. Link to existing tickets (comments/attachments).

## Error Handling

| Error | Response |
|-------|----------|
| Linear API error (individual ticket) | Skip and continue. Add to failures list. |
| Linear API rate limit | Wait and retry (max 3 attempts). |
| Ticket already deleted/archived | Skip and add to failures list. |
| Circular parent-child reference | Skip and add to failures list. |
| Cleanup failure | Does NOT block Add execution. |

## Display Format

After execution, present results to the user:

```
## Refresh Result

### Cleanup
✓ Success: N items
✗ Failed: N items
- ISSUE-XX: set parent failed (reason)

### Add
✓ Created: N tickets
✓ Linked: N tickets
✗ Failed: N items
- #XX: create failed (reason)

### Changed Tickets
| Ticket | Type | Change |
|--------|------|--------|
| ISSUE-XX | cleanup | Set parent to ISSUE-YY |
```
