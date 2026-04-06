# Approve Phase Format

Display format for the orchestrator to present the unified plan to the user.

## Input

The orchestrator reads `.belt/refresh-plan.json` directly (this is the only phase
where the orchestrator reads data files).

## Display Format

Present the plan using this structure:

### Header

```
## Linear Refresh Plan

**Team:** {team_id} ({total_tickets} tickets analyzed, {external_sources.explored} external sources explored)
```

### Cleanup Section

Group changes by category with tables:

| Category | Table Columns |
|----------|--------------|
| Parent-child (parent) | Ticket, Parent, Rationale |
| Blocking (blocking) | Ticket, blockedBy, Rationale |
| Related (related) | Ticket, relatedTo, Rationale |
| Context addition (context_addition) | ID, Ticket, Content |
| Project assignment (project) | ID, Ticket, Project, Rationale |
| Due date (due_date) | ID, Ticket, Due Date, Rationale |
| Title change (title) | ID, Ticket, Current → New |
| Duplicate (duplicate) | ID, Ticket, duplicateOf, Rationale |

Omit categories with 0 items.

### Add Section

Group items by disposition:

| Disposition | Table Columns |
|------------|--------------|
| create | ID, Title, Priority, Status |
| link | ID, Ticket, Content |
| skip | ID, Content, Reason |

### Approval Prompt

```
This plan will execute {cleanup_count} cleanup changes and {add_create + add_link} add actions.
Approve? (ok / modify / cancel)
```

## Approval Flow

- `ok` / approve → proceed to execute phase
- Modification request → update `.belt/refresh-plan.json` and re-present
- `cancel` → exit pipeline
- `--force` mode: display plan but skip waiting, proceed immediately
