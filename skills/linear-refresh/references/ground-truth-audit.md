# Ground Truth Audit

Pre-plan validation for the audit phase. Run these 3 questions for each In Progress ticket
before generating the unified plan.

## Q1: Implementation Context Audit

> "If an implementer picks up this ticket tomorrow, what are the latest specs and decisions they need to know?"

**Check:** Is this information reflected in Plan A's "context addition" section?

**If not reflected:**
- Identify which external source (from Step 0-3 / 0-3b results) contains the information.
- Add to Plan A as a context addition item.

## Q2: Recent Activity Audit

> "What is the latest event on this ticket?"

**Check:**
1. Is `latest_activity_ts` within 72 hours of refresh execution?
2. If yes, AND `deferred_signals` is non-empty: are those deferred commitments reflected in the plan?

**If not reflected:**
- Update Plan A or Plan B to include the deferred commitment.

## Q3: Untracked References Audit

> "Are there untracked URLs linked to this ticket?"

**Check:** Are there `referenced_urls` from Step 0-3 that were NOT followed in Step 0-3b
(because the ticket didn't meet the 2-hop filter conditions)?

**If untracked references remain:**
- Decide: record as "untracked reference" in the plan, OR run additional single-shot exploration.

## Remediation Loop

When any question reveals a gap that requires additional exploration:

1. Run single-shot exploration for the specific URL(s) only.
2. Append results to `.belt/collected-context.json`.
3. Re-run cleanup analysis (regenerate Plan A).
4. Re-run add analysis (regenerate Plan B).
5. Re-audit the affected tickets.

This loop is bounded by `max_retries: 2` on the audit phase. After 2 remediation
cycles, proceed with the current plan and note remaining gaps.

## Output

After audit passes (or max_retries exhausted), merge Plan A and Plan B into
`.belt/refresh-plan.json` with the following structure:

```json
{
  "summary": {
    "total_tickets": 0,
    "cleanup_changes": 0,
    "add_detections": { "create": 0, "link": 0, "skip": 0 },
    "external_sources": { "explored": 0, "skipped": 0, "failed": 0 }
  },
  "cleanup": [],
  "add": []
}
```
