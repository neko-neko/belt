# Review Sub-pipeline Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate 4 review child skills (spec-review, implementation-review, code-review, test-review) into belt sub-pipelines under `examples/skills/`, then wire feature-dev to reference them via `uses:`.

**Architecture:** Each review skill becomes an independent 2-phase pipeline (review → fix) with `config.agents` declaring subagent_types. Feature-dev references them via `uses:` and delegates dispatch to child SKILL.md. No belt-core changes.

**Tech Stack:** YAML (pipeline definitions), TOML (belt.toml), Markdown (SKILL.md)

**Spec:** `docs/specs/2026-04-07-review-sub-pipeline-migration.md`

---

### Task 1: Fix feature-dev belt.toml key name

**Files:**
- Modify: `examples/skills/feature-dev/belt.toml`

- [ ] **Step 1: Fix key name**

The current `belt.toml` uses `pipeline_file` but belt-core config.rs expects `pipeline`:

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Validate**

Run: `cargo run -p belt -- lint --config examples/skills/feature-dev/belt.toml`
Expected: lint passes (or errors unrelated to config key)

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/belt.toml
git commit -m "fix(example): correct belt.toml key name pipeline_file → pipeline"
```

---

### Task 2: Create spec-review sub-pipeline

**Files:**
- Create: `examples/skills/spec-review/belt.toml`
- Create: `examples/skills/spec-review/pipeline.yml`
- Create: `examples/skills/spec-review/SKILL.md`

- [ ] **Step 1: Create belt.toml**

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Create pipeline.yml**

```yaml
name: spec-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "4-perspective spec review"
    config:
      agents:
        - spec-review-requirements
        - spec-review-design-judgment
        - spec-review-feasibility
        - spec-review-consistency
      ui_agent: spec-review-ui-design
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

- [ ] **Step 3: Create SKILL.md**

```markdown
---
name: spec-review
description: >-
  4-perspective spec review pipeline. Dispatches requirements, design-judgment,
  feasibility, and consistency agents in parallel with N-way voting.
argument-hint: "[--codex] [--iterations N] [--ui] [--swarm]"
---

# Spec Review

4-perspective spec review with N-way voting and interactive dialogue resolution.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Add `config.ui_agent` if `args.ui`. Add Codex (`adversarial-review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → triage → present |
| `fix` phase | Dispatch `feature-implementer` with accepted findings. Modify spec bottom-up to prevent line-shift |

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (section-based):
- Match: same `section` + similar `description` (>80% semantic overlap)
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup

**Base selection**: iteration with most findings becomes the base set.
Subsequent iterations vote to confirm/deny each base finding.
Complementary findings (present in minority but absent from base) are added if unique.

## Triage

Categories: `requirements`, `design-judgment`, `feasibility`, `consistency`, `codex-adversarial`, `ui-design`

**Dialogue group** (interactive, max 3 rounds per finding):
- `requirements` findings with severity high or medium
- `design-judgment` findings with severity high or medium

Present each finding. Ask user for intent/context. Revise suggestion based on response.
After dialogue, user confirms revised suggestion or rejects.

**Selection group** (direct accept/reject):
- All other findings (feasibility, consistency, low-severity, codex, ui)

Present as numbered list sorted by severity descending. User selects which to fix.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user
- Ignore consensus vote results

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
```

- [ ] **Step 4: Validate with belt lint**

Run: `cargo run -p belt -- lint examples/skills/spec-review/pipeline.yml`
Expected: All checks pass

- [ ] **Step 5: Commit**

```bash
git add examples/skills/spec-review/
git commit -m "feat(example): add spec-review sub-pipeline

2-phase pipeline (review → fix) with config.agents declaring
4 subagent_types. Follows belt SKILL.md authoring principle."
```

---

### Task 3: Create implementation-review sub-pipeline

**Files:**
- Create: `examples/skills/implementation-review/belt.toml`
- Create: `examples/skills/implementation-review/pipeline.yml`
- Create: `examples/skills/implementation-review/SKILL.md`

- [ ] **Step 1: Create belt.toml**

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Create pipeline.yml**

```yaml
name: implementation-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "3-perspective plan review"
    config:
      agents:
        - implementation-review-clarity
        - implementation-review-feasibility
        - implementation-review-consistency
      ui_agent: implementation-review-ui-spec
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

- [ ] **Step 3: Create SKILL.md**

```markdown
---
name: implementation-review
description: >-
  3-perspective implementation plan review pipeline. Dispatches clarity,
  feasibility, and consistency agents in parallel with N-way voting.
argument-hint: "[--codex] [--iterations N] [--ui] [--swarm]"
---

# Implementation Review

3-perspective implementation plan review with N-way voting and interactive
dialogue resolution.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Add `config.ui_agent` if `args.ui`. Add Codex (`adversarial-review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → triage → present |
| `fix` phase | Dispatch `feature-implementer` with accepted findings. Modify plan bottom-up to prevent line-shift |

