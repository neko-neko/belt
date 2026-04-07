# SKILL.md Authoring Principle & Example Rewrite

**Linear**: BELT-20 (child)
**Status**: Draft
**Date**: 2026-04-07

## Summary

Establish an authoring principle for belt pipeline SKILL.md files that eliminates redundancy with pipeline.yml and belt-agent SKILL.md. Rewrite all example SKILL.md files to conform.

## Background

belt's 2-layer architecture separates declaration (pipeline.yml) from interpretation (SKILL.md + references/). However, current example SKILL.md files violate this boundary by re-describing:

- The belt-agent command loop (init → next → verify → step) — already in `skills/belt-agent/SKILL.md`
- Phase Map tables mirroring pipeline.yml's phase structure — already returned by `belt-agent next/status`
- Decision Rules (no gate → skip verify, etc.) — already in belt-agent's Decision Rules table
- HARD-GATE re-statements — already in belt-agent's HARD-GATE section

This drift-prone redundancy contradicts belt's "Do One Thing" philosophy and the established principle that belt CLI outputs facts while skills provide behavioral instructions.

### Design Decision History

- `4b59262`: Initial belt-agent Protocol skill (172 lines, full JSON examples)
- `272c09b`: Simplified to command-example-driven reference (87 lines) — "agents learn from actual output"
- `a50a4b2`: linear-refresh SKILL.md rewritten to orchestration-only (158 → 68 lines) — Lean Orchestrator pattern
- `e65c7cb`: Added regate protocol and step troubleshooting to belt-agent SKILL.md

## Authoring Principle

> SKILL.md documents ONLY what pipeline.yml and belt-agent SKILL.md cannot express.

### Three responsibilities of SKILL.md

| # | Responsibility | Example |
|---|---|---|
| 1 | Config key interpretation rules | `config.audit: "required"` → dispatch phase-auditor agent |
| 2 | Domain-specific constraints / Red Flags | "Never auto-answer brainstorming design questions" |
| 3 | Pointers to references/ | "Read `references/done-criteria/{config.criteria}.md`" |

### What SKILL.md must NOT contain

| Content | Where it belongs |
|---|---|
| Phase structure (IDs, ordering, descriptions) | pipeline.yml |
| Gate checks, regate targets, when conditions | pipeline.yml |
| Belt-agent command loop | belt-agent SKILL.md |
| Decision Rules (verify skip, FAIL handling) | belt-agent SKILL.md |
| HARD-GATE (validate + --confirm) | belt-agent SKILL.md |
| Phase Map tables | Eliminated — `belt-agent next/status` returns this dynamically |

### SKILL.md Structure Template

```markdown
# {Skill Name}
One-line role definition.

## Dispatch Rules
Config key interpretation rules (table or prose).
How to translate config from `belt-agent next` into actions.

## [Domain-specific sections]
Domain logic that pipeline.yml cannot express (only if needed).

## Red Flags
Skill-specific constraints. Must not overlap with belt-agent HARD-GATE.
```

## Config Key Responsibility Separation

belt-agent SKILL.md's Well-known Config Keys remains `config.skill` only. No expansion.

Each SKILL.md defines its own config vocabulary in its Dispatch Rules section. belt passes all config keys through as an opaque map; interpretation is the skill's responsibility.

## Changes Per File

### belt-agent SKILL.md

No changes to Well-known Config Keys. The authoring principle is enforced by CLAUDE.md, not by belt-agent.

### feature-dev/SKILL.md (74 → ~35 lines)

**Remove:**
- Belt-Agent Loop section (lines 20-28) — belt-agent SKILL.md covers this
- Implicit Phase Map structure — pipeline.yml declares this

**Keep:**
- Dispatch Rules: config key interpretation for 3 dispatch types (work / audit / lite)
- Evidence Plan reference (1 line)
- Red Flags (remove HARD-GATE overlap: "Pass an audit phase with only belt-agent verify --passed")

**Rewritten structure:**

