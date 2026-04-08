# Audit Gate Pattern Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the repeated audit gate pattern into a reusable `audit-gate` sub-pipeline with done-criteria schema and standard catalog.

**Architecture:** Single-phase sub-pipeline (`audit-gate/pipeline.yml`) referenced via `uses:` from consumer pipelines. Consumers inject `criteria`, `regate`, and `when` through belt's sub-pipeline expansion rules. Done-criteria files remain consumer-owned; audit-gate provides canonical templates.

**Tech Stack:** belt YAML pipelines, belt lint CLI

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `examples/skills/audit-gate/pipeline.yml` | Create | Audit mechanism sub-pipeline (single phase) |
| `examples/skills/audit-gate/references/audit-protocol.md` | Create | Shared mechanism spec (genericized from existing) |
| `examples/skills/audit-gate/done-criteria/_schema.md` | Create | Done-criteria format definition |
| `examples/skills/audit-gate/done-criteria/code-review.md` | Create | Standard catalog — canonical (identical to consumer versions) |
| `examples/skills/audit-gate/done-criteria/execute.md` | Create | Standard catalog — generic version |
| `examples/skills/audit-gate/done-criteria/smoke-test.md` | Create | Standard catalog — generic version |
| `examples/skills/audit-gate/done-criteria/test-review.md` | Create | Standard catalog — generic version |
| `examples/skills/feature-dev/pipeline.yml` | Modify | Replace 9 audit leaf phases with `uses: ../audit-gate/pipeline.yml` |
| `examples/skills/debug-flow/pipeline.yml` | Modify | Replace 7 audit leaf phases with `uses: ../audit-gate/pipeline.yml` |
| `examples/skills/feature-dev/SKILL.md` | Modify | Update audit-protocol.md path reference |
| `examples/skills/debug-flow/SKILL.md` | Modify | Update audit-protocol.md path reference |
| `examples/skills/feature-dev/references/audit-protocol.md` | Delete | Replaced by audit-gate's shared version |
| `examples/skills/debug-flow/references/audit-protocol.md` | Delete | Replaced by audit-gate's shared version |

---

### Task 1: Create audit-gate/pipeline.yml

**Files:**
- Create: `examples/skills/audit-gate/pipeline.yml`

- [ ] **Step 1: Create the audit-gate sub-pipeline**

```yaml
name: audit-gate
description: "Reusable audit gate — dispatches phase-auditor against done-criteria"
version: 1
phases:
  - id: check
    description: "Phase-auditor dispatch against done-criteria"
    config:
      audit: required
    gate:
      - has_output: true
    validate:
      - "All audit criteria pass"
    confirm: true
    max_retries: 3
```

Write to `examples/skills/audit-gate/pipeline.yml`.

- [ ] **Step 2: Verify YAML is valid**

Run: `target/debug/belt lint examples/skills/audit-gate/pipeline.yml`
Expected: `ok: examples/skills/audit-gate/pipeline.yml` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/pipeline.yml
git commit -m "feat(examples): add audit-gate sub-pipeline"
```

---

### Task 2: Create audit-gate/references/audit-protocol.md

**Files:**
- Create: `examples/skills/audit-gate/references/audit-protocol.md`
- Reference: `examples/skills/feature-dev/references/audit-protocol.md` (current version to genericize)

- [ ] **Step 1: Create the shared audit-protocol.md**

Copy from `examples/skills/feature-dev/references/audit-protocol.md` and genericize 3 locations:

1. Line 36 — change `{artifacts from the work phase — design docs, plan docs, code changes, etc.}` to `{artifacts from the work phase}`
2. Line 52 — change `"id": "DESIGN-01"` to `"id": "CRITERIA-01"` and `"Design file found at docs/plans/2026-04-07-foo-design.md"` to `"Primary artifact verified at expected path"`
3. Lines 63, 68 — change `"DESIGN-05: alternatives section is brief"` to `"CRITERIA-05: section could be more detailed"` and `"Design doc is well-structured but alternatives section could be more detailed"` to `"Artifact is well-structured but some sections could be more detailed"`

All other content (dispatch procedure, verdict format, verdict rules, failure handling, PAUSE recovery) stays identical.

- [ ] **Step 2: Verify content matches protocol structure**

Run: `grep -c "^##" examples/skills/audit-gate/references/audit-protocol.md`
Expected: `7` (7 level-2 headings: Overview, Auditor Dispatch, Audit Context Template, Verdict Format, Verdict Rules, Failure Handling, PAUSE Recovery).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/references/audit-protocol.md
git commit -m "feat(examples): add shared audit-protocol.md to audit-gate"
```

