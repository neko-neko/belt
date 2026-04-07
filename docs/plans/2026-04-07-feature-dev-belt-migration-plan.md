# feature-dev Belt Pipeline Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the feature-dev skill (10-phase quality-gated development orchestrator) into a belt pipeline example at `examples/skills/feature-dev/`.

**Architecture:** belt pipeline (pipeline.yml) handles deterministic phase transitions, gates, regate, and conditional skipping. SKILL.md provides slim orchestrator protocol (~4KB). Done-criteria and reference files live in `references/`. All repository content in English.

**Tech Stack:** belt-agent CLI, YAML pipeline format, Markdown references

**Spec:** `docs/specs/2026-04-07-feature-dev-belt-migration.md`

---

## File Structure

```
examples/skills/feature-dev/
├── pipeline.yml                          # CREATE — 19-phase pipeline definition
├── belt.toml                             # CREATE — pipeline_file pointer
├── SKILL.md                              # CREATE — slim orchestrator protocol
└── references/
    ├── done-criteria/
    │   ├── design.md                     # MIGRATE from phase-1-design.md
    │   ├── spec-review.md                # MIGRATE from phase-2-spec-review.md
    │   ├── plan.md                       # MIGRATE from phase-3-plan.md
    │   ├── plan-review.md                # MIGRATE from phase-4-plan-review.md
    │   ├── execute.md                    # MIGRATE from phase-5-execute.md
    │   ├── doc-audit.md                  # CREATE — new (no source)
    │   ├── smoke-test.md                 # MIGRATE from phase-7-smoke-test.md
    │   ├── code-review.md                # MIGRATE from phase-8-code-review.md
    │   └── test-review.md                # MIGRATE from phase-9-test-review.md
    ├── audit-protocol.md                 # CREATE — condensed from audit-gate-protocol.md
    ├── evidence-plan-protocol.md         # CREATE — extracted from feature-dev SKILL.md
    └── fix-dispatch-strategy.md          # CREATE — extracted from feature-dev SKILL.md
```

Source directory: `/Users/nishikataseiichi/go/src/github.com/neko-neko/dotfiles/claude/skills/feature-dev/`

---

### Task 1: Directory structure, pipeline.yml, belt.toml

**Files:**
- Create: `examples/skills/feature-dev/pipeline.yml`
- Create: `examples/skills/feature-dev/belt.toml`
- Create: `examples/skills/feature-dev/references/done-criteria/` (directory)

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p examples/skills/feature-dev/references/done-criteria
```

- [ ] **Step 2: Write pipeline.yml**

Write the full pipeline.yml to `examples/skills/feature-dev/pipeline.yml`. Content is defined in the spec (`docs/specs/2026-04-07-feature-dev-belt-migration.md`, section "pipeline.yml 全文"). Copy the YAML verbatim from the spec.

- [ ] **Step 3: Write belt.toml**

Write to `examples/skills/feature-dev/belt.toml`:

```toml
pipeline_file = "pipeline.yml"
```

- [ ] **Step 4: Verify with belt lint**

```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
```

Expected: `ok: examples/skills/feature-dev/pipeline.yml`

- [ ] **Step 5: Commit**

```bash
git add examples/skills/feature-dev/pipeline.yml examples/skills/feature-dev/belt.toml
git commit -m "feat(examples): add feature-dev pipeline.yml and belt.toml"
```

---

### Task 2: SKILL.md — orchestrator protocol

**Files:**
- Create: `examples/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

Write to `examples/skills/feature-dev/SKILL.md`:

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

## Belt-Agent Loop

```
belt-agent init pipeline.yml [--smoke=true ...] → loop:
  1. belt-agent next           → phase info (JSON)
  2. Dispatch (see below)      → do the work
  3. belt-agent verify         → run gate checks
  4. belt-agent regate         → run regate targets (if any)
  5. belt-agent step [--confirm] → advance to next phase
```

## Phase Dispatch Rules

