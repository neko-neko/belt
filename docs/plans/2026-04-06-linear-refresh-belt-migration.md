# linear-refresh Belt Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/linear-refresh` スキルを belt pipeline + skill の2層構造に移植し、既存スキル（belt-agent, smoke-test）にフロントマターを追加する。

**Architecture:** `pipelines/linear-refresh.yml` が6フェーズのトップレベルパイプラインを定義し、`linear-cleanup.yml` / `linear-add.yml` を sub-pipeline として参照する。`skills/linear-refresh/` にスキル本体 + 4リファレンスファイル。既存スキル2つにフロントマターを追加。

**Tech Stack:** YAML (belt pipeline), Markdown (skill files)

**Spec:** [docs/specs/2026-04-06-linear-refresh-belt-migration.md](../specs/2026-04-06-linear-refresh-belt-migration.md)

---

## File Structure

```
pipelines/
├── linear-refresh.yml                      # top-level (6 phases)
├── linear-cleanup.yml                      # sub-pipeline (1 phase)
└── linear-add.yml                          # sub-pipeline (1 phase)
skills/
├── belt-agent/
│   └── SKILL.md                            # MODIFY: add frontmatter
├── smoke-test/
│   └── SKILL.md                            # MODIFY: add frontmatter, fix ref
└── linear-refresh/
    ├── SKILL.md                            # NEW: flow + rules + HARD-GATE
    └── references/
        ├── collected-context-schema.md     # NEW: CollectedContext JSON schema
        ├── external-source-exploration.md  # NEW: URL filtering, budgets, hops
        ├── ground-truth-audit.md           # NEW: 3 audit questions + loop
        └── execution-report.md             # NEW: result report format
```

---

### Task 1: Add frontmatter to `skills/belt-agent/SKILL.md`

**Files:**
- Modify: `skills/belt-agent/SKILL.md:1-8`

- [ ] **Step 1: Add frontmatter and update header**

Replace lines 1-8 of `skills/belt-agent/SKILL.md`:

Before:
```markdown
# Belt Protocol

Generic protocol for driving the belt-agent CLI. This skill defines how LLM agents
interact with belt's deterministic state machine — the command loop, response
interpretation, and safety constraints.

Pipeline-specific skills reference this protocol for consistent belt-agent usage.
This skill is not invoked directly by users.
```

After:
```markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Generic protocol for driving the belt-agent CLI. Defines how LLM agents interact
with belt's deterministic state machine — the command loop, response interpretation,
and safety constraints.
```

- [ ] **Step 2: Verify frontmatter**

Run: `head -6 skills/belt-agent/SKILL.md`
Expected:
```
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---
```

---

### Task 2: Add frontmatter to `skills/smoke-test/SKILL.md`

**Files:**
- Modify: `skills/smoke-test/SKILL.md:1-8`

- [ ] **Step 1: Add frontmatter and fix belt-agent reference**

Replace lines 1-8 of `skills/smoke-test/SKILL.md`:

Before:
```markdown
# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline. It follows the
[Belt Protocol](../belt-agent/SKILL.md) for pipeline driving.
```

After:
```markdown
---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---

# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline.
Invoke /belt-agent for protocol details.
```

- [ ] **Step 2: Verify frontmatter and reference**

Run: `head -7 skills/smoke-test/SKILL.md`
Expected:
```
---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---
```

Run: `grep 'belt-agent\|Belt Protocol' skills/smoke-test/SKILL.md`
Expected: `Invoke /belt-agent for protocol details.` (no file path reference)

---

### Task 3: Create pipeline YAML files

**Files:**
- Create: `pipelines/linear-refresh.yml`
- Create: `pipelines/linear-cleanup.yml`
- Create: `pipelines/linear-add.yml`

- [ ] **Step 1: Create `pipelines/linear-refresh.yml`**

```yaml
name: linear-refresh
version: 1
args:
  force: { type: bool, default: false }

phases:
  - id: collect
    description: "Fetch all tickets and explore external sources (1-hop + 2-hop)."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/collected-context.json"

  - id: cleanup-analysis
    description: "Analyze tickets for structural issues."
    uses: ./pipelines/linear-cleanup.yml

  - id: add-analysis
    description: "Detect new ticket candidates from external sources."
    uses: ./pipelines/linear-add.yml

  - id: audit
    description: "Ground Truth audit — verify CollectedContext completeness and Plan quality."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-plan.json"
    validate:
      - "Every In Progress ticket's latest context is reflected in the plan"
      - "No untracked references remain for high-priority tickets"
      - "Deferred signals from external sources are addressed in the plan"
    regate: [collect]
    max_retries: 2

  - id: approve
    description: "Present unified plan for user approval."
    when: "!args.force"
    config:
      skill: "/linear-refresh"
    confirm: true

  - id: execute
    description: "Execute cleanup changes, then add changes."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-result.json"
```

