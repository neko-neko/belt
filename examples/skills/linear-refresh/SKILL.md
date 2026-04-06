---
name: linear-refresh-v2
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets
and external sources once, analyzes for cleanup and add candidates, audits plan
quality, then executes approved changes.

This skill is used with the `linear-refresh.yml` belt pipeline.
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