### Work phases (config.skill present)

Invoke the skill specified in `config.skill`. Pass other config keys
(`codex`, `iterations`, `swarm`, `ui`) as options to the skill invocation.

Example: if `belt-agent next` returns:
```json
{
  "phase": { "config": { "skill": "/code-review", "codex": "args.codex", "iterations": "args.iterations" } }
}
```
Then invoke `/code-review` with `--codex` and `--iterations` flags as indicated by
the resolved arg values in the `args` field of the response.

### Audit phases (config.audit == "required")

1. Read `references/done-criteria/{config.criteria}.md`
2. Dispatch a `phase-auditor` subagent following `references/audit-protocol.md`
3. Write `verdict.json` to the phase's `output_dir`
4. Run `belt-agent verify` (the `has_output: true` gate checks the file exists)
5. If verdict is PASS: `belt-agent step --confirm`
6. If verdict is FAIL: apply fix per `references/fix-dispatch-strategy.md`, then re-audit

### Integrate phase (audit: lite)

No separate audit phase. The orchestrator directly evaluates the `validate` criteria
and runs `belt-agent step --confirm` after user chooses integration method.

## Evidence Plan

Generated after `design-audit` completes. Re-evaluated after `plan-review-audit`
if design hash changed. Details: `references/evidence-plan-protocol.md`.
Collection requirements are injected into executor prompts for `execute` and later phases.

## Red Flags — NEVER DO

- Skip the `design` phase
- Auto-answer brainstorming design questions
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Pass an audit phase with only `belt-agent verify --passed` (auditor dispatch is mandatory)
- Proceed past a FAIL verdict without fix + re-audit or user intervention
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "feat(examples): add feature-dev SKILL.md orchestrator protocol"
```

---

### Task 3: Migrate done-criteria files (8 files)

**Files:**
- Create: `examples/skills/feature-dev/references/done-criteria/design.md`
- Create: `examples/skills/feature-dev/references/done-criteria/spec-review.md`
- Create: `examples/skills/feature-dev/references/done-criteria/plan.md`
- Create: `examples/skills/feature-dev/references/done-criteria/plan-review.md`
- Create: `examples/skills/feature-dev/references/done-criteria/execute.md`
- Create: `examples/skills/feature-dev/references/done-criteria/smoke-test.md`
- Create: `examples/skills/feature-dev/references/done-criteria/code-review.md`
- Create: `examples/skills/feature-dev/references/done-criteria/test-review.md`

**Source directory:** `/Users/nishikataseiichi/go/src/github.com/neko-neko/dotfiles/claude/skills/feature-dev/done-criteria/`

**Transformation rules** (apply to ALL files):

| Rule | Before | After |
|------|--------|-------|
| Filename | `phase-N-{name}.md` | `{name}.md` |
| Frontmatter `phase:` | `phase: N` | Remove entirely |
| Frontmatter `audit:` | Keep as-is | Keep as-is |
| Criteria ID prefix | `DN-XX` | `{NAME}-XX` (uppercase of phase name) |
| Language | Japanese | English |
| `Observation Collection` section | Japanese | English (standardized) |
| Artifact paths with `phase-N` | `phase-N-review.json` | `{name}-review.json` |

**Criteria ID mapping:**

| Source | Target | File |
|--------|--------|------|
| `D1-01` through `D1-07` | `DESIGN-01` through `DESIGN-07` | design.md |
| `D2-01` through `D2-05` | `SPEC-REVIEW-01` through `SPEC-REVIEW-05` | spec-review.md |
| `D3-01` through `D3-06` | `PLAN-01` through `PLAN-06` | plan.md |
| `D4-01` through `D4-04` | `PLAN-REVIEW-01` through `PLAN-REVIEW-04` | plan-review.md |
| `D5-01` through `D5-09` | `EXECUTE-01` through `EXECUTE-09` | execute.md |
| `D7-01` through `D7-04` | `SMOKE-TEST-01` through `SMOKE-TEST-04` | smoke-test.md |
| `D8-01` through `D8-04` | `CODE-REVIEW-01` through `CODE-REVIEW-04` | code-review.md |
| `D9-01` through `D9-03` | `TEST-REVIEW-01` through `TEST-REVIEW-03` | test-review.md |

**Standardized Observation Collection section** (append to each file):

```markdown
## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 1: Read all 8 source done-criteria files**

