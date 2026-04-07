# Discover Agent

Sub-agent instructions for Step 2 (Discover). Searches Slack and GitHub for
external sources not linked from any Linear ticket.

## Sub-Agent Types

Two types of sub-agents are dispatched in parallel by the main agent.

---

### Slack Search Agent

#### Prerequisites

1. Invoke `/slackcli` skill to load Slack CLI usage patterns.

#### Input

- `seeds`: List of search queries (ticket IDs, keywords, project names)
- `exclude_urls`: URLs already collected in Step 1 (for deduplication)
- `time_window`: "last 30 days"
- `ticket_context`: Summary of related tickets (for relevance judgment)

#### Procedure

1. For each seed, run Slack search via slackcli.
2. For each result:
   a. Check if URL is in `exclude_urls` → skip if yes.
   b. Judge relevance: does this message/thread relate to the ticket context?
   c. If related: explore the full thread, extract summary + deferred signals.
   d. Apply summary budget per [external-source-exploration.md](external-source-exploration.md).
3. Deduplicate results (normalize Slack thread URLs to parent message form).
4. If results exceed 30, prioritize by ticket priority + recency.

#### Return Format

Return a JSON array of DiscoverySource objects:

```json
[
  {
    "url": "https://workspace.slack.com/archives/CHAN/p1234567890",
    "discovery_query": "RAKMY-98",
    "related_tickets": ["RAKMY-98", "RAKMY-97"],
    "accessible": true,
    "summary": "Thread discusses month-start dropdown implementation...",
    "latest_activity_ts": "2026-04-05T14:30:00+09:00",
    "deferred_signals": ["will follow up after deploy"],
    "source_type": "slack_thread"
  }
]
```

---

### GitHub Search Agent

#### Prerequisites

None (gh CLI is available by default).

#### Input

- `seeds`: List of search queries
- `repo`: Target repository (e.g., "rakmy/rakmy_server")
- `exclude_urls`: URLs already collected in Step 1
- `ticket_context`: Summary of related tickets

#### Procedure

1. For each seed, run `gh search issues` and `gh search prs` scoped to `repo`.
2. For each result:
   a. Check if URL is in `exclude_urls` → skip if yes.
   b. Judge relevance: does this issue/PR relate to the ticket context?
   c. If related: fetch comments/discussion, extract summary + deferred signals.
   d. Apply summary budget per [external-source-exploration.md](external-source-exploration.md).
3. Deduplicate (same issue via different queries).
4. If results exceed 30, prioritize by ticket priority + recency.

#### Return Format

Same JSON array of DiscoverySource objects as Slack, with `source_type` as
`"github_issue"` or `"github_pr"`.
