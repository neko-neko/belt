# SKILL.md Authoring Principle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a SKILL.md authoring principle and rewrite all example SKILL.md files to eliminate redundancy with pipeline.yml and belt-agent SKILL.md.

**Architecture:** Documentation-only changes across 7 files. CLAUDE.md gets the principle, 3 SKILL.md files get rewritten, 3 pipeline YAML files get `config.reference` keys. No Rust code changes.

**Tech Stack:** Markdown, YAML, belt lint CLI

---

### Task 1: Add authoring principle to CLAUDE.md

**Files:**
- Modify: `CLAUDE.md:200-205` (after "CLI 命名" section)

- [ ] **Step 1: Add the SKILL.md Authoring Principle section**

Insert after the "CLI 命名" section (line 205), before "Verification Contract":

```markdown
### SKILL.md Authoring Principle

- SKILL.md documents only what pipeline.yml and belt-agent SKILL.md cannot express
- Phase structure is declared by pipeline.yml and returned dynamically by belt-agent next/status. Do not re-describe in SKILL.md
- Protocol is taught by belt-agent SKILL.md. Do not re-state
- SKILL.md responsibilities: (1) config key interpretation rules, (2) domain-specific constraints, (3) references/ pointers
```

- [ ] **Step 2: Verify the addition**

Run: `grep -n "Authoring Principle" CLAUDE.md`
Expected: One match showing the new section header.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(CLAUDE.md): add SKILL.md authoring principle"
```

---

### Task 2: Rewrite feature-dev/SKILL.md

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md` (full rewrite, 74 → ~35 lines)

- [ ] **Step 1: Rewrite the file**

Replace the entire content with:

```markdown
---
name: feature-dev
description: >-
  Quality-gated development orchestrator. Drives a 10-phase pipeline
  (design → review → plan → review → execute → doc-audit → smoke-test →
  code-review → test-review → integrate) via belt-agent CLI.
  Conditional phases: --e2e (test-review), --smoke (smoke-test), --doc (doc-audit).
  Passthrough flags: --codex, --ui, --iterations N, --swarm.
user-invocable: true
argument-hint: "[--e2e] [--smoke] [--doc] [--codex] [--ui] [--iterations N] [--swarm]"
---

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

- [ ] **Step 2: Verify no Phase Map or belt-agent loop re-description**

Run: `grep -c "Phase Map\|Belt-Agent Loop\|init.*next.*verify.*step" examples/skills/feature-dev/SKILL.md`
Expected: `0`

- [ ] **Step 3: Verify line count reduction**

Run: `wc -l examples/skills/feature-dev/SKILL.md`
Expected: ~38 lines (down from 74)

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "refactor(feature-dev): rewrite SKILL.md per authoring principle

Remove Belt-Agent Loop section and implicit Phase Map.
Keep dispatch rules (work/audit/lite), evidence plan, red flags."
```

---

### Task 3: Rewrite smoke-test/SKILL.md

**Files:**
- Modify: `examples/skills/smoke-test/SKILL.md` (remove Phase Map table, ~82 → ~55 lines)

- [ ] **Step 1: Remove the Phase Map table**

Remove lines 16-20 (the Phase Map section):

```markdown
## Phase Map

| Phase | What to do | Reference |
|-------|-----------|-----------|
| env-setup | Start dev server, verify accessible | [server-detection.md](references/server-detection.md) |
| adhoc-test | Generate & execute smoke scenarios | [scenario-generation.md](references/scenario-generation.md), [report-template.md](references/report-template.md) |
| vrt-check | Run VRT if tooling detected | [vrt-detection.md](references/vrt-detection.md) |
| e2e-detection | Run E2E with flaky detection | [e2e-flaky-detection.md](references/e2e-flaky-detection.md) |
```

The per-phase sections (`## Phase: env-setup`, etc.) already contain reference links and domain logic. The Phase Map table is purely redundant.

- [ ] **Step 2: Verify no Phase Map remains**

Run: `grep -c "Phase Map" examples/skills/smoke-test/SKILL.md`
Expected: `0`

- [ ] **Step 3: Verify per-phase sections still intact**

Run: `grep -c "^## Phase:" examples/skills/smoke-test/SKILL.md`
Expected: `4` (env-setup, adhoc-test, vrt-check, e2e-detection)

- [ ] **Step 4: Commit**

```bash
git add examples/skills/smoke-test/SKILL.md
git commit -m "refactor(smoke-test): remove Phase Map per authoring principle

Per-phase sections already contain reference links and domain logic.
Phase Map table was redundant with pipeline.yml structure."
```

---

### Task 4: Add config.reference to linear-refresh pipeline files

**Files:**
- Modify: `examples/skills/linear-refresh/pipeline.yml` (4 phases)
- Modify: `examples/skills/linear-refresh/linear-cleanup.yml` (1 phase)
- Modify: `examples/skills/linear-refresh/linear-add.yml` (1 phase)

- [ ] **Step 1: Add config.reference to pipeline.yml phases**

Modify `examples/skills/linear-refresh/pipeline.yml`:

For `collect` phase, change:
```yaml
  - id: collect
    description: "Fetch all tickets and explore external sources (1-hop + 2-hop)."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/collected-context.json"
```
to:
```yaml
  - id: collect
    description: "Fetch all tickets and explore external sources (1-hop + 2-hop)."
    config:
      skill: "/linear-refresh"
      reference: "references/collect-agent.md"
    gate:
      - file_exists: ".belt/collected-context.json"
```

For `audit` phase, change:
```yaml
    config:
      skill: "/linear-refresh"
```
to:
```yaml
    config:
      skill: "/linear-refresh"
      reference: "references/audit-agent.md"
```