Read each file from the source directory. The files are:
- `phase-1-design.md` (6.4KB, 7 criteria)
- `phase-2-spec-review.md` (5.2KB, 5 criteria)
- `phase-3-plan.md` (5.5KB, 6 criteria)
- `phase-4-plan-review.md` (4.3KB, 4 criteria)
- `phase-5-execute.md` (9.2KB, 9 criteria)
- `phase-7-smoke-test.md` (5.1KB, 4 criteria)
- `phase-8-code-review.md` (4.3KB, 4 criteria)
- `phase-9-test-review.md` (3.3KB, 3 criteria)

- [ ] **Step 2: Transform and write each file**

For each source file, apply ALL transformation rules above and write to the target path.
Key transformation details per file:

**design.md**: Remove `phase: 1` from frontmatter. `D1-01` → `DESIGN-01` through `D1-07` → `DESIGN-07`. Translate all Japanese to English. Keep `max_retries: 3` and `audit: required`.

**spec-review.md**: Remove `phase: 2`. `D2-01` → `SPEC-REVIEW-01` through `D2-05` → `SPEC-REVIEW-05`. Replace `phase-2-review.json` → `spec-review-review.json` in artifact paths.

**plan.md**: Remove `phase: 3`. `D3-01` → `PLAN-01` through `D3-06` → `PLAN-06`.

**plan-review.md**: Remove `phase: 4`. `D4-01` → `PLAN-REVIEW-01` through `D4-04` → `PLAN-REVIEW-04`. Replace `phase-4-review.json` → `plan-review-review.json`.

**execute.md**: Remove `phase: 5`. `D5-01` → `EXECUTE-01` through `D5-09` → `EXECUTE-09`. This is the largest file (9 criteria).

**smoke-test.md**: Remove `phase: 7`. `D7-01` → `SMOKE-TEST-01` through `D7-04` → `SMOKE-TEST-04`.

**code-review.md**: Remove `phase: 8`. `D8-01` → `CODE-REVIEW-01` through `D8-04` → `CODE-REVIEW-04`. Replace `phase-8-review.json` → `code-review-review.json`.

**test-review.md**: Remove `phase: 9`. `D9-01` → `TEST-REVIEW-01` through `D9-03` → `TEST-REVIEW-03`. Replace `phase-9-review.json` → `test-review-review.json`.

- [ ] **Step 3: Verify all files exist and have content**

```bash
ls -la examples/skills/feature-dev/references/done-criteria/
wc -c examples/skills/feature-dev/references/done-criteria/*.md
```

Expected: 8 files, each with non-zero size.

- [ ] **Step 4: Spot-check: verify no Japanese text remains**

```bash
# Check for any remaining Japanese characters (CJK range)
grep -rP '[\x{3000}-\x{9FFF}]' examples/skills/feature-dev/references/done-criteria/ || echo "No Japanese text found — PASS"
```

Expected: "No Japanese text found — PASS"

- [ ] **Step 5: Spot-check: verify no D-number IDs remain**

```bash
grep -rE 'D[0-9]+-[0-9]+' examples/skills/feature-dev/references/done-criteria/ || echo "No legacy IDs found — PASS"
```

Expected: "No legacy IDs found — PASS"

- [ ] **Step 6: Commit**

```bash
git add examples/skills/feature-dev/references/done-criteria/
git commit -m "feat(examples): add feature-dev done-criteria (migrated to English, semantic IDs)"
```

---