- [ ] **Step 2: Create `pipelines/linear-cleanup.yml`**

```yaml
name: linear-cleanup
version: 1
description: "Analyze CollectedContext for structural issues in existing tickets."
phases:
  - id: analyze
    description: "Detect parent-child, blocking, duplicate, and status issues."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/plan-a.json"
```

- [ ] **Step 3: Create `pipelines/linear-add.yml`**

```yaml
name: linear-add
version: 1
description: "Detect new ticket candidates from CollectedContext, excluding Plan A items."
phases:
  - id: analyze
    description: "Identify create/link/skip candidates from external sources."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/plan-b.json"
```

- [ ] **Step 4: Validate with belt lint**

Run:
```bash
./target/debug/belt lint pipelines/linear-refresh.yml
```

Expected: lint passes with no errors.

- [ ] **Step 5: Validate with belt-agent init**

Run:
```bash
./target/debug/belt-agent init pipelines/linear-refresh.yml 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'pipeline: {d[\"pipeline\"]}')
print(f'first phase: {d[\"phase\"][\"id\"]}')
print(f'gate count: {len(d.get(\"gate\", []))}')
print('PASS' if d['phase']['id'] == 'collect' and d['pipeline'] == 'linear-refresh' else 'FAIL')
"
```

Expected: `PASS` with pipeline=linear-refresh, first phase=collect.

- [ ] **Step 6: Validate --force skips approve**

Run:
```bash
RUN_ID=$(./target/debug/belt-agent init pipelines/linear-refresh.yml --arg force=true 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])")
# advance: collect -> cleanup-analysis/analyze -> add-analysis/analyze -> audit -> execute (approve skipped)
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null > /dev/null
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null > /dev/null
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null > /dev/null
./target/debug/belt-agent step --confirm --run "$RUN_ID" 2>/dev/null > /dev/null
./target/debug/belt-agent next --run "$RUN_ID" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
phase_id = d.get('phase', {}).get('id', 'N/A')
print(f'current phase: {phase_id}')
print('PASS' if phase_id == 'execute' else f'FAIL - expected execute, got {phase_id}')
"
```

Expected: `PASS` — approve phase skipped, now at execute.

- [ ] **Step 7: Cleanup test runs**

Run: `rm -rf .belt/runs/`

---

### Task 4: Create `skills/linear-refresh/SKILL.md`

**Files:**
- Create: `skills/linear-refresh/SKILL.md`

- [ ] **Step 1: Create SKILL.md**

```markdown
---
name: linear-refresh
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets
and external sources once, analyzes for cleanup and add candidates, audits plan
quality, then executes approved changes.

This skill is used with the `pipelines/linear-refresh.yml` belt pipeline.
Invoke /belt-agent for protocol details.

## HARD-GATE

<HARD-GATE>
Before starting the collect phase, invoke /linear-cli and /slackcli skills
to load their context. These skills provide the CLI usage patterns required
for ticket retrieval and external source exploration.
</HARD-GATE>

## Output

- `.belt/collected-context.json` — all tickets + external sources
- `.belt/plan-a.json` — cleanup change candidates
- `.belt/plan-b.json` — add detection candidates
- `.belt/refresh-plan.json` — unified plan (cleanup + add)
- `.belt/refresh-result.json` — execution results

## Phase Map

| Phase | What to do | Reference |
|-------|-----------|-----------|
| collect | Fetch tickets, explore external sources | [collected-context-schema.md](references/collected-context-schema.md), [external-source-exploration.md](references/external-source-exploration.md) |
| cleanup-analysis/analyze | Analyze for structural issues | Read /linear-cleanup skill guidelines |
| add-analysis/analyze | Detect new ticket candidates | Read /linear-add skill guidelines |
| audit | Ground Truth audit, generate unified plan | [ground-truth-audit.md](references/ground-truth-audit.md) |
| approve | Present plan, wait for user approval | — |
| execute | Run cleanup then add changes | [execution-report.md](references/execution-report.md) |

## Phase: collect

1. Invoke /linear-cli and /slackcli (HARD-GATE).
2. Team selection: 1 team → auto-select, multiple → ask user, 0 → error.
3. Step 0-1: Fetch all tickets via `linear issue list`.
4. Step 0-2: Fetch details for active tickets (Agent parallel, 10-ticket batches).
5. Step 0-3: 1-hop external source exploration (Agent parallel).
   - See [external-source-exploration.md](references/external-source-exploration.md) for filtering, budgets.
6. Step 0-3b: 2-hop recursive expansion for high-priority tickets (Agent parallel).
   - Condition: In Progress + High/Urgent + activity within 72h.
7. Step 0-4: Generate `.belt/collected-context.json`.
   - See [collected-context-schema.md](references/collected-context-schema.md) for schema.

## Phase: cleanup-analysis/analyze

1. Read /linear-cleanup SKILL.md analysis guidelines.
2. Analyze CollectedContext for: parent-child, blocking, related, status, duplicates, context gaps.
3. Output `.belt/plan-a.json`.

## Phase: add-analysis/analyze

1. Read /linear-add SKILL.md detection criteria.
2. Exclude items already in Plan A (deduplication).
3. Analyze CollectedContext for new ticket candidates.
4. Output `.belt/plan-b.json`.

## Phase: audit

1. Read [ground-truth-audit.md](references/ground-truth-audit.md).
2. Run 3 audit questions (Q1-Q3) for each In Progress ticket.
3. If issues found and additional exploration needed:
   - Run single-shot exploration for specific URLs.
   - Update `.belt/collected-context.json`.
   - Regenerate Plan A/B.
4. Generate unified plan `.belt/refresh-plan.json`.

## Phase: approve

1. Present unified plan with Cleanup and Add sections.
2. Wait for user approval.
   - Approve → proceed to execute.
   - Modify → update plan, re-present.
   - Cancel → exit.
3. Skipped when `--force` is set.

## Phase: execute

1. **Cleanup** (in order):
   - Parent-child relationships.
   - Parallel: blocking, related, status, context additions.
   - Duplicate merges (close + duplicateOf).
2. **Add** (in order):
   - Create new tickets.
   - Link to existing tickets.
3. Error handling: skip individual failures, continue execution.
4. Generate `.belt/refresh-result.json`.
   - See [execution-report.md](references/execution-report.md) for format.

## Red Flags

**Never:**
- Execute changes without an approved plan (unless --force)
- Delete or archive tickets (cleanup closes duplicates only)
- Rewrite ticket descriptions (context additions use comments/attachments)
- Explore beyond 2 hops (infinite expansion prevention)

**Always:**
- Invoke /linear-cli and /slackcli before collect
- Respect summary budgets (200/400/800 chars by priority)
- Record deferred signals from external sources
- Report all execution failures in result JSON
```