For `approve` phase, change:
```yaml
    config:
      skill: "/linear-refresh"
```
to:
```yaml
    config:
      skill: "/linear-refresh"
      reference: "references/approve-format.md"
```

For `execute` phase, change:
```yaml
    config:
      skill: "/linear-refresh"
```
to:
```yaml
    config:
      skill: "/linear-refresh"
      reference: "references/execute-agent.md"
```

- [ ] **Step 2: Add config.reference to linear-cleanup.yml**

Change `examples/skills/linear-refresh/linear-cleanup.yml`:

```yaml
    config:
      skill: "/linear-cleanup"
```
to:
```yaml
    config:
      skill: "/linear-cleanup"
      reference: "references/cleanup-agent.md"
```

- [ ] **Step 3: Add config.reference to linear-add.yml**

Change `examples/skills/linear-refresh/linear-add.yml`:

```yaml
    config:
      skill: "/linear-add"
```
to:
```yaml
    config:
      skill: "/linear-add"
      reference: "references/add-agent.md"
```

- [ ] **Step 4: Verify all reference files exist**

Run:
```bash
for f in \
  examples/skills/linear-refresh/references/collect-agent.md \
  examples/skills/linear-refresh/references/audit-agent.md \
  examples/skills/linear-refresh/references/approve-format.md \
  examples/skills/linear-refresh/references/execute-agent.md \
  examples/skills/linear-refresh/references/cleanup-agent.md \
  examples/skills/linear-refresh/references/add-agent.md; do
  [ -f "$f" ] && echo "OK: $f" || echo "MISSING: $f"
done
```
Expected: All 6 lines show `OK:`.

- [ ] **Step 5: Run belt lint on modified pipeline**

Run: `cargo run -p belt -- lint examples/skills/linear-refresh/pipeline.yml`
Expected: No errors. `config` is opaque to belt; adding keys does not affect lint.

- [ ] **Step 6: Commit**

```bash
git add examples/skills/linear-refresh/pipeline.yml \
       examples/skills/linear-refresh/linear-cleanup.yml \
       examples/skills/linear-refresh/linear-add.yml
git commit -m "feat(linear-refresh): add config.reference to pipeline phases

Move phase→reference mapping from SKILL.md Phase Map into pipeline.yml
config keys. belt passes config through as opaque map; each skill
interprets config.reference in its Dispatch Rules."
```

---

### Task 5: Rewrite linear-refresh/SKILL.md

**Files:**
- Modify: `examples/skills/linear-refresh/SKILL.md` (full rewrite, 86 → ~35 lines)

- [ ] **Step 1: Rewrite the file**

Replace the entire content with:

```markdown
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
```

- [ ] **Step 2: Verify no Phase Map, Dispatch Pattern, or Output Files table**

Run: `grep -c "Phase Map\|Dispatch Pattern\|Output Files" examples/skills/linear-refresh/SKILL.md`
Expected: `0`

- [ ] **Step 3: Verify no belt-agent loop re-description**

Run: `grep -c "init.*next.*verify.*step\|belt-agent next.*phase info\|belt-agent verify.*belt-agent step" examples/skills/linear-refresh/SKILL.md`
Expected: `0`

- [ ] **Step 4: Verify line count reduction**

Run: `wc -l examples/skills/linear-refresh/SKILL.md`
Expected: ~40 lines (down from 86)

- [ ] **Step 5: Commit**

```bash
git add examples/skills/linear-refresh/SKILL.md
git commit -m "refactor(linear-refresh): rewrite SKILL.md per authoring principle

Remove Phase Map, Dispatch Pattern, Output Files tables.
Keep role definition, config.reference dispatch rule, red flags.
Phase structure now lives in pipeline.yml; reference mapping in config."
```

---

### Task 6: Cross-check verification

**Files:**
- Read-only verification across all modified files

- [ ] **Step 1: Verify no SKILL.md contains Phase Map tables**

Run: `grep -rl "Phase Map" examples/skills/*/SKILL.md`
Expected: No output (no matches).

- [ ] **Step 2: Verify no SKILL.md re-describes belt-agent loop**

Run: `grep -rn "belt-agent init\|belt-agent next.*→\|init → next → verify" examples/skills/*/SKILL.md`
Expected: No matches in feature-dev or linear-refresh. smoke-test may have individual command references in per-phase instructions (acceptable — domain usage, not protocol re-description).

- [ ] **Step 3: Verify no Decision Rules overlap with belt-agent SKILL.md**

Run: `grep -rn "Skip.*verify\|go directly to.*step\|verify returns FAIL" examples/skills/*/SKILL.md`
Expected: No matches.

- [ ] **Step 4: Verify CLAUDE.md contains the principle**

Run: `grep -c "SKILL.md Authoring Principle" CLAUDE.md`
Expected: `1`

- [ ] **Step 5: Verify config.reference keys in pipeline YAML**

Run: `grep -c "reference:" examples/skills/linear-refresh/pipeline.yml examples/skills/linear-refresh/linear-cleanup.yml examples/skills/linear-refresh/linear-add.yml`
Expected: `pipeline.yml:4`, `linear-cleanup.yml:1`, `linear-add.yml:1`

- [ ] **Step 6: Run belt lint on linear-refresh pipeline**

Run: `cargo run -p belt -- lint examples/skills/linear-refresh/pipeline.yml`
Expected: No errors.

- [ ] **Step 7: Line count summary**

Run: `wc -l examples/skills/feature-dev/SKILL.md examples/skills/smoke-test/SKILL.md examples/skills/linear-refresh/SKILL.md`
Expected totals:
- feature-dev: ~38 lines (was 74)
- smoke-test: ~70 lines (was 82)
- linear-refresh: ~40 lines (was 86)