### Task 4: Create doc-audit.md done-criteria (new)

**Files:**
- Create: `examples/skills/feature-dev/references/done-criteria/doc-audit.md`

This file has no source — the original feature-dev had no Phase 6 done-criteria. Write based on the doc-audit skill's behavior (4-layer document audit).

- [ ] **Step 1: Write doc-audit.md**

Write to `examples/skills/feature-dev/references/done-criteria/doc-audit.md`:

```markdown
---
name: doc-audit
max_retries: 3
audit: required
---

## Criteria

### DOC-AUDIT-01: Doc audit report exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Check for the existence of a doc-audit report file in the output directory or working directory.
- **pass_condition**: At least one doc-audit report file exists
- **fail_diagnosis_hint**: Verify that `/doc-audit` skill was invoked and completed. Check if the report was written to the correct directory.
- **depends_on_artifacts**: []

### DOC-AUDIT-02: All broken dependency links are resolved
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Parse all markdown files with `depends-on` frontmatter
  2. Verify each declared dependency path exists via Glob
  3. Check for undeclared dependencies (file path references in body text without `depends-on` entry)
- **pass_condition**: Zero broken dependency links. Zero undeclared dependencies detected.
- **fail_diagnosis_hint**: Run `doc-check` to identify and fix broken dependencies. Undeclared deps need `depends-on` frontmatter additions.
- **depends_on_artifacts**: [docs/]

### DOC-AUDIT-03: No stale documentation detected
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Compare doc-audit findings against the current code state
  2. Verify that documentation references (function names, file paths, API endpoints) match the current implementation
  3. Check that documented behavior matches actual behavior for changed modules
- **pass_condition**: Zero stale documentation findings unresolved. All flagged items either fixed or confirmed current.
- **fail_diagnosis_hint**: Cross-reference the doc-audit report's stale signals with `git diff` to identify which docs need updating. Focus on docs whose `depends-on` targets were modified.
- **depends_on_artifacts**: [docs/, src/]

### DOC-AUDIT-04: User-approved fixes applied
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all findings from the doc-audit report that were approved by the user for fixing
  2. Verify each approved fix has been applied (check git diff or file content)
  3. Ensure no approved fix was skipped or partially applied
- **pass_condition**: All user-approved fixes fully applied. Zero approved-but-unapplied items.
- **fail_diagnosis_hint**: Check the doc-audit report for the list of approved findings and cross-reference with recent commits. Partially applied fixes may need manual completion.
- **depends_on_artifacts**: [docs/]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/feature-dev/references/done-criteria/doc-audit.md
git commit -m "feat(examples): add doc-audit done-criteria (new for belt pipeline)"
```

---

### Task 5: Reference files — audit-protocol, evidence-plan, fix-dispatch

**Files:**
- Create: `examples/skills/feature-dev/references/audit-protocol.md`
- Create: `examples/skills/feature-dev/references/evidence-plan-protocol.md`
- Create: `examples/skills/feature-dev/references/fix-dispatch-strategy.md`

**Source:** `/Users/nishikataseiichi/go/src/github.com/neko-neko/dotfiles/claude/skills/feature-dev/references/audit-gate-protocol.md` (10KB) and feature-dev SKILL.md sections.

- [ ] **Step 1: Write audit-protocol.md**

Read the source `audit-gate-protocol.md`, then condense and translate to English. Focus on the belt-relevant parts: verdict format, auditor dispatch, PASS/FAIL handling, cumulative diagnosis. Remove sections that belt handles (phase transitions, handover state, re-gate loop — belt manages these via pipeline.yml regate).

Write to `examples/skills/feature-dev/references/audit-protocol.md`:

