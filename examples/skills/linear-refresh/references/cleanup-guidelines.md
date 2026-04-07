# Cleanup Analysis Guidelines

Analysis guidelines for detecting structural issues in existing Linear tickets.
Each guideline is a heuristic — final judgment is based on contextual understanding
of the ticket landscape, not mechanical rule application.

## Input

- `.linear-refresh/collected-context.json` — all tickets + external sources + discovery sources

## Output

- Cleanup change candidates for inclusion in `.linear-refresh/plan.json`

## Categories

### Parent-child Relationships

| Signal | Judgment |
|--------|---------|
| description mentions "parent ticket", "Epic", etc. | Target as parent candidate |
| Same functional area with Feature/Epic + Bug/Improvement combination | Feature/Epic as parent candidate |
| Multiple tickets reference same PR, one scope contains the other | Containing scope as parent candidate |
| Titles contain Phase N / Step N across multiple tickets | Infer common parent |

### Blocking Relationships

| Signal | Judgment |
|--------|---------|
| description mentions "waiting for completion of", "blocker" | blockedBy candidate |
| External source conversation agrees "after release of X" | blockedBy candidate |
| description describes competing resource in same environment | blockedBy candidate |

### Related (relatedTo)

| Signal | Judgment |
|--------|---------|
| description mentions another ticket but no relation set | relatedTo candidate |
| Multiple tickets discussed in same external source thread | relatedTo candidate |
| Cause → fix, symptom → correction causal relationship | relatedTo candidate |

### Status Inconsistency

| Pattern | Judgment |
|---------|---------|
| archived but completedAt is null | Inconsistency → Done or unarchive |
| In Progress but all referenced PRs/Issues are closed | Done candidate |
| In Progress but blockedBy dependency is unresolved | Report as Blocked |
| Not completed but external source confirms release/deployment done | Done candidate |
| Done but referenced PR is still open | Report as inconsistency |

### Duplicates

| Signal | Judgment |
|--------|---------|
| Same PR/Issue referenced AND same scope | Merge candidate |
| Same feature/bug/request described from different angles | Merge candidate (flag for user judgment) |

### Context Gaps

| Pattern | Judgment |
|---------|---------|
| description mentions external URL but not in attachments | Link addition candidate |
| External source discusses this ticket but no linkage exists | Link addition candidate |
| project unset but sibling tickets (same label) belong to a specific project | Project assignment candidate |

### Title Inaccuracy

| Signal | Judgment |
|--------|---------|
| External source discussion / actual scope diverges from ticket title | Title change candidate |
| Ticket scope evolved since creation, title no longer reflects reality | Title change candidate |

### Missing Due Date

| Signal | Judgment |
|--------|---------|
| External source records an agreed deadline but ticket has no dueDate | Due date candidate |
| description mentions a deadline but dueDate is unset | Due date candidate |