---

### Task 3: Create audit-gate/done-criteria/_schema.md

**Files:**
- Create: `examples/skills/audit-gate/done-criteria/_schema.md`

- [ ] **Step 1: Create the format definition**

```markdown
# Done-Criteria Schema

This document defines the format for done-criteria files used by the audit-gate pattern.
Each done-criteria file is consumed by the `phase-auditor` subagent during audit phases.

## File Format

Done-criteria files use markdown with YAML frontmatter:

### Frontmatter (required)

```yaml
---
name: <criteria-name>          # Must match the config.criteria value
max_retries: 3                 # Must match audit-gate's max_retries
audit: required                # Audit mode
---
```

### Body Structure

```markdown
## Criteria

### <PREFIX>-<NN>: <criterion title>
- **severity**: blocker | quality
- **verify_type**: automated | inspection
- **verification**:
  <specific verification steps>
- **pass_condition**: <quantitative pass condition>
- **fail_diagnosis_hint**: <investigation guidance for auditor on FAIL>
- **depends_on_artifacts**: [<dependent artifact paths>]
- **forward_check**: <impact note for downstream phases>

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
```

## Field Specification

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `severity` | Yes | `blocker` or `quality` | `blocker` causes verdict FAIL. `quality` produces warnings only |
| `verify_type` | Yes | `automated` or `inspection` | `automated`: verifiable by command execution. `inspection`: requires LLM judgment |
| `verification` | Yes | text | Step-by-step verification procedure. For `automated`, include concrete commands |
| `pass_condition` | Yes | text | Quantitative pass condition (counts, existence, patterns) |
| `fail_diagnosis_hint` | Yes | text | Guidance for auditor to determine root cause and fix strategy on FAIL |
| `depends_on_artifacts` | No | list | Artifact paths this criterion depends on |
| `forward_check` | No | text | Note to prevent redundant verification in downstream audit phases |

## Naming Conventions

- **Criterion ID**: `<PHASE_PREFIX>-<NN>` where prefix is the uppercased criteria name
  - `execute` → `EXECUTE-01`, `EXECUTE-02`, ...
  - `code-review` → `CODE-REVIEW-01`, `CODE-REVIEW-02`, ...
- **File name**: matches `config.criteria` value with `.md` extension
  - `config.criteria: "execute"` → `execute.md`

## Verdict Rules

The phase-auditor applies these rules to produce a verdict:

- **PASS**: All `blocker` criteria pass. Quality warnings are reported but do not block
- **FAIL**: At least one `blocker` criterion fails
- **FAIL with escalation**: Fundamental issue that retries cannot fix. Triggers immediate PAUSE
```

Write to `examples/skills/audit-gate/done-criteria/_schema.md`.

- [ ] **Step 2: Commit**

```bash
git add examples/skills/audit-gate/done-criteria/_schema.md
git commit -m "feat(examples): add done-criteria schema definition to audit-gate"
```

---

### Task 4: Create audit-gate/done-criteria/code-review.md