### Related Design Doc Detection

Before dispatching agents, detect the related design spec:
1. Extract date prefix from plan filename (e.g., `2026-04-07` from `2026-04-07-foo-plan.md`)
2. Find matching `docs/plans/<prefix>*-design.md`
3. Pass as `design_doc_path` context to the `consistency` agent

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (section-based):
- Match: same `section` + similar `description` (>80% semantic overlap)
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `impl-clarity`, `impl-feasibility`, `impl-consistency`, `codex-adversarial`, `impl-ui-spec`

**Dialogue group** (interactive, max 3 rounds per finding):
- `impl-clarity` findings with severity high or medium
- `impl-feasibility` findings with severity high

**Selection group** (direct accept/reject):
- All other findings

Present as numbered list sorted by severity descending. User selects which to fix.

## Verify (after fix)

1. `git diff` — confirm only target plan files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve
4. Design doc alignment — modified sections still reference correct design decisions

## Red Flags

**Never:**
- Modify plan without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user
- Ignore consensus vote results

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Pass related design doc to consistency agent
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
```

- [ ] **Step 4: Validate with belt lint**

Run: `cargo run -p belt -- lint examples/skills/implementation-review/pipeline.yml`
Expected: All checks pass

- [ ] **Step 5: Commit**

```bash
git add examples/skills/implementation-review/
git commit -m "feat(example): add implementation-review sub-pipeline

2-phase pipeline (review → fix) with config.agents declaring
3 subagent_types. Includes related design doc auto-detection."
```

---

### Task 4: Create code-review sub-pipeline

**Files:**
- Create: `examples/skills/code-review/belt.toml`
- Create: `examples/skills/code-review/pipeline.yml`
- Create: `examples/skills/code-review/SKILL.md`

- [ ] **Step 1: Create belt.toml**

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Create pipeline.yml**

```yaml
name: code-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "7-perspective code review"
    config:
      agents:
        - code-review-quality
        - code-review-security
        - code-review-performance
        - code-review-test
        - code-review-ai-antipattern
        - code-review-impact
      skills:
        - "/simplify"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

- [ ] **Step 3: Create SKILL.md**

```markdown
---
name: code-review
description: >-
  7-perspective code review pipeline. Dispatches 6 review agents + /simplify
  in parallel with N-way voting. File/line-based semantic similarity.
argument-hint: "[--codex] [--iterations N] [--swarm]"
---

# Code Review

7-perspective code review with N-way voting and direct selection triage.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Invoke each entry in `config.skills` via Skill tool. Add Codex (`review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → triage → present |
| `fix` phase | `simplify` findings → re-invoke `/simplify` for application. Other findings → dispatch `feature-implementer`. Serial modification to avoid conflicts |

### Scope Detection

Determine diff scope before dispatching agents:
1. If branch differs from main → `git diff main...HEAD`
2. If staged changes → `git diff --staged`
3. Pass diff summary as context to all agents

### /simplify Handling

`/simplify` is invoked via Skill tool (not Agent tool). Its output is free-text, not structured JSON.
Parse simplify output into findings format (file, description, suggestion).
Simplify findings are **not subject to N-way voting** — included directly after dedup.

### code-review-impact Context

If a design doc exists in the output directory (`*-design.md`), pass its Impact Analysis
section as additional context to the `code-review-impact` agent.

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (file/line-based):
- Match: same `file` + line within ±10 lines + similar `description`
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup
- Simplify findings: not voted, included directly

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `simplify`, `quality`, `security`, `performance`, `test`, `ai-antipattern`, `impact`, `codex`

**No dialogue phase.** All findings presented as numbered list sorted by severity descending.
User selects which to fix by number.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope
2. **Auto-detect and run project linter:**

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. **Auto-detect and run project tests:**

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

4. If linter or tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify code without user approval of findings
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Ignore consensus vote results
- Suppress or hide test/linter failures

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Run linter and tests after applying fixes
- Apply fixes serially to avoid merge conflicts in the same file
```

- [ ] **Step 4: Validate with belt lint**

Run: `cargo run -p belt -- lint examples/skills/code-review/pipeline.yml`
Expected: All checks pass

- [ ] **Step 5: Commit**

```bash
git add examples/skills/code-review/
git commit -m "feat(example): add code-review sub-pipeline