- [ ] **Step 2: Verify**

Run: `head -6 skills/linear-refresh/SKILL.md`
Expected:
```
---
name: linear-refresh
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---
```

---

### Task 5: Create `skills/linear-refresh/references/collected-context-schema.md`

**Files:**
- Create: `skills/linear-refresh/references/collected-context-schema.md`

- [ ] **Step 1: Create collected-context-schema.md**

```markdown
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
```

---

### Task 6: Create `skills/linear-refresh/references/external-source-exploration.md`

**Files:**
- Create: `skills/linear-refresh/references/external-source-exploration.md`

- [ ] **Step 1: Create external-source-exploration.md**

```markdown
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
```

---

### Task 7: Create `skills/linear-refresh/references/ground-truth-audit.md`

**Files:**
- Create: `skills/linear-refresh/references/ground-truth-audit.md`

- [ ] **Step 1: Create ground-truth-audit.md**

```markdown
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
```

---

### Task 8: Create `skills/linear-refresh/references/execution-report.md`

**Files:**
- Create: `skills/linear-refresh/references/execution-report.md`

- [ ] **Step 1: Create execution-report.md**

```markdown
# Execution Report

Format for the `.belt/refresh-result.json` artifact produced by the execute phase.

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
```

---

### Task 9: Commit all files

**Files:**
- Stage: all modified and new files

- [ ] **Step 1: Commit existing skill modifications**

Run:
```bash
git add skills/belt-agent/SKILL.md skills/smoke-test/SKILL.md
git commit -m "chore: add frontmatter to belt-agent and smoke-test skills

- belt-agent: add name, description, user-invocable: false
- smoke-test: add name, description, argument-hint; change
  Belt Protocol file reference to /belt-agent skill invoke"
```

- [ ] **Step 2: Commit linear-refresh files**

Run:
```bash
git add pipelines/linear-refresh.yml pipelines/linear-cleanup.yml pipelines/linear-add.yml skills/linear-refresh/
git commit -m "feat: add linear-refresh belt pipeline and skill

Migrates /linear-refresh to belt's 2-layer architecture:
- pipelines/linear-refresh.yml: 6-phase pipeline with regate-based
  Ground Truth audit loop and --force approval skip
- pipelines/linear-cleanup.yml: cleanup analysis sub-pipeline
- pipelines/linear-add.yml: add analysis sub-pipeline
- skills/linear-refresh/SKILL.md: flow + HARD-GATE + phase map
- skills/linear-refresh/references/: 4 reference docs"
```

---

### Task 10: Cleanup

- [ ] **Step 1: Remove test runs**

Run: `rm -rf .belt/runs/`