```markdown
# Audit Protocol

## Overview

Each audit phase dispatches a `phase-auditor` subagent to independently verify
the preceding work phase against its done-criteria. This protocol defines the
dispatch procedure, verdict format, and failure handling.

## Auditor Dispatch

When `belt-agent next` returns a phase with `config.audit == "required"`:

1. Read `references/done-criteria/{config.criteria}.md`
2. Compose the Audit Context (see template below)
3. Launch a `phase-auditor` subagent via the Agent tool
4. Validate the returned JSON (must have all required fields)
5. Write the verdict to `{output_dir}/verdict.json`

If the JSON is invalid, retry once. If still invalid, PAUSE.

## Audit Context Template

Inject the following into the phase-auditor prompt:

```
## Audit Context

### Phase
name: {criteria name from config}
attempt: {current attempt number, from belt-agent next response}

### Done Criteria
{full content of references/done-criteria/{criteria}.md}

### Artifacts to Verify
- primary: {artifacts from the work phase — design docs, plan docs, code changes, etc.}
- dependencies: {artifacts from prior phases referenced by done-criteria}

### Cumulative Diagnosis (attempt 2+ only)
{previous verdict(s) and their fail details, so the auditor knows what was already tried}
```

## Verdict Format

The phase-auditor must return JSON in this structure:

```json
{
  "verdict": "PASS | FAIL",
  "criteria_results": [
    {
      "id": "DESIGN-01",
      "passed": true,
      "severity": "blocker",
      "detail": "Design file found at docs/plans/2026-04-07-foo-design.md"
    }
  ],
  "summary": {
    "total": 7,
    "passed": 7,
    "failed": 0,
    "blocking_issues": [],
    "quality_warnings": ["DESIGN-05: alternatives section is brief"]
  },
  "observations": [
    {
      "type": "quality",
      "content": "Design doc is well-structured but alternatives section could be more detailed"
    }
  ],
  "escalation": null
}
```

### Required fields
- `verdict`: "PASS" or "FAIL"
- `criteria_results`: array with one entry per criterion
- `summary`: counts + blocking issues + quality warnings
- `observations`: array (may be empty, but field must exist)
- `escalation`: null or object with `reason` and `recommendation`

## Verdict Rules

- **PASS**: All `blocker` criteria pass. Quality warnings are reported but don't block.
- **FAIL**: At least one `blocker` criterion fails.
- **FAIL with escalation**: The auditor identifies a fundamental issue that retries cannot fix (e.g., design is fundamentally flawed). Set `escalation` to a non-null object. This triggers an immediate PAUSE regardless of remaining retries.

## Failure Handling

When verdict is FAIL (no escalation):
1. Extract `fix_instruction` from each failed criterion's detail
2. Apply fix per `references/fix-dispatch-strategy.md`
3. Re-run `belt-agent verify` to confirm output still exists
4. The orchestrator re-dispatches the auditor (attempt increments automatically via belt)

When `max_retries` (3) is exhausted:
1. Compile a cumulative diagnosis from all attempts
2. PAUSE and present the diagnosis to the user
3. User intervention resets the attempt counter

## PAUSE Recovery

After user intervenes and instructs to continue:
1. belt's `max_retries` counter has been exhausted — user must acknowledge
2. Apply any user-directed fixes
3. Re-run the audit from the beginning (the orchestrator manages this via belt-agent)
```

- [ ] **Step 2: Write evidence-plan-protocol.md**

Write to `examples/skills/feature-dev/references/evidence-plan-protocol.md`:

