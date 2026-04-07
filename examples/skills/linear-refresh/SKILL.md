---
name: linear-refresh-v2
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Orchestrates linear-cleanup and linear-add in a single workflow.

## Role

You are the **orchestrator**. Your responsibilities:

1. Drive belt-agent through the pipeline
2. Dispatch sub-agents per phase with the reference file from `config.reference`
3. Present the unified plan at the approve phase

You do NOT:
- Hold ticket data or external source content
- Read `.belt/collected-context.json`, `.belt/plan-a.json`, or `.belt/plan-b.json`
- Load domain skills (`/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add`)

## Dispatch Rules

| config pattern | Action |
|---|---|
| `config.reference` present | Dispatch Agent: "Read `{config.reference}` and execute. Return only a count summary." |
| Phase `approve` | Orchestrator direct: read `.belt/refresh-plan.json`, format per `config.reference`, present to user |

## Red Flags

**Never:**
- Return ticket data from sub-agents to the orchestrator
- Load `/linear-cli`, `/slackcli`, `/linear-cleanup`, `/linear-add` in the orchestrator
- Read `.belt/collected-context.json` in the orchestrator
- Execute Linear API calls from the orchestrator
- Explore beyond 2 hops

**Always:**
- Dispatch one agent per phase
- Verify sub-agent wrote the expected gate file before running `belt-agent verify`
