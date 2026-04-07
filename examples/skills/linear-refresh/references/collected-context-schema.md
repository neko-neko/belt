# CollectedContext Schema

JSON schema for the `.linear-refresh/collected-context.json` artifact produced
by Step 1 (Collect) and extended by Step 2 (Discover).

## Structure

```json
{
  "team_id": "string",
  "tickets": [],
  "external_sources": [],
  "discovery_sources": []
}
```

## tickets[]

Each ticket entry contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Linear issue identifier (e.g., "RAKMY-20") |
| `title` | string | Issue title |
| `status` | string | Current status (Backlog, Todo, In Progress, Done, Cancelled) |
| `priority` | string | Priority level (Urgent, High, Medium, Low, No priority) |
| `labels` | string[] | Label names |
| `project` | string \| null | Project name if assigned |
| `parentId` | string \| null | Parent issue ID |
| `assignee` | string \| null | Assignee name |
| `completedAt` | string \| null | ISO 8601 completion timestamp |
| `archivedAt` | string \| null | ISO 8601 archive timestamp |
| `attachments` | object[] | `[{ "url": "string", "title": "string" }]` |
| `description_urls` | string[] | URLs extracted from description via `https?://[^\s)]+` |
| `relations` | object | `{ "relatedTo": [], "blocks": [], "blockedBy": [] }` |

## external_sources[]

Sources discovered via ticket URLs (Step 1). Each entry:

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Explored URL |
| `ticket_id` | string | Referring ticket ID |
| `hop` | number | 1 = from ticket URL, 2 = from referenced_urls in hop 1 |
| `accessible` | boolean | Whether the URL was successfully fetched |
| `summary` | string | Content summary (budget: 200/400/800 chars by priority) |
| `referenced_urls` | string[] | URLs mentioned in the fetched content |
| `latest_activity_ts` | string | ISO 8601 timestamp of last activity |
| `deferred_signals` | string[] | Detected deferred commitment patterns |
| `source_type` | string | One of: slack_thread, github_issue, github_pr, github_comment, document |

## discovery_sources[]

Sources discovered via keyword search / ticket reverse-lookup (Step 2).
Separate from external_sources because provenance and confidence differ.

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Discovered URL |
| `discovery_query` | string | The search query that found this source |
| `related_tickets` | string[] | Ticket IDs this source is judged to relate to |
| `accessible` | boolean | Whether the URL was successfully fetched |
| `summary` | string | Content summary (budget per external-source-exploration.md) |
| `latest_activity_ts` | string | ISO 8601 timestamp of last activity |
| `deferred_signals` | string[] | Detected deferred commitment patterns |
| `source_type` | string | One of: slack_message, slack_thread, github_issue, github_pr |
