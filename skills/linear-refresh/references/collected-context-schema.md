# CollectedContext Schema

JSON schema for the `.belt/collected-context.json` artifact produced by the collect phase.

## Structure

```json
{
  "team_id": "string",
  "tickets": [],
  "external_sources": []
}
```

## tickets[]

Each ticket entry contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Linear issue identifier (e.g., "BELT-20") |
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

Each external source entry contains:

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
