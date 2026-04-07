# Collect Agent

Sub-agent instructions for Step 1 (Collect). Three types of I/O sub-agents
are dispatched in parallel by the main agent.

## Sub-Agent Types

---

### Detail Fetch Agent

Fetches ticket details in batches of 10.

#### Prerequisites

1. Invoke `/linear-cli` skill to load CLI usage patterns.

#### Input

- `ticket_ids`: List of ticket IDs to fetch (max 10 per batch)

#### Procedure

1. For each ticket ID, run `linear issue show {ticket_id}`.
2. Extract from each result:
   - `attachments`: `[{ "url": "string", "title": "string" }]`
   - `description_urls`: URLs extracted via regex `https?://[^\s)]+`
   - `relations`: `{ "relatedTo": [], "blocks": [], "blockedBy": [] }`
3. Return enriched ticket objects.

#### Return Format

```json
[
  {
    "id": "RAKMY-81",
    "attachments": [{ "url": "https://...", "title": "..." }],
    "description_urls": ["https://github.com/...", "https://admin.rakmy.jp/..."],
    "relations": { "relatedTo": ["RAKMY-82"], "blocks": [], "blockedBy": [] }
  }
]
```

---

### 1-Hop Exploration Agent

Explores external URLs linked from ticket descriptions and attachments.

#### Prerequisites

1. Invoke `/slackcli` skill for Slack thread exploration.

#### Input

- `urls`: List of URLs to explore (pre-filtered by main agent)
- `ticket_context`: For each URL, the referring ticket's ID, title, and priority
- `classification`: Per-URL classification (explore / metadata-only) from main agent

#### Procedure

1. For each URL classified as "explore":
   a. Use `/slackcli` for Slack URLs, WebFetch for others.
   b. Summarize content within budget:
      - Backlog/Low priority ticket: 200 chars
      - Todo/Medium: 400 chars
      - In Progress/Urgent/High: 800 chars + raw excerpts
   c. Extract `referenced_urls` (URLs mentioned in the fetched content).
   d. Detect `deferred_signals` per [external-source-exploration.md](external-source-exploration.md).
2. For each URL classified as "metadata-only":
   a. Check title + accessibility only.
3. Return ExternalSource objects.

#### Return Format

```json
[
  {
    "url": "https://rakmy.slack.com/archives/CHAN/p123",
    "ticket_id": "RAKMY-81",
    "hop": 1,
    "accessible": true,
    "summary": "Thread discusses...",
    "referenced_urls": ["https://github.com/..."],
    "latest_activity_ts": "2026-04-05T10:00:00+09:00",
    "deferred_signals": ["will update after testing"],
    "source_type": "slack_thread"
  }
]
```

---

### 2-Hop Exploration Agent

Expands `referenced_urls` from 1-hop results under strict conditions.

#### Prerequisites

1. Invoke `/slackcli` skill for Slack thread exploration.

#### Input

- `urls`: List of URLs to explore (pre-filtered by main agent per 2-hop criteria)
- `ticket_context`: Referring ticket info from the 1-hop source

#### 2-Hop Criteria (applied by main agent before dispatch)

ALL conditions must be met for a URL to qualify:
- Referring ticket is **In Progress** AND **Urgent or High** priority
- `latest_activity_ts` from 1-hop is within **72 hours** of current time

URL types to follow: GitHub PR/Issue comments, cross-references, Slack threads, dev URLs.
URL types to skip: Already explored in 1-hop, static documents, images, 3rd+ hop URLs.

#### Procedure

Same as 1-hop exploration agent, but mark all results with `hop: 2`.

#### Return Format

Same as 1-hop, with `"hop": 2`.