**Files:**
- Create: `examples/skills/audit-gate/done-criteria/code-review.md`
- Source: `examples/skills/feature-dev/references/done-criteria/code-review.md` (identical to debug-flow's)

- [ ] **Step 1: Copy code-review.md**

Copy `examples/skills/feature-dev/references/done-criteria/code-review.md` to `examples/skills/audit-gate/done-criteria/code-review.md` verbatim. This file is already identical between feature-dev and debug-flow — no genericization needed.

- [ ] **Step 2: Verify files are identical**

Run: `diff examples/skills/audit-gate/done-criteria/code-review.md examples/skills/feature-dev/references/done-criteria/code-review.md`
Expected: no output (exit 0).

Run: `diff examples/skills/audit-gate/done-criteria/code-review.md examples/skills/debug-flow/references/done-criteria/code-review.md`
Expected: no output (exit 0).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/done-criteria/code-review.md
git commit -m "feat(examples): add code-review canonical done-criteria to audit-gate"
```

---

### Task 5: Create audit-gate/done-criteria/execute.md

**Files:**
- Create: `examples/skills/audit-gate/done-criteria/execute.md`
- Reference: `examples/skills/feature-dev/references/done-criteria/execute.md`
- Reference: `examples/skills/debug-flow/references/done-criteria/execute.md`

- [ ] **Step 1: Create generic execute.md**

Start from feature-dev's `execute.md`. Genericize the 5 context-dependent references:

1. EXECUTE-01 line 13: `the plan document` → `the implementation plan document`
   - line 18: `in the plan document` → `in the implementation plan document`
   - line 19: `[docs/plans/*-plan.md]` → `[docs/plans/]`
2. EXECUTE-05 line 52: `the plan document's test case section` → `the implementation plan document's test case section`
   - line 58: `[docs/plans/*-plan.md, tests/]` → `[docs/plans/, tests/]`
3. EXECUTE-06 line 64: `defined in the design document` → `defined in the project's architecture documentation`
   - line 68: `with the design document's component diagram` → `with the project's architecture documentation`
   - line 69: `[docs/plans/*-design.md, src/]` → `[docs/plans/, src/]`
4. EXECUTE-07 line 71: `End-to-end traceability from design to plan to implementation` → `End-to-end traceability from requirements to plan to implementation`
   - line 75: `all requirements from the design document` → `all requirements from the requirements source document`
   - line 76: `the plan document task ID` → `the implementation plan document task ID`
   - lines 78-82: `requirement-to-task-to-implementation` → `requirement-to-task-to-implementation` (already generic)
   - line 82: `[docs/plans/*-design.md, docs/plans/*-plan.md, src/]` → `[docs/plans/, src/]`
5. EXECUTE-09 line 96: `Test cases cover both requirement coverage and impact scope` → keep as-is (already generic enough)
   - line 100: `all requirements from the design document` → `all requirements from the requirements source document`
   - line 101: `the impacted files/modules from the impact-analyzer output` → `the impacted files/modules from the impact analysis`
   - line 107: `[docs/plans/*-design.md, tests/, src/]` → `[docs/plans/, tests/, src/]`

All other criteria (EXECUTE-02, 03, 04, 08) are already generic — copy verbatim.

- [ ] **Step 2: Verify schema compliance**

Run: `grep -c "^### EXECUTE-" examples/skills/audit-gate/done-criteria/execute.md`
Expected: `9` (9 criteria).

Run: `grep -c "severity.*blocker" examples/skills/audit-gate/done-criteria/execute.md`
Expected: `8` (8 blockers — EXECUTE-06 is quality).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/done-criteria/execute.md
git commit -m "feat(examples): add generic execute done-criteria to audit-gate"
```

---

### Task 6: Create audit-gate/done-criteria/smoke-test.md

**Files:**
- Create: `examples/skills/audit-gate/done-criteria/smoke-test.md`
- Reference: `examples/skills/feature-dev/references/done-criteria/smoke-test.md`
- Reference: `examples/skills/debug-flow/references/done-criteria/smoke-test.md`

- [ ] **Step 1: Create generic smoke-test.md**

Start from feature-dev's `smoke-test.md`. Genericize 1 location:

1. SMOKE-TEST-03 line 37: `the design document` → `the project's requirements documentation`
   - line 39: `reference the design document` → `reference the project's requirements documentation`
   - line 40: `[artifacts/smoke-test/, docs/plans/*-design.md]` → `[artifacts/smoke-test/, docs/plans/]`

All other criteria (SMOKE-TEST-01, 02, 04) are already generic — copy verbatim.

- [ ] **Step 2: Verify schema compliance**

Run: `grep -c "^### SMOKE-TEST-" examples/skills/audit-gate/done-criteria/smoke-test.md`
Expected: `4` (4 criteria).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/done-criteria/smoke-test.md
git commit -m "feat(examples): add generic smoke-test done-criteria to audit-gate"
```

---

### Task 7: Create audit-gate/done-criteria/test-review.md

**Files:**
- Create: `examples/skills/audit-gate/done-criteria/test-review.md`
- Reference: `examples/skills/feature-dev/references/done-criteria/test-review.md`
- Reference: `examples/skills/debug-flow/references/done-criteria/test-review.md`

- [ ] **Step 1: Create generic test-review.md**

Start from feature-dev's `test-review.md`. Genericize 1 location:

1. TEST-REVIEW-03 line 30: `All design test perspectives are covered by test code` → `All required test perspectives are covered by test code`
   - line 34: `from the design document's test perspectives section` → `from the project's test perspectives documentation`
   - line 41: `[docs/plans/*-design.md, tests/]` → `[docs/plans/, tests/]`

All other criteria (TEST-REVIEW-01, 02) are already generic — copy verbatim.

- [ ] **Step 2: Verify schema compliance**

Run: `grep -c "^### TEST-REVIEW-" examples/skills/audit-gate/done-criteria/test-review.md`
Expected: `3` (3 criteria).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/audit-gate/done-criteria/test-review.md
git commit -m "feat(examples): add generic test-review done-criteria to audit-gate"
```

---

### Task 8: Modify feature-dev/pipeline.yml

**Files:**
- Modify: `examples/skills/feature-dev/pipeline.yml:24-183`

- [ ] **Step 1: Replace all 9 audit leaf phases with uses: references**

Replace each audit leaf phase with the `uses:` pattern. The full modified pipeline.yml:

```yaml
name: feature-dev
description: "Quality-gated development orchestrator"
version: 1
args:
  e2e: { type: bool, default: false }
  smoke: { type: bool, default: false }
  doc: { type: bool, default: false }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm: { type: bool, default: false }

phases:
  # ─── Design ───
  - id: design
    description: "Create design spec via brainstorming"
    config:
      skill: "/brainstorming"
      swarm: "args.swarm"
    gate:
      - file_exists: "docs/plans/*-design.md"

  - id: design-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "design"

  # ─── Spec Review ───
  - id: spec-review
    uses: ../spec-review/pipeline.yml

  - id: spec-review-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "spec-review"

  # ─── Plan ───
  - id: plan
    description: "Create implementation plan and test cases"
    config:
      skill: "/writing-plans"
    gate:
      - file_exists: "docs/plans/*-plan.md"
      - file_exists: "docs/plans/*-test-cases.md"

  - id: plan-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "plan"

  # ─── Plan Review ───
  - id: plan-review
    uses: ../implementation-review/pipeline.yml

  - id: plan-review-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "plan-review"

  # ─── Execute ───
  - id: execute
    description: "TDD implementation following the plan"
    config:
      skill: "/subagent-driven-development"
    max_retries: 3

  - id: execute-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "execute"

  # ─── Doc Audit (conditional) ───
  - id: doc-audit
    description: "4-layer document audit"
    when: "args.doc"
    config:
      skill: "/doc-audit"

  - id: doc-audit-audit
    when: "args.doc"
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "doc-audit"

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    config:
      skill: "/smoke-test"
    gate:
      - file_exists: "smoke-test-report.md"

  - id: smoke-test-audit
    when: "args.smoke"
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "smoke-test"

  # ─── Code Review ───
  - id: code-review
    uses: ../code-review/pipeline.yml

  - id: code-review-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "code-review"
    regate: [execute, smoke-test, doc-audit]

  # ─── Test Review (conditional) ───
  - id: test-review
    when: "args.e2e"
    uses: ../test-review/pipeline.yml

  - id: test-review-audit
    when: "args.e2e"
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "test-review"
    regate: [execute]

  # ─── Integrate ───
  - id: integrate
    description: "Merge, PR, or branch management"
    config:
      skill: "/worktrunk"
    validate:
      - "Integration method chosen and executed"
      - "All pre-merge checks pass"
    confirm: true
```

- [ ] **Step 2: Run lint**

Run: `target/debug/belt lint --config examples/skills/feature-dev/belt.toml`
Expected: `ok: examples/skills/feature-dev/pipeline.yml` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/pipeline.yml
git commit -m "refactor(examples): feature-dev audit phases use audit-gate sub-pipeline"
```

---

### Task 9: Modify debug-flow/pipeline.yml

**Files:**
- Modify: `examples/skills/debug-flow/pipeline.yml:22-155`

- [ ] **Step 1: Replace all 7 audit leaf phases with uses: references**

The full modified pipeline.yml:

```yaml
name: debug-flow
description: "Quality-gated debugging orchestrator"
version: 1
args:
  e2e: { type: bool, default: false }
  smoke: { type: bool, default: false }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm: { type: bool, default: false }

phases:
  # ─── Root Cause Analysis ───
  - id: rca
    description: "Investigate root cause via parallel exploration"
    config:
      skill: "/systematic-debugging"
      swarm: "args.swarm"
    gate:
      - file_exists: "docs/plans/*-rca-report.md"

  - id: rca-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "rca"

  # ─── Fix Plan ───
  - id: fix-plan
    description: "Create fix plan from RCA report"
    config:
      skill: "/writing-plans"
    gate:
      - file_exists: "docs/plans/*-fix-plan.md"

  - id: fix-plan-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "fix-plan"

  # ─── Fix Plan Review ───
  - id: fix-plan-review
    uses: ../implementation-review/pipeline.yml

  - id: fix-plan-review-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "fix-plan-review"

  # ─── Execute ───
  - id: execute
    description: "TDD implementation following the fix plan"
    config:
      skill: "/subagent-driven-development"
    max_retries: 3

  - id: execute-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "execute"

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    config:
      skill: "/smoke-test"
    gate:
      - file_exists: "smoke-test-report.md"

  - id: smoke-test-audit
    when: "args.smoke"
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "smoke-test"

  # ─── Code Review ───
  - id: code-review
    uses: ../code-review/pipeline.yml

  - id: code-review-audit
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "code-review"
    regate: [execute, smoke-test]

  # ─── Test Review (conditional) ───
  - id: test-review
    when: "args.e2e"
    uses: ../test-review/pipeline.yml

  - id: test-review-audit
    when: "args.e2e"
    uses: ../audit-gate/pipeline.yml
    config:
      criteria: "test-review"
    regate: [execute]

  # ─── Integrate ───
  - id: integrate
    description: "Merge, PR, or branch management"
    config:
      skill: "/worktrunk"
    validate:
      - "Integration method chosen and executed"
      - "All pre-merge checks pass"
    confirm: true
```

- [ ] **Step 2: Run lint**

Run: `target/debug/belt lint --config examples/skills/debug-flow/belt.toml`
Expected: `ok: examples/skills/debug-flow/pipeline.yml` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/debug-flow/pipeline.yml
git commit -m "refactor(examples): debug-flow audit phases use audit-gate sub-pipeline"
```

---

### Task 10: Update SKILL.md audit-protocol references

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md:25`
- Modify: `examples/skills/debug-flow/SKILL.md:28`

- [ ] **Step 1: Update feature-dev/SKILL.md**

In line 25, change:
```
per `references/audit-protocol.md`
```
to:
```
per `../audit-gate/references/audit-protocol.md`
```

- [ ] **Step 2: Update debug-flow/SKILL.md**

In line 28, change:
```
per `references/audit-protocol.md`
```
to:
```
per `../audit-gate/references/audit-protocol.md`
```

- [ ] **Step 3: Verify both references point to existing file**

Run: `ls examples/skills/audit-gate/references/audit-protocol.md`
Expected: file listed (exit 0).

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md examples/skills/debug-flow/SKILL.md
git commit -m "refactor(examples): update SKILL.md audit-protocol path to audit-gate"
```

---

### Task 11: Delete old audit-protocol.md files

**Files:**
- Delete: `examples/skills/feature-dev/references/audit-protocol.md`
- Delete: `examples/skills/debug-flow/references/audit-protocol.md`

- [ ] **Step 1: Delete the files**

```bash
git rm examples/skills/feature-dev/references/audit-protocol.md
git rm examples/skills/debug-flow/references/audit-protocol.md
```

- [ ] **Step 2: Verify no remaining references to deleted paths**

Run: `grep -r "references/audit-protocol.md" examples/skills/feature-dev/ examples/skills/debug-flow/`
Expected: no output (no remaining references to the old path — SKILL.md was updated in Task 10).

- [ ] **Step 3: Commit**

```bash
git commit -m "chore(examples): remove duplicate audit-protocol.md from consumers"
```

---

### Task 12: Final lint and expansion verification

**Files:** None (verification only)

- [ ] **Step 1: Lint both pipelines**

Run: `target/debug/belt lint --config examples/skills/feature-dev/belt.toml`
Expected: `ok: examples/skills/feature-dev/pipeline.yml` (exit 0).

Run: `target/debug/belt lint --config examples/skills/debug-flow/belt.toml`
Expected: `ok: examples/skills/debug-flow/pipeline.yml` (exit 0).

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p belt-core`
Expected: all tests pass (exit 0). This validates that the expander correctly handles the audit-gate sub-pipeline during lint's expansion trial.

- [ ] **Step 3: Verify expanded phase IDs**

Run: `cargo test -p belt-core expand 2>&1 | head -20`
Expected: expander tests pass. The existing expander tests confirm that `{parent_id}/{sub_phase_id}` naming works correctly for all sub-pipeline references.

- [ ] **Step 4: Spot-check audit-gate expansion for feature-dev**

Create a one-off test or use `belt-agent init` to verify the expanded phases:

Run: `target/debug/belt-agent init --config examples/skills/feature-dev/belt.toml --output-dir /tmp/belt-audit-gate-test 2>&1 | head -5`
Expected: JSON output showing successful initialization. The pipeline should expand without errors.

Run: `target/debug/belt-agent status --config examples/skills/feature-dev/belt.toml --output-dir /tmp/belt-audit-gate-test 2>&1 | grep "design-audit/check"`
Expected: phase `design-audit/check` appears in the status output, confirming the sub-pipeline expansion works.

Clean up: `rm -rf /tmp/belt-audit-gate-test`

- [ ] **Step 5: Spot-check audit-gate expansion for debug-flow**

Run: `target/debug/belt-agent init --config examples/skills/debug-flow/belt.toml --output-dir /tmp/belt-audit-gate-test2 2>&1 | head -5`
Expected: JSON output showing successful initialization.

Run: `target/debug/belt-agent status --config examples/skills/debug-flow/belt.toml --output-dir /tmp/belt-audit-gate-test2 2>&1 | grep "rca-audit/check"`
Expected: phase `rca-audit/check` appears in the status output.

Clean up: `rm -rf /tmp/belt-audit-gate-test2`
