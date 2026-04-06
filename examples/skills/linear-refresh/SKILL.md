---
name: linear-refresh-v2
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Invoke `/belt-agent` for protocol details.

## Role

You are the **orchestrator**. Your responsibilities:

1. Advance belt-agent phases (init → next → verify → step)
2. Dispatch sub-agents per phase with a reference file path
3. Present the unified plan at the approve phase

You do NOT:
- Hold ticket data or external source content
- Read `.belt/collected-context.json`, `.belt/plan-a.json`, or `.belt/plan-b.json`
- Load domain skills (`/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add`)

Sub-agents handle all data processing. They read reference files for instructions, load required skills, and write output to `.belt/` files. You receive only a summary string.

## Phase Map

| Phase | Dispatch | Reference |
|-------|----------|-----------|
| collect | Agent: fetch tickets + explore external sources | [collect-agent.md](references/collect-agent.md) |
| cleanup-analysis/analyze | Agent: detect structural issues | [cleanup-agent.md](references/cleanup-agent.md) |
| add-analysis/analyze | Agent: detect new ticket candidates | [add-agent.md](references/add-agent.md) |
| audit | Agent: ground truth audit + unified plan | [audit-agent.md](references/audit-agent.md) |
| approve | **Orchestrator**: read refresh-plan.json, present | [approve-format.md](references/approve-format.md) |
| execute | Agent: run cleanup then add changes | [execute-agent.md](references/execute-agent.md) |

## Dispatch Pattern

For each phase (except approve):

1. `belt-agent next` — get current phase info
2. Read the phase's reference file path from the table above
3. Dispatch Agent with prompt:

   > Read `{reference_path}` and execute.
   > Team: {team_id}. [Phase-specific parameters if any.]
   > Return only a count summary.

4. Agent completes → receive summary string
5. `belt-agent verify` → `belt-agent step` → next phase

## approve Phase (Orchestrator Direct)

The only phase where the orchestrator reads data:

1. `belt-agent next` — get approve phase info
2. Read `.belt/refresh-plan.json` (~30 lines)
3. Read [approve-format.md](references/approve-format.md) for display layout
4. Present formatted plan to user
5. Handle approval / modification / cancellation
6. `belt-agent step --confirm`

## Output Files

| File | Produced by |
|------|------------|
| `.belt/collected-context.json` | collect-agent |
| `.belt/plan-a.json` | cleanup-agent |
| `.belt/plan-b.json` | add-agent |
| `.belt/refresh-plan.json` | audit-agent |
| `.belt/refresh-result.json` | execute-agent |

## Red Flags

**Never:**
- Return ticket data from sub-agents to the orchestrator
- Load `/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add` in the orchestrator
- Read `.belt/collected-context.json` in the orchestrator
- Execute Linear API calls from the orchestrator
- Explore beyond 2 hops

**Always:**
- Dispatch one agent per phase
- Pass reference file path in the agent prompt
- Verify sub-agent wrote the expected gate file before running `belt-agent verify`