```markdown
# Evidence Plan Protocol

## Overview

The Evidence Plan defines what evidence must be collected during pipeline execution
to support audit decisions. It is generated once and updated as the design evolves.

## Lifecycle

| Event | Action |
|-------|--------|
| `design-audit` PASS | Generate Evidence Plan |
| `plan-review-audit` PASS | Re-evaluate if design doc hash changed since generation |
| `execute` and later phases | Inject collection requirements into executor prompts |

## Generation

After `design-audit` passes, the orchestrator generates the Evidence Plan by analyzing:

1. The design document's requirements and test perspectives
2. The done-criteria for all upcoming phases
3. Project characteristics (language, framework, UI presence, API presence)

The plan is written to the `design-audit` output directory.

## Structure

```json
{
  "project_type": "rust-cli | web-frontend | api-backend | ...",
  "has_ui": false,
  "has_api": false,
  "activities": [
    {
      "type": "implementation",
      "phases": ["execute"],
      "collect": ["build output", "test results", "lint results", "coverage report"]
    },
    {
      "type": "review",
      "phases": ["spec-review", "plan-review", "code-review", "test-review"],
      "collect": ["review findings JSON", "consensus findings count", "applied fixes"]
    },
    {
      "type": "smoke-test",
      "phases": ["smoke-test"],
      "collect": ["smoke-test-report.md", "screenshots", "flaky test list"]
    },
    {
      "type": "doc-maintenance",
      "phases": ["doc-audit"],
      "collect": ["doc-audit report", "broken deps count", "stale signals"]
    }
  ]
}
```

## Injection

When dispatching a work phase executor, include the relevant collection requirements:

> "In addition to the phase work, collect the following evidence and write to the output directory:
> {list from Evidence Plan for this phase's activity type}"

The auditor verifies that required evidence was actually collected.
```

- [ ] **Step 3: Write fix-dispatch-strategy.md**

Write to `examples/skills/feature-dev/references/fix-dispatch-strategy.md`:

```markdown
# Fix Dispatch Strategy

When an audit phase returns FAIL, the orchestrator applies fixes using the
executor appropriate for the failed work phase.

## Dispatch Table

| Work Phase | Fix Executor | Strategy |
|------------|-------------|----------|
| design | Orchestrator | Re-read investigation record, rescan codebase if info is missing |
| spec-review | Orchestrator | Edit design doc directly based on audit findings |
| plan | Orchestrator | Edit plan doc directly based on audit findings |
| plan-review | Orchestrator | Edit plan doc directly based on audit findings |
| execute | `feature-implementer` subagent | Decompose fix instructions into TDD tasks, launch with full task context |
| doc-audit | Orchestrator or `feature-implementer` | depends-on fixes → Edit directly; content fixes → invoke /doc-check; new docs → feature-implementer |
| smoke-test | `feature-implementer` subagent | Bug fixes to implementation code |
| code-review | `feature-implementer` subagent | Apply review finding fixes |
| test-review | `feature-implementer` subagent | Apply test code fixes |

## Fix Context Template

When dispatching a subagent for fixes, inject:

```
## Fix Context

### Failed Criteria
{criterion ID, severity, and detail from the audit verdict}