2-phase pipeline (review → fix) with 6 config.agents + /simplify
skill invoke. File/line-based voting, linter/test auto-detection."
```

---

### Task 5: Create test-review sub-pipeline

**Files:**
- Create: `examples/skills/test-review/belt.toml`
- Create: `examples/skills/test-review/pipeline.yml`
- Create: `examples/skills/test-review/SKILL.md`

- [ ] **Step 1: Create belt.toml**

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Create pipeline.yml**

```yaml
name: test-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "3-perspective test review"
    config:
      agents:
        - test-review-coverage
        - test-review-quality
        - test-review-design-alignment
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

- [ ] **Step 3: Create SKILL.md**

```markdown
---
name: test-review
description: >-
  3-perspective test review pipeline. Dispatches coverage, quality, and
  design-alignment agents in parallel with N-way voting. Produces requirement map.
argument-hint: "[--codex] [--iterations N] [--swarm]"
---

# Test Review

3-perspective test review with N-way voting, requirement mapping, and direct
selection triage.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `review` phase, `config.agents` present | Dispatch each agent via `Agent(subagent_type=<name>)` in parallel. Add Codex (`review` mode) if `config.codex`. If `config.swarm` → use TeamCreate. Collect → vote → build requirement map → triage → present |
| `fix` phase | Dispatch `feature-implementer` with accepted findings. Fix strategy varies by category (see below) |

### Design Spec Resolution

The `test-review-design-alignment` agent requires a design spec path:
1. Check output directory for `*-design.md`
2. If not found, check `docs/plans/*-design.md` with matching date prefix
3. Pass as `design_doc_path` context to the agent
4. If no design spec found → dispatch agent without it (reduced coverage)

### Requirement Map

The `design-alignment` agent returns a `requirement_map` in addition to findings:

| Column | Content |
|---|---|
| # | Sequential number |
| Requirement | Requirement from design spec |
| Source | Section in design spec |
| Test | Test file:line covering this requirement (or "—") |
| Gap | Missing coverage description (or "—") |

The requirement map is **informational only** — not subject to voting or selection.
Present it as a table in the review report. Gap entries inform coverage findings.

## Voting Protocol

Activated when `config.iterations` > 1. Each agent is dispatched N times independently.

**Semantic similarity** (file/line-based):
- Match: same `file` + line within ±10 lines + similar `description`
- Threshold: majority (>50% of iterations must agree)
- Codex findings: not voted, included if unique after dedup
- `requirement_map`: not voted, most detailed version is adopted

**Base selection**: iteration with most findings becomes the base set.

## Triage

Categories: `test-coverage`, `test-quality`, `test-design-alignment`, `codex`

**No dialogue phase.** All findings presented as numbered list sorted by severity descending.
User selects which to fix by number.

### Fix Strategy by Category

| Category | Fix action |
|---|---|
| `test-coverage` | Add new test cases for uncovered paths |
| `test-quality` | Improve existing test structure, naming, assertions |
| `test-design-alignment` | Add requirement-based tests from requirement map gaps |

## Verify (after fix)

1. `git diff` — confirm changes are test files only
2. **Auto-detect and run project tests:**

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

3. No linter step (test-only review)
4. If tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify production code (only test files)
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Ignore consensus vote results
- Classify test failures as acceptable without investigation

**Always:**
- Announce which agents are being dispatched and how many iterations
- Wait for all parallel agents to complete before voting
- Include requirement map in report even if no gaps found
- Run full test suite after applying fixes
```

- [ ] **Step 4: Validate with belt lint**

Run: `cargo run -p belt -- lint examples/skills/test-review/pipeline.yml`
Expected: All checks pass

- [ ] **Step 5: Commit**

```bash
git add examples/skills/test-review/
git commit -m "feat(example): add test-review sub-pipeline

2-phase pipeline (review → fix) with 3 config.agents.
File/line-based voting, requirement map from design-alignment."
```

---

### Task 6: Update feature-dev pipeline.yml to use sub-pipelines

**Files:**
- Modify: `examples/skills/feature-dev/pipeline.yml`

- [ ] **Step 1: Replace spec-review phase**

Change lines 36-42 from:

```yaml
  - id: spec-review
    description: "4-perspective spec review"
    config:
      skill: "/spec-review"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
```

To:

```yaml
  - id: spec-review
    uses: ../spec-review/pipeline.yml
```

- [ ] **Step 2: Replace plan-review phase**

Change lines 78-84 from:

```yaml
  - id: plan-review
    description: "3-perspective implementation plan review"
    config:
      skill: "/implementation-review"
      codex: "args.codex"
      iterations: "args.iterations"
      ui: "args.ui"
```

To:

```yaml
  - id: plan-review
    uses: ../implementation-review/pipeline.yml
```

- [ ] **Step 3: Replace code-review phase**

Change lines 162-168 from:

```yaml
  - id: code-review
    description: "7-perspective code review"
    config:
      skill: "/code-review"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
```

To:

```yaml
  - id: code-review
    uses: ../code-review/pipeline.yml
```

- [ ] **Step 4: Replace test-review phase**

Change lines 184-190 from:

```yaml
  - id: test-review
    description: "3-perspective test review"
    when: "args.e2e"
    config:
      skill: "/test-review"
      codex: "args.codex"
      iterations: "args.iterations"
```

To:

```yaml
  - id: test-review
    when: "args.e2e"
    uses: ../test-review/pipeline.yml
```

Note: `when: "args.e2e"` is preserved on the parent phase. The expander propagates it to all sub-phases.

- [ ] **Step 5: Validate with belt lint**

Run: `cargo run -p belt -- lint --config examples/skills/feature-dev/belt.toml`
Expected: All checks pass including sub-pipeline expansion

This validates:
- All `uses:` paths resolve correctly
- Sub-pipeline expansion produces valid phase IDs
- No duplicate phase IDs after expansion
- Regate targets still reference valid phase IDs

- [ ] **Step 6: Commit**

```bash
git add examples/skills/feature-dev/pipeline.yml
git commit -m "refactor(example): wire feature-dev to review sub-pipelines

Replace config.skill references with uses: for spec-review,
implementation-review, code-review, and test-review."
```

---

### Task 7: Update feature-dev SKILL.md with dispatch delegation

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Add sub-pipeline dispatch rule**

In the Dispatch Rules table, add a new row between the `config.skill` row and the `config.audit` row:

```markdown
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory (resolved from `uses:` path in pipeline.yml). Follow that SKILL.md's dispatch rules for the current sub-phase. Runtime args (`codex`, `iterations`, `swarm`, `ui`) come from top-level pipeline args |
```

The full updated Dispatch Rules table becomes:

```markdown
## Dispatch Rules

| config pattern | Action |
|---|---|
| `config.skill` present | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options (resolved from `args`) |
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory (resolved from `uses:` path in pipeline.yml). Follow that SKILL.md's dispatch rules for the current sub-phase. Runtime args (`codex`, `iterations`, `swarm`, `ui`) come from top-level pipeline args |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch `phase-auditor` subagent per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |
| `config.audit: "lite"` | Orchestrator directly evaluates `validate` criteria. `step --confirm` after user chooses integration method |
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "docs(example): add sub-pipeline dispatch delegation to feature-dev SKILL.md"
```

---

### Task 8: Final validation

- [ ] **Step 1: Lint all new pipelines individually**

```bash
cargo run -p belt -- lint examples/skills/spec-review/pipeline.yml
cargo run -p belt -- lint examples/skills/implementation-review/pipeline.yml
cargo run -p belt -- lint examples/skills/code-review/pipeline.yml
cargo run -p belt -- lint examples/skills/test-review/pipeline.yml
```

Expected: All pass

- [ ] **Step 2: Lint feature-dev with full expansion**

```bash
cargo run -p belt -- lint --config examples/skills/feature-dev/belt.toml
```

Expected: All pass. This validates the complete expanded pipeline including all sub-pipeline references.

- [ ] **Step 3: Verify expanded phase structure**

```bash
cargo run -p belt-agent -- init --config examples/skills/feature-dev/belt.toml --output-dir /tmp/belt-test-feature-dev
cargo run -p belt-agent -- status --state-dir /tmp/belt-test-feature-dev
```

Verify the status output shows the expected expanded phases:
- `spec-review/review`, `spec-review/fix`
- `plan-review/review`, `plan-review/fix`
- `code-review/review`, `code-review/fix`
- `test-review/review`, `test-review/fix`

And that audit phases remain as leaf phases:
- `spec-review-audit`, `plan-review-audit`, `code-review-audit`, `test-review-audit`

- [ ] **Step 4: Verify conditional phase expansion**

```bash
cargo run -p belt-agent -- init --config examples/skills/feature-dev/belt.toml --output-dir /tmp/belt-test-e2e --args '{"e2e": true}'
cargo run -p belt-agent -- status --state-dir /tmp/belt-test-e2e
```

Verify `test-review/review` and `test-review/fix` appear in the phase list (not skipped).

```bash
cargo run -p belt-agent -- init --config examples/skills/feature-dev/belt.toml --output-dir /tmp/belt-test-default --args '{}'
cargo run -p belt-agent -- status --state-dir /tmp/belt-test-default
```

Verify `test-review/review` and `test-review/fix` are skipped (when: args.e2e is false by default).

- [ ] **Step 5: Clean up**

```bash
rm -rf /tmp/belt-test-feature-dev /tmp/belt-test-e2e /tmp/belt-test-default
```

- [ ] **Step 6: Commit (if any fixes were needed)**

Only if validation found issues that required fixes in earlier tasks.
