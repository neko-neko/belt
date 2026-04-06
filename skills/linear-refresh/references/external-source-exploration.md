# External Source Exploration

Rules for collecting and summarizing external sources linked from Linear tickets.

## URL Filtering

Classify each URL before exploration:

| Classification | Action | Examples |
|---------------|--------|---------|
| Explore (full) | Fetch content + summarize | Slack threads, GitHub issues/PRs with discussion |
| Metadata only | Check title + accessibility | Google Docs/Sheets, Notion pages, static documents |
| Skip | Do not explore | Images, screenshots, already-registered attachments |

## Summary Budgets

Budget is determined by the referring ticket's priority:

| Ticket Priority | Budget | Additional Requirements |
|----------------|--------|------------------------|
| Backlog / Low | 200 chars | Basic summary |
| Todo / Medium | 400 chars | Preserve `referenced_urls` in separate field |
| In Progress / Urgent / High | 800 chars + raw excerpts | Latest messages in full, all mentioned URLs listed, `deferred_signals` recorded |

## Deferred Signals

Patterns in external sources that indicate pending commitments. Detect and record these:

- "will follow up" / "will update"
- "checking" / "confirming" / "investigating"
- "planned for release" / "scheduled for"
- "added to backlog" / "will prioritize"
- "pending review" / "awaiting approval"

## 1-Hop Exploration (Step 0-3)

For every URL found in ticket descriptions and attachments:

1. Classify using the filtering table above.
2. For "explore" URLs: dispatch Agent in parallel with ticket context (title, description summary).
3. Each agent returns the standard ExternalSource fields (see collected-context-schema.md).
4. Use WebFetch as fallback if specialized tools are unavailable.

## 2-Hop Recursive Expansion (Step 0-3b)

Expand `referenced_urls` from 1-hop results under strict conditions:

**ALL conditions must be met:**
- Referring ticket is **In Progress** AND **Urgent or High** priority
- `latest_activity_ts` is within **72 hours** of refresh execution

**URL types to follow:**

| Type | Examples | Reason |
|------|---------|--------|
| GitHub PR/Issue comments | `#issuecomment-*`, `#discussion_r*` | Spec updates, review feedback |
| GitHub cross-references | `owner/repo#123` | Related work |
| Slack threads (same workspace) | `slack.com/archives/.../p*` | Nested discussion |
| Dev-related URLs | `github.io`, `*.vercel.app` | Specs, mocks, prototypes |

**URL types to skip:**
- Already explored in Step 0-3 (deduplication)
- Static documents (Google Docs/Sheets, Notion) — metadata only
- Images and screenshots
- **3rd hop and beyond** (infinite expansion prevention)

## Agent Prompt Structure

> Given the following context, explore the URL and return a structured summary.
>
> **Ticket:** {ticket_id} — {ticket_title}
> **URL:** {url}
> **Budget:** {budget} characters
>
> Return: summary, referenced_urls, latest_activity_ts, deferred_signals, source_type