### Fix Instructions
{the auditor's recommended fix — what to change, where, and why}

### Current State
{relevant git diff or file content showing the current state}

### Verification
After applying the fix, verify by:
{the criterion's verification steps from done-criteria}
```

## Rules

- The orchestrator MUST NOT fix on behalf of a subagent executor.
  If the dispatch table says `feature-implementer`, launch one.
- Fixes that produce code changes will trigger regate on the next `belt-agent step`
  (belt handles this automatically via the pipeline's regate configuration).
- If a fix is blocked (cannot be applied), report `blocked` status and PAUSE.
```

- [ ] **Step 4: Verify all reference files exist**

```bash
ls -la examples/skills/feature-dev/references/*.md
```

Expected: 3 files (audit-protocol.md, evidence-plan-protocol.md, fix-dispatch-strategy.md)

- [ ] **Step 5: Commit**

```bash
git add examples/skills/feature-dev/references/audit-protocol.md \
        examples/skills/feature-dev/references/evidence-plan-protocol.md \
        examples/skills/feature-dev/references/fix-dispatch-strategy.md
git commit -m "feat(examples): add feature-dev reference files (audit, evidence, fix-dispatch)"
```

---

### Task 6: Integration verification

**Files:** None (read-only verification)

- [ ] **Step 1: Run belt lint**

```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
```

Expected: `ok: examples/skills/feature-dev/pipeline.yml`

- [ ] **Step 2: Verify full pipeline structure**

```bash
find examples/skills/feature-dev -type f | sort
```

Expected output:
```
examples/skills/feature-dev/SKILL.md
examples/skills/feature-dev/belt.toml
examples/skills/feature-dev/pipeline.yml
examples/skills/feature-dev/references/audit-protocol.md
examples/skills/feature-dev/references/done-criteria/code-review.md
examples/skills/feature-dev/references/done-criteria/design.md
examples/skills/feature-dev/references/done-criteria/doc-audit.md
examples/skills/feature-dev/references/done-criteria/execute.md
examples/skills/feature-dev/references/done-criteria/plan-review.md
examples/skills/feature-dev/references/done-criteria/plan.md
examples/skills/feature-dev/references/done-criteria/smoke-test.md
examples/skills/feature-dev/references/done-criteria/spec-review.md
examples/skills/feature-dev/references/done-criteria/test-review.md
examples/skills/feature-dev/references/evidence-plan-protocol.md
examples/skills/feature-dev/references/fix-dispatch-strategy.md
```

Total: 15 files

- [ ] **Step 3: Belt-agent init + walkthrough (default args)**

Build belt-agent first: `cargo build -p belt-agent`

```bash
cd /tmp && rm -rf belt-fdev-verify && mkdir belt-fdev-verify && cd belt-fdev-verify && git init -q
BELT_AGENT="<project-root>/target/debug/belt-agent"
# Copy pipeline to temp dir (belt-agent resolves paths relative to cwd)
cp -r <project-root>/examples/skills/feature-dev/* .

# Init
$BELT_AGENT init $PIPELINE 2>&1 | jq '{phase: .phase.id, gate: .gate}'
# Expected: phase: "design"

# Verify phase count from status
$BELT_AGENT status 2>&1 | jq '.progress'
# Expected: { completed: 0, skipped: 0, remaining: 19, total: 19 }
```

- [ ] **Step 4: Verify conditional skipping (no flags = skip 6 phases)**

Continue from step 3, fast-forward through all phases (verify + step --confirm)
until pipeline completes. Verify final status:

```bash
$BELT_AGENT status 2>&1 | jq '.progress'
# Expected: { completed: 13, skipped: 6, remaining: 0, total: 19 }
```

The 6 skipped phases should be: doc-audit, doc-audit-audit, smoke-test, smoke-test-audit, test-review, test-review-audit.

- [ ] **Step 5: Verify conditional enabling (--smoke=true)**

```bash
cd /tmp && rm -rf belt-fdev-verify2 && mkdir belt-fdev-verify2 && cd belt-fdev-verify2 && git init -q
cp -r <project-root>/examples/skills/feature-dev/* .
$BELT_AGENT init pipeline.yml --smoke=true 2>&1 | jq '{phase: .phase.id}'
$BELT_AGENT status 2>&1 | jq '.progress'
# Expected remaining: 19 (total), skipped phases will appear after fast-forward
```

Fast-forward and verify: `{ completed: 15, skipped: 4, remaining: 0, total: 19 }`

- [ ] **Step 6: Verify no Japanese text in any example file**

```bash
grep -rP '[\x{3000}-\x{9FFF}]' examples/skills/feature-dev/ || echo "PASS: no Japanese text"
```

Expected: "PASS: no Japanese text"

- [ ] **Step 7: Clean up temp directories**

```bash
rm -rf /tmp/belt-fdev-verify /tmp/belt-fdev-verify2
```

---

### Task 7: Final commit and verify

- [ ] **Step 1: Verify all changes are committed**

```bash
git status -- examples/skills/feature-dev/
```

Expected: nothing to commit (all files already committed in Tasks 1-5)

- [ ] **Step 2: Verify commit history**

```bash
git log --oneline -- examples/skills/feature-dev/ | head -10
```

Expected: 4 commits matching Tasks 1-5.
