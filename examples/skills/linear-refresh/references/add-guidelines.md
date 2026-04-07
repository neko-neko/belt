# Add Detection Guidelines

Detection criteria for identifying items worth attention — new ticket creation,
linking to existing tickets, or explicit skip. Each guideline is a heuristic —
final judgment is based on contextual understanding, not mechanical rule application.

## Input

- `.linear-refresh/collected-context.json` — all tickets + external sources + discovery sources
- Cleanup results from the current analysis pass (for deduplication)

## Output

- Add item candidates for inclusion in `.linear-refresh/plan.json`

## Detection Criteria

The core question: "Is there an unresolved obligation, or an opportunity for intervention?"

### Explicit Obligations

Judgment axis: "Is the ball in our court?"

| Signal | Judgment |
|--------|---------|
| External source records agreed work but no corresponding Linear ticket exists | New ticket candidate |
| External source requests a response/action that remains unanswered | New ticket candidate |
| External source records a clear next step that is untracked | New ticket candidate |
| Ticket description/comments mention "next sprint", "follow-up" work | Follow-up ticket candidate |

### Intervention Opportunities

Not our ball, but intervening could add value.

| Signal | Judgment |
|--------|---------|
| External source discussion has stalled or become circular — a proposal could unblock it | Detection target (judge via disposition) |
| Conversation continues without clear direction | Detection target (judge via disposition) |
| Discussion deadlocked without decision-maker present | Detection target (judge via disposition) |

### Structural Gaps

| Signal | Judgment |
|--------|---------|
| Parent ticket description contains work decomposition but child tickets are missing | Child ticket candidate |
| "Waiting for X" / "after X completes" but X has no Linear ticket | Blocker ticket candidate |

## Exclusion Criteria

Do NOT detect:
- Conversations that are answered and concluded
- Topics under discussion where the ball is not in our court
- Work that is already completed
- Work covered by an existing ticket's scope (duplicate)

## Disposition

Each detected item receives one of:

| Disposition | Meaning | Execution action |
|-------------|---------|-----------------|
| `create` | New ticket needed | Create ticket with title, description, priority, labels, parent, links, dueDate |
| `link` | Add context to existing ticket | Add comment + relation/attachment to existing ticket |
| `skip` | Not actionable or premature | No action. Record reason in plan. |

### Boundary with Cleanup "Context Addition"

- **Cleanup context addition**: A URL already mentioned in ticket description/comments is missing from attachments → register it
- **Add link**: A newly discovered external discussion is relevant to an existing ticket → add comment + link

Trigger differs (existing reference vs new discovery), but operation is similar. When running in the same analysis pass, deduplication against cleanup results prevents overlap.