```markdown
# Feature Dev Orchestrator
Quality-gated development pipeline driven by belt-agent.
belt handles phase transitions, gates, regate, and conditional skipping.
The orchestrator dispatches skills per phase and auditor agents per audit.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `config.skill` present | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options (resolved from `args`) |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch `phase-auditor` subagent per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |
| `config.audit: "lite"` | Orchestrator directly evaluates `validate` criteria. `step --confirm` after user chooses integration method |

## Evidence Plan

Generated after `design-audit` completes. Re-evaluated after `plan-review-audit`
if design hash changed. Details: `references/evidence-plan-protocol.md`.

## Red Flags — NEVER DO

- Skip the `design` phase
- Auto-answer brainstorming design questions
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention
```

### smoke-test/SKILL.md (82 → ~55 lines)

**Remove:**
- Phase Map table (lines 16-20) — pipeline.yml declares this

**Keep:**
- Per-phase execution instructions (env-setup, adhoc-test, vrt-check, e2e-detection) — domain logic, not expressible in pipeline.yml
- Red Flags
- Unify section headers to `## Phase: {id}` for runtime lookup via `belt-agent next` phase ID

**Note:** smoke-test already has minimal belt-agent overlap. Reduction is modest.

### linear-refresh/SKILL.md (86 → ~35 lines)

**Remove:**
- Phase Map table (lines 28-36) — pipeline.yml declares this
- Dispatch Pattern section (lines 37-51) — belt-agent loop re-description
- Output Files table (lines 63-70) — inferable from gate `file_exists` checks

**Keep:**
- Role definition (orchestrator responsibility boundary)
- approve phase orchestrator-direct rule (exception behavior)
- Dispatch Rules: `config.reference` interpretation (1 line)
- Red Flags

**pipeline.yml change:** Add `reference` key to each phase's config:

```yaml
# Before
- id: collect
  config:
    skill: "/linear-refresh"

# After
- id: collect
  config:
    skill: "/linear-refresh"
    reference: "references/collect-agent.md"
```

Apply to phases in `pipeline.yml`: collect, audit, approve, execute.
Apply to sub-pipeline files: `linear-cleanup.yml` (analyze phase), `linear-add.yml` (analyze phase).

**Rewritten structure:**

```markdown
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
```

### linear-cleanup/SKILL.md, linear-add/SKILL.md

**Out of scope.** These are standalone domain skills, not belt pipeline orchestrators. The authoring principle applies to skills that drive belt pipelines.

## CLAUDE.md Addition

Add to the "CLI 命名" or similar conventions section:

```markdown
### SKILL.md Authoring Principle

- SKILL.md documents only what pipeline.yml and belt-agent SKILL.md cannot express
- Phase structure is declared by pipeline.yml and returned dynamically by belt-agent next/status. Do not re-describe in SKILL.md
- Protocol is taught by belt-agent SKILL.md. Do not re-state
- SKILL.md responsibilities: (1) config key interpretation rules, (2) domain-specific constraints, (3) references/ pointers
```

## Scope Boundaries

### In scope
- Authoring principle establishment (CLAUDE.md)
- feature-dev/SKILL.md rewrite
- smoke-test/SKILL.md rewrite
- linear-refresh/SKILL.md rewrite
- linear-refresh/pipeline.yml config enrichment

### Out of scope
- linear-cleanup/SKILL.md, linear-add/SKILL.md (standalone domain skills)
- belt-agent SKILL.md changes (Well-known Config Keys stays as-is)
- New features or behavioral changes to belt-agent CLI
- Reference file content changes

## Test Plan

### Verification

| # | Check | Method |
|---|---|---|
| 1 | Rewritten SKILL.md contains no Phase Map tables | Manual review |
| 2 | Rewritten SKILL.md contains no belt-agent loop re-description | Grep for `init.*next.*verify.*step` pattern |
| 3 | Rewritten SKILL.md contains no Decision Rules overlap | Compare against belt-agent SKILL.md Decision Rules |
| 4 | pipeline.yml config.reference keys match existing reference file paths | `file_exists` check for each path |
| 5 | CLAUDE.md contains the authoring principle | Manual review |
| 6 | belt lint passes on modified pipeline.yml | `cargo run -p belt -- lint examples/skills/linear-refresh/pipeline.yml` |
