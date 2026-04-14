# Feature-Dev Pipeline Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `examples/skills/feature-dev/` with a new 8-phase test-first quality-gated pipeline (design → test-scenarios → plan → execute → code-review → monkey-test → dogfood → integrate), creating 2 new Claude Code skills (`/test-scenarios`, `/monkey-test`) and applying the dotfiles supplement pattern for skill behavior overrides without modifying the underlying skills.

**Architecture:** 8 belt phases declared in `examples/skills/feature-dev/pipeline.yml`. Each phase uses `Invoker::Skill` to dispatch a global skill (/brainstorming, /test-scenarios, /writing-plans, /subagent-driven-development, /code-review, /monkey-test, /dogfood, /worktrunk). Skills marked with supplement reads (via `./references/*-supplement.md`) override output paths to `docs/features/<YYYY-MM-DD-topic>/` and inject pipeline context (design/test-strategy/scenarios/results) as hints to downstream skills. Conditional phases (monkey-test, dogfood) gate on `args.e2e`. Code-review regate targets `execute` with `max_retries: 3`.

**Tech Stack:**
- YAML (pipeline.yml, scenarios.yml, belt.toml)
- Markdown (SKILL.md, supplements, criteria, new-skill SKILL.md files)
- Rust (belt-core integration tests under `crates/belt-core/tests/`)
- belt CLI (`belt lint`) and belt-agent CLI for phase driving

**Implementation spec:** `docs/specs/2026-04-14-feature-dev-refresh-design.md`

---

## Prerequisites

- belt workspace at `~/go/src/github.com/neko-neko/belt`
- dotfiles workspace at `~/.dotfiles/claude/skills/` (VCS-tracked)
- `wt` (worktrunk) CLI available
- `gh` CLI authenticated
- Rust toolchain per `rust-toolchain.toml` (1.94.1+)

---

## File Structure

### belt repo (new / modified / deleted)

**New files** (`examples/skills/feature-dev/` replaces all existing):
```
examples/skills/feature-dev/
├── belt.toml                                 # NEW (replaces existing)
├── pipeline.yml                              # NEW (replaces existing)
├── SKILL.md                                  # NEW (replaces existing)
├── criteria/
│   ├── design.md                             # NEW
│   ├── test-scenarios.md                     # NEW
│   ├── plan.md                               # NEW (replaces existing)
│   ├── monkey-test.md                        # NEW
│   ├── dogfood.md                            # NEW
│   └── integrate.md                          # NEW
└── references/
    ├── path-convention.md                    # NEW
    ├── brainstorming-supplement.md           # NEW
    ├── writing-plans-supplement.md           # NEW
    ├── monkey-test-supplement.md             # NEW
    ├── dogfood-supplement.md                 # NEW
    └── worktrunk-supplement.md               # NEW
```

**Deleted** (during atomic cutover, after new content is verified):
```
examples/skills/feature-dev/criteria/spec-review.md       # DELETE (phase removed)
examples/skills/feature-dev/criteria/plan-review.md       # DELETE (phase removed)
examples/skills/feature-dev/criteria/doc-audit.md         # DELETE (phase removed)
examples/skills/feature-dev/references/evidence-plan-protocol.md   # DELETE (not used in new flow)
examples/skills/feature-dev/references/fix-dispatch-strategy.md    # DELETE (not used in new flow)
```

**Modified:**
```
crates/belt-core/tests/integration_feature_dev_refresh.rs  # NEW test file
docs/specs/2026-04-07-feature-dev-belt-migration.md        # add superseded note
```

### dotfiles repo (new skills)

```
~/.dotfiles/claude/skills/test-scenarios/
└── SKILL.md                                  # NEW

~/.dotfiles/claude/skills/monkey-test/
└── SKILL.md                                  # NEW
```

Each is symlinked to `~/.claude/skills/<name>` (dotfiles convention — see user's existing skills).

---

## Task 1: Worktree Setup

**Files:**
- None (workspace configuration)

- [ ] **Step 1: Create feature worktree for belt repo**

```bash
cd ~/go/src/github.com/neko-neko/belt
wt switch --create 2026-04-14-feature-dev-refresh -b main
```

Expected: New worktree created at a path like `~/go/src/github.com/neko-neko/belt-wt/2026-04-14-feature-dev-refresh`. Terminal is now inside the new worktree.

- [ ] **Step 2: Verify baseline build passes**

```bash
cargo build --workspace
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Verify baseline tests pass**

```bash
cargo test --workspace
```

Expected: All tests pass.

- [ ] **Step 4: Verify baseline lint passes**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: `Finished` with no warnings.

No commit yet — this is environment setup only.

---

## Task 2: Directory Scaffolding

**Files:**
- Create: `examples/skills/feature-dev/criteria/` (directory)
- Create: `examples/skills/feature-dev/references/` (directory)

- [ ] **Step 1: Remove old supplementary directories from existing feature-dev**

```bash
rm -rf examples/skills/feature-dev/criteria
rm -rf examples/skills/feature-dev/references
```

Expected: directories deleted. `ls examples/skills/feature-dev/` now shows only `belt.toml`, `pipeline.yml`, `SKILL.md`.

- [ ] **Step 2: Recreate empty directories**

```bash
mkdir -p examples/skills/feature-dev/criteria
mkdir -p examples/skills/feature-dev/references
```

Expected: Empty directories present. (The 3 top-level files remain from the old pipeline; they will be overwritten in later tasks.)

- [ ] **Step 3: Commit the scaffold state**

```bash
git add -A
git commit -m "chore(feature-dev): scaffold new criteria and references dirs"
```

Expected: Commit created. `git status` clean.

---

## Task 3: path-convention.md (SSOT for supplements)

**Files:**
- Create: `examples/skills/feature-dev/references/path-convention.md`

- [ ] **Step 1: Write required-sections test**

Run before creating the file:
```bash
test -f examples/skills/feature-dev/references/path-convention.md && echo "EXISTS" || echo "MISSING"
```
Expected: `MISSING`

- [ ] **Step 2: Create `path-convention.md`**

Write to `examples/skills/feature-dev/references/path-convention.md`:

````markdown
---
name: path-convention
description: >-
  Single source of truth for the docs/features/<YYYY-MM-DD-topic>/ directory
  naming and file layout used by all feature-dev phases.
---

# Path Convention for feature-dev Artifacts

All feature-dev artifact files live under `docs/features/<YYYY-MM-DD-topic>/`.
Every supplement references this document for naming rules.

## Directory Name

`docs/features/<YYYY-MM-DD-topic>/`

- `<YYYY-MM-DD>`: the date Phase 1 (design) is first invoked, in UTC (ISO 8601).
- `<topic>`: a kebab-case slug (lowercase letters, digits, hyphens; no spaces,
  no underscores). Chosen interactively with the user during Phase 1.

Examples:
- `docs/features/2026-04-14-user-authentication/`
- `docs/features/2026-05-01-payment-refactor/`

## Topic Slug Rules

- Only `[a-z0-9-]`, no leading/trailing hyphens, no consecutive hyphens.
- Minimum 3 characters, maximum 48 characters.
- Must not collide with an existing directory under `docs/features/`.
- Must be stable for the duration of the feature (do not rename mid-flight).

If a collision is detected, Phase 1 supplement appends `-N` (e.g. `-2`) until
unique.

## Worktree Branch Correspondence

The worktree branch name created in Phase 1 must match:

```
feature/<YYYY-MM-DD-topic>
```

Example: directory `docs/features/2026-04-14-user-authentication/` maps to
branch `feature/2026-04-14-user-authentication`.

## File Layout per Feature

| File | Phase | Producer | When |
|------|-------|----------|------|
| `design.md` | 1 | /brainstorming (+ brainstorming-supplement) | always |
| `test-strategy.md` | 2 | /test-scenarios | always |
| `scenarios.yml` | 2 | /test-scenarios | when `args.e2e` |
| `plan.md` | 3 | /writing-plans (+ writing-plans-supplement) | always |
| `monkey-test-report.md` | 6 | /monkey-test | when `args.e2e` |
| `monkey-test-results.json` | 6 | /monkey-test | when `args.e2e` |
| `dogfood-report/report.md` | 7 | /dogfood (+ dogfood-supplement) | when `args.e2e` |
| `dogfood-report/screenshots/*` | 7 | /dogfood | when `args.e2e` |
| `dogfood-report/videos/*` | 7 | /dogfood | when `args.e2e` |

Phase 4 (execute) and Phase 5 (code-review) write to git history and
`.belt/runs/*/review/findings.json`, not under `docs/features/`.

Phase 8 (integrate) consumes from `docs/features/<topic>/` but does not write
there.

## Editing Rules

- Phases generate these files; do not hand-edit.
- Hand-edits break belt's phase-start mtime filter (BELT-32 DD-1) used for
  artifact glob resolution.
- If a correction is needed, re-run the owning phase (verify → regate → step).
````

- [ ] **Step 3: Verify structural requirements**

```bash
test -f examples/skills/feature-dev/references/path-convention.md && \
  grep -q "^## Directory Name" examples/skills/feature-dev/references/path-convention.md && \
  grep -q "^## Topic Slug Rules" examples/skills/feature-dev/references/path-convention.md && \
  grep -q "^## File Layout per Feature" examples/skills/feature-dev/references/path-convention.md && \
  echo "OK" || echo "FAIL"
```
Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/references/path-convention.md
git commit -m "feat(feature-dev): add path-convention.md SSOT"
```

---

## Task 4: brainstorming-supplement.md

**Files:**
- Create: `examples/skills/feature-dev/references/brainstorming-supplement.md`

- [ ] **Step 1: Create file**

Write to `examples/skills/feature-dev/references/brainstorming-supplement.md`:

````markdown
---
name: brainstorming-supplement
description: >-
  feature-dev Phase 1 only. Read BEFORE invoking superpowers:brainstorming to
  inject path override, parallel codebase exploration, implicit rules
  extraction, required design sections, and worktree creation order.
---

# Brainstorming Supplement for feature-dev

This supplement is Read into context BEFORE `/brainstorming` is invoked by
feature-dev Phase 1. Once loaded, the constraints below override/augment the
standard brainstorming flow.

Path convention reference: `./path-convention.md`.

## Output Path Override

The final design document MUST be written to:

```
docs/features/<YYYY-MM-DD-topic>/design.md
```

This overrides brainstorming's default `docs/superpowers/specs/` location.
Topic slug selection follows `./path-convention.md`.

## Interactive Execution Constraints

- Do NOT delegate brainstorming steps to subagents via TaskCreate.
- Ask every question directly to the user; do not auto-answer.
- One question at a time.

## Added Steps (inserted between brainstorming step 2 and step 3)

After clarifying questions complete, BEFORE proposing 2-3 approaches, execute
S1 through S4.

### S1: Parallel Codebase Exploration

From the clarifying answers, derive three exploration prompts and launch three
Agent calls in a SINGLE message:

1. `code-explorer` — trace existing code flow related to the feature area;
   report dependencies, patterns, constraints.
2. `code-architect` — identify architecture patterns, conventions, reusable
   components in the feature area.
3. `impact-analyzer` — reverse-trace the change target; report shared state,
   implicit contracts, side-effect risks.

If any agent fails, fall back to Grep/Read in the main context for that area
only. Use successful results even if partial.

### S2: Implicit Rules Extraction

From S1 findings, enumerate:

- Validation rules (value ranges, required-field checks)
- Conditional branches (status-based routing)
- Business logic (formulas, permission checks)

Confirm each rule with the user ONE AT A TIME:

> "The existing code has [rule]. Does this constraint apply to the new feature?"

Do not proceed until all rules are confirmed.

### S3: Investigation Record

Record S1/S2 findings in the design document under these required sections:

- **Prerequisites** — existing constraints/rules the new feature depends on.
- **Impact Scope** — modules/tables that may be affected.
- **Impact Analysis**
  - **Reverse Dependencies** — callers of the change target (file:line, strength).
  - **Shared State** — shared resources (kind, constraint, usage).
  - **Implicit Contracts** — implicit invariants (file:line, dependency, violation impact).
  - **Side Effect Risks** — side-effect scenarios (severity, trigger, impact).
- **Must-Verify Checklist** — items to confirm during implementation/testing.
  Later phases (plan, code-review, dogfood) consume this checklist directly.

### S4: Test Perspectives

Add a "Test Perspectives" section to the design document containing:

- Normal-case scenarios (named).
- Boundary / error-case scenarios (named).
- Non-functional requirements (performance, security, accessibility).

Quality bar (applied in Phase 3 when expanding to Given/When/Then):
For every input parameter, cover at minimum one case from EACH of:
- Normal (representative value)
- Boundary (min, max, exactly at boundary)
- Abnormal (wrong type, null/undefined, out of range)
- State transition (different preconditions)

Cases failing to meet this bar will be rejected by Phase 2 (test-scenarios)
and Phase 5 (code-review) review.

## Workspace Creation (before committing design document)

Before committing `design.md`:

1. Invoke `worktrunk:worktrunk` with `wt switch -c feature/<YYYY-MM-DD-topic>`
   (base = current branch at Phase 1 start; resume from handover if set).
2. Let worktrunk's pre-start hook install dependencies.
3. Run baseline tests.
   - Pass → commit `design.md` inside the new worktree.
   - Fail → PAUSE; ask user whether to continue or stop.
4. All subsequent phases operate inside this worktree.
5. Record worktree path and branch name; the belt-agent will read these from
   git at verify-time.

## Completion Criteria (for Phase 1 gate)

- `docs/features/<YYYY-MM-DD-topic>/design.md` exists, committed in the worktree.
- All required sections (Prerequisites / Impact Scope / Impact Analysis /
  Must-Verify Checklist / Test Perspectives) are populated.
- Test Perspectives meet the quality bar (all four categories per input).
- Worktree `feature/<YYYY-MM-DD-topic>` exists and baseline tests passed.
````

- [ ] **Step 2: Verify required sections**

```bash
grep -c "^## " examples/skills/feature-dev/references/brainstorming-supplement.md
```
Expected: at least `6` (Output Path Override, Interactive Execution Constraints, Added Steps, Workspace Creation, Completion Criteria, plus sub-headings).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/references/brainstorming-supplement.md
git commit -m "feat(feature-dev): add brainstorming-supplement for Phase 1"
```

---

## Task 5: writing-plans-supplement.md

**Files:**
- Create: `examples/skills/feature-dev/references/writing-plans-supplement.md`

- [ ] **Step 1: Create file**

Write to `examples/skills/feature-dev/references/writing-plans-supplement.md`:

````markdown
---
name: writing-plans-supplement
description: >-
  feature-dev Phase 3 only. Read BEFORE invoking superpowers:writing-plans to
  override input paths, output path, and test-case expansion rules.
---

# Writing-Plans Supplement for feature-dev

Read BEFORE invoking `/writing-plans` in Phase 3. Path convention reference:
`./path-convention.md`.

## Input Paths (read these)

- `docs/features/<topic>/design.md` — the feature design
- `docs/features/<topic>/test-strategy.md` — test perspectives + strategy
- `docs/features/<topic>/scenarios.yml` — when `args.e2e` is true

## Output Path Override

Write the plan to:

```
docs/features/<topic>/plan.md
```

This overrides writing-plans' default `docs/superpowers/plans/` location.

## Plan Content Requirements

Beyond the standard writing-plans output:

1. **Test-case integration**: Every task that implements a feature requirement
   MUST reference at least one entry from `test-strategy.md` and, when e2e is
   enabled, at least one `scenarios.yml` `id` that exercises the feature.

2. **Must-Verify Checklist mapping**: Every item in the design's
   Must-Verify Checklist MUST map to at least one task that verifies it
   (cite the item ID in the task).

3. **Given/When/Then expansion**: For every input parameter surfaced in
   `test-strategy.md`, include Given/When/Then tests covering the four
   categories (normal, boundary, abnormal, state-transition).

4. **No placeholders**: per the writing-plans standard — no TBD/TODO/"add
   appropriate error handling". Show actual code in every step.

## When args.e2e is true

- Each scenario in `scenarios.yml` must be referenced by at least one plan
  task (either implementation or verification).
- Plan tasks that produce UI-bearing code must include a step that verifies
  the UI can be reached by the corresponding `scenarios.yml` scenario's
  `given` preconditions.

## Completion Criteria (for Phase 3 gate)

- `docs/features/<topic>/plan.md` exists, committed in the worktree.
- All four plan-content requirements above are satisfied.
- Plan links every Must-Verify Checklist item to at least one task.
- When e2e: plan links every `scenarios.yml` `id` to at least one task.
````

- [ ] **Step 2: Verify**

```bash
grep -c "^## " examples/skills/feature-dev/references/writing-plans-supplement.md
```
Expected: at least `5`.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/references/writing-plans-supplement.md
git commit -m "feat(feature-dev): add writing-plans-supplement for Phase 3"
```

---

## Task 6: monkey-test-supplement.md

**Files:**
- Create: `examples/skills/feature-dev/references/monkey-test-supplement.md`

- [ ] **Step 1: Create file**

Write to `examples/skills/feature-dev/references/monkey-test-supplement.md`:

````markdown
---
name: monkey-test-supplement
description: >-
  feature-dev Phase 6 only. Read BEFORE invoking /monkey-test to inject
  design/plan/test-strategy as interpretation hints and to fix output paths.
---

# Monkey-Test Supplement for feature-dev

Read BEFORE invoking `/monkey-test` in Phase 6. This phase only runs when
`args.e2e` is true. Path convention reference: `./path-convention.md`.

## Primary Input

- `docs/features/<topic>/scenarios.yml` — the scripted scenarios (required)

## Hint Inputs (read these into context)

- `docs/features/<topic>/design.md`
  - Use to resolve ambiguity in scenarios' natural-language Given/When/Then
    (e.g., "valid email" → use the exact validation rule from design).
  - Use Impact Analysis to predict likely regressions.
- `docs/features/<topic>/test-strategy.md`
  - Use `category`/`severity` from each scenario's matching strategy entry
    to set the failure severity in results.
- `docs/features/<topic>/plan.md`
  - Use to decide `SKIP` verdicts: if a scenario targets a feature whose
    implementing task is marked incomplete in the plan, SKIP the scenario
    and note the reason.

## Output Paths

- `docs/features/<topic>/monkey-test-report.md` — human-readable
- `docs/features/<topic>/monkey-test-results.json` — machine-readable

## Behavior

1. Parse `scenarios.yml`; collect `id`, `given`, `when`, `then`, `severity`.
2. For each scenario:
   a. Determine SKIP if an associated plan task is incomplete.
   b. Launch agent-browser (restore auth-state if present).
   c. Interpret `given` → navigate/setup; `when` → actions; `then` →
      assertions. Resolve ambiguity via `design.md`.
   d. Capture a screenshot at each step (save under
      `docs/features/<topic>/monkey-test-screenshots/` — create if missing).
   e. Record result.
3. After all scenarios, write both outputs.

## results.json Schema

```json
{
  "scenarios": [
    {
      "id": "string",
      "status": "PASS | FAIL | SKIP",
      "severity": "critical | high | medium | low",
      "duration_ms": 1234,
      "error": "string (only when FAIL)",
      "skip_reason": "string (only when SKIP)",
      "screenshots": ["docs/features/<topic>/monkey-test-screenshots/<id>-step1.png", "..."]
    }
  ],
  "summary": {
    "total": 10,
    "passed": 8,
    "failed": 1,
    "skipped": 1
  }
}
```

## Completion Criteria (for Phase 6 gate)

- Both output files exist and are committed.
- Every scenario id in `scenarios.yml` is present in `results.json.scenarios`.
- `results.json` validates against the schema above.
- Every FAIL with severity `critical` or `high` is surfaced in the primary
  section of `monkey-test-report.md`.
````

- [ ] **Step 2: Verify**

```bash
grep -q "^## Primary Input" examples/skills/feature-dev/references/monkey-test-supplement.md && \
  grep -q "^## Output Paths" examples/skills/feature-dev/references/monkey-test-supplement.md && \
  grep -q "^## results.json Schema" examples/skills/feature-dev/references/monkey-test-supplement.md && \
  echo "OK" || echo "FAIL"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/references/monkey-test-supplement.md
git commit -m "feat(feature-dev): add monkey-test-supplement for Phase 6"
```

---

## Task 7: dogfood-supplement.md

**Files:**
- Create: `examples/skills/feature-dev/references/dogfood-supplement.md`

- [ ] **Step 1: Create file**

Write to `examples/skills/feature-dev/references/dogfood-supplement.md`:

````markdown
---
name: dogfood-supplement
description: >-
  feature-dev Phase 7 only. Read BEFORE invoking /dogfood to override output
  directory, scope exploration to the feature diff, filter severity, and
  inject prior-phase artifacts as exploration hints.
---

# Dogfood Supplement for feature-dev

Read BEFORE invoking `/dogfood` in Phase 7. This phase only runs when
`args.e2e` is true. Path convention reference: `./path-convention.md`.

## Output Path Override

```
docs/features/<topic>/dogfood-report/
├── report.md
├── screenshots/
└── videos/
```

This overrides /dogfood's default `./dogfood-output/`.

## Scope Override

Restrict exploration to code areas changed by the feature branch:

```bash
git diff <base>..HEAD --name-only
```

Map changed files to corresponding UI pages/components; prioritize those.
Do NOT explore the full site.

## Severity Filter

- `critical` and `high` issues: full detail in report.md primary section.
- `medium` and `low` issues: summary only (counts + one-line description).

## Context Injection (read these BEFORE starting exploration)

### 1. `docs/features/<topic>/design.md`
Focus on:
- **Prerequisites** — a violation is a likely bug.
- **Impact Scope** — modules where side-effects may surface.
- **Impact Analysis > Side Effect Risks** — attempt to reproduce each risk.
- **Must-Verify Checklist** — VERIFY EVERY ITEM during dogfood.

### 2. `docs/features/<topic>/test-strategy.md`
Focus on:
- **Non-functional requirements** (performance, security, accessibility) —
  these are typically uncovered by scripted tests.
- **Boundary / state-transition** items requiring exotic combinations.

### 3. `docs/features/<topic>/scenarios.yml`
Use to AVOID redundant exploration of scripted happy paths. Spend effort on
combinations NOT in scenarios.yml (e.g., scenario A then B, mid-flow
interrupt, concurrent operations, long-idle resumes).

### 4. `docs/features/<topic>/monkey-test-results.json`
- Read all `FAIL` entries. Retry each by hand.
  - Still broken → file as "Known issue re-encountered" (do not double-count
    as a new finding).
  - Fixed → note as "Previously failed, now passing".
- Read all `SKIP` entries. Verify that the SKIP reason still holds in the
  current build.

### 5. `docs/features/<topic>/plan.md`
Read for context on implementation scope (do not re-verify every task).

## Exploration Priority

1. Verify every item in the Must-Verify Checklist from `design.md`.
2. Attempt to reproduce every Side Effect Risk from Impact Analysis.
3. Exercise non-functional requirements from `test-strategy.md`.
4. Combinations and exotic cases not covered by `scenarios.yml`.
5. Surface UI/UX bugs (typos, misalignment, console errors, a11y).

Scripted happy paths: verify existence only (smoke confirm), do not deep-test.

## Report Structure

```markdown
# Dogfood Report: <feature-name>

## Summary
- Exploration time: XX min
- Pages visited: N
- New issues found: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N (from monkey-test-results.json)
- Must-Verify Checklist: X/Y items verified (list any unverified)

## Critical and High Issues (new findings)
<per-issue: id, severity, repro steps, screenshot/video evidence>

## Must-Verify Checklist Verification
<table: item, status (PASS/FAIL/N/A), notes>

## Known Issues Re-encountered
<per-issue: scenarios.yml id, status from monkey-test, dogfood observation>

## Medium and Low Issues (summary)
<counts plus one-line descriptions>
```

## Completion Criteria (for Phase 7 gate)

- `docs/features/<topic>/dogfood-report/report.md` exists and is committed.
- Every Must-Verify Checklist item has a verification status in report.md.
- Every `FAIL` scenario in `monkey-test-results.json` is addressed in
  "Known Issues Re-encountered".
- Either ≥ 5 new issues are well-documented with evidence, OR the report
  explicitly states "No critical or high issues found" with rationale.
````

- [ ] **Step 2: Verify**

```bash
grep -q "^## Output Path Override" examples/skills/feature-dev/references/dogfood-supplement.md && \
  grep -q "^## Context Injection" examples/skills/feature-dev/references/dogfood-supplement.md && \
  grep -q "^## Exploration Priority" examples/skills/feature-dev/references/dogfood-supplement.md && \
  grep -q "^## Report Structure" examples/skills/feature-dev/references/dogfood-supplement.md && \
  echo "OK" || echo "FAIL"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/references/dogfood-supplement.md
git commit -m "feat(feature-dev): add dogfood-supplement with prior-phase context"
```

---

## Task 8: worktrunk-supplement.md

**Files:**
- Create: `examples/skills/feature-dev/references/worktrunk-supplement.md`

- [ ] **Step 1: Create file**

Write to `examples/skills/feature-dev/references/worktrunk-supplement.md`:

````markdown
---
name: worktrunk-supplement
description: >-
  feature-dev Phase 8 only. Read BEFORE invoking /worktrunk to define the
  merge-vs-PR user choice flow and the PR-body template.
---

# Worktrunk Supplement for feature-dev (Phase 8 Integrate)

Read BEFORE invoking `/worktrunk` in Phase 8.

## Required User Prompt

At the start of Phase 8, present exactly:

```
Select integration mode:
(A) merge — run `wt merge` to parent branch, then `wt remove` this worktree
(B) PR    — run `gh pr create` with an auto-generated body, keep worktree
```

Wait for explicit (A) or (B). Do not proceed on any other input.

## (A) Merge Flow

1. Ensure worktree is clean (`git status` shows no uncommitted changes).
2. Invoke `/worktrunk` with `wt merge`. This runs the project's pre-merge
   hook (typically tests + build). Abort if the hook fails.
3. After a successful fast-forward merge, invoke `wt remove` to delete the
   worktree.
4. Record the merged commit SHA.

## (B) PR Flow

1. Ensure worktree is clean.
2. Push the branch to origin (`git push -u origin <branch>`).
3. Run `gh pr create --title "<title>" --body "<body>"` with:
   - title: `feat: <topic>` (from `docs/features/<topic>/` directory name;
     strip the date prefix)
   - body: use the template below.
4. Record the PR URL.
5. Do NOT run `wt merge` or `wt remove`.

## PR Body Template

```markdown
## Summary

<One paragraph from the "Summary" or opening section of design.md.>

## Changes

<Bulleted list of task titles from plan.md, Task 1..N.>

## Testing

### Code Review
- Findings: { critical: N, high: N, medium: N, low: N }
- Status: (all addressed | N outstanding — see comments)

### Monkey Test (when args.e2e)
- Scenarios: <total>; Passed <n>; Failed <n>; Skipped <n>
- Link: docs/features/<topic>/monkey-test-report.md

### Dogfood (when args.e2e)
- New issues: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N
- Link: docs/features/<topic>/dogfood-report/report.md

## Must-Verify Checklist

<Table copied from dogfood-report.md's "Must-Verify Checklist Verification"
section. If args.e2e is false, copy directly from design.md with an
"Unverified (no e2e run)" note.>

## Spec and Plan

- Spec: <link to design.md at the merged SHA>
- Plan: <link to plan.md at the merged SHA>
```

## Completion Criteria (for Phase 8 gate)

- User explicitly selected (A) or (B).
- (A): merge commit exists in parent branch; worktree removed.
- (B): PR URL exists; body populated from template (no template placeholder
  text like `<...>` remains in the published body).
````

- [ ] **Step 2: Verify**

```bash
grep -q "^## Required User Prompt" examples/skills/feature-dev/references/worktrunk-supplement.md && \
  grep -q "^## (A) Merge Flow" examples/skills/feature-dev/references/worktrunk-supplement.md && \
  grep -q "^## (B) PR Flow" examples/skills/feature-dev/references/worktrunk-supplement.md && \
  grep -q "^## PR Body Template" examples/skills/feature-dev/references/worktrunk-supplement.md && \
  echo "OK" || echo "FAIL"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/references/worktrunk-supplement.md
git commit -m "feat(feature-dev): add worktrunk-supplement with A/B choice"
```

---

## Task 9: criteria/design.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/design.md`

- [ ] **Step 1: Create file**

````markdown
---
name: design-done-criteria
audit: lite
phase: design
---

# Phase 1 (design) Done Criteria

All items must be satisfied for the phase to pass.

- **DESIGN-01**: `docs/features/<topic>/design.md` exists and is committed
  inside the feature worktree.
- **DESIGN-02**: The document contains all required sections:
  - `Prerequisites`
  - `Impact Scope`
  - `Impact Analysis` with subsections
    `Reverse Dependencies`, `Shared State`, `Implicit Contracts`,
    `Side Effect Risks`
  - `Must-Verify Checklist`
  - `Test Perspectives`
- **DESIGN-03**: `Test Perspectives` covers at minimum one case for EACH of:
  normal, boundary, abnormal, state-transition.
- **DESIGN-04**: Worktree branch `feature/<YYYY-MM-DD-topic>` exists
  (verify with `git branch --list`).
- **DESIGN-05**: Baseline tests pass in the worktree at the time of
  `design.md` commit (verify via the worktrunk pre-start hook output or a
  fresh `cargo test` / project-appropriate test command).
- **DESIGN-06**: `git status` in the worktree is clean after the `design.md`
  commit.
````

- [ ] **Step 2: Verify all IDs are present**

```bash
for id in DESIGN-01 DESIGN-02 DESIGN-03 DESIGN-04 DESIGN-05 DESIGN-06; do
  grep -q "$id" examples/skills/feature-dev/criteria/design.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: Only `DONE` printed (no MISSING lines).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/design.md
git commit -m "feat(feature-dev): add Phase 1 design done-criteria"
```

---

## Task 10: criteria/test-scenarios.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/test-scenarios.md`

- [ ] **Step 1: Create file**

````markdown
---
name: test-scenarios-done-criteria
audit: lite
phase: test-scenarios
---

# Phase 2 (test-scenarios) Done Criteria

- **TEST-01**: `docs/features/<topic>/test-strategy.md` exists and is committed.
- **TEST-02**: `test-strategy.md` contains sections:
  - `Test Design Techniques` (ISTQB-based: equivalence partitioning,
    boundary-value analysis, decision tables, state transitions)
  - `Quality Characteristics` (ISO 25010-based: functional suitability,
    performance efficiency, compatibility, usability, reliability, security,
    maintainability, portability)
  - `Priority Matrix` mapping characteristics to criticality
- **TEST-03**: Every item in `design.md`'s `Must-Verify Checklist` has at
  least one corresponding entry in `test-strategy.md` (verified by ID cross-
  reference).
- **TEST-04**: When `args.e2e` is true:
  - `docs/features/<topic>/scenarios.yml` exists, committed.
  - Contains at least 3 scenarios.
  - Every scenario has: `id` (kebab-case), `category`, `severity`
    (`critical|high|medium|low`), `given`, `when`, `then`.
  - `preconditions` and `postconditions` are present when applicable.
- **TEST-05**: `test-strategy.md` includes at least one non-functional
  requirement (performance, security, or accessibility) with a concrete
  acceptance criterion.
````

- [ ] **Step 2: Verify IDs**

```bash
for id in TEST-01 TEST-02 TEST-03 TEST-04 TEST-05; do
  grep -q "$id" examples/skills/feature-dev/criteria/test-scenarios.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/test-scenarios.md
git commit -m "feat(feature-dev): add Phase 2 test-scenarios done-criteria"
```

---

## Task 11: criteria/plan.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/plan.md` (replaces old one)

- [ ] **Step 1: Create file**

````markdown
---
name: plan-done-criteria
audit: lite
phase: plan
---

# Phase 3 (plan) Done Criteria

- **PLAN-01**: `docs/features/<topic>/plan.md` exists and is committed.
- **PLAN-02**: `plan.md` contains the required header:
  `Goal`, `Architecture`, `Tech Stack`, and at least one `Task N` section.
- **PLAN-03**: Every `Task` follows the TDD shape (failing test → minimal
  implementation → passing test → commit) with explicit code and commands
  per step.
- **PLAN-04**: Every item in `design.md`'s `Must-Verify Checklist` is cited
  by at least one task (cite ID, e.g. `MV-01`).
- **PLAN-05**: No placeholder language remains (no `TBD`, `TODO`,
  `add appropriate error handling`, `similar to Task N`, or unresolved
  types/functions).
- **PLAN-06**: When `args.e2e` is true, every `scenarios.yml` `id` is
  cited by at least one task.
- **PLAN-07**: Every input parameter surfaced in `test-strategy.md` has
  Given/When/Then coverage for: normal, boundary, abnormal, state-transition.
````

- [ ] **Step 2: Verify IDs**

```bash
for id in PLAN-01 PLAN-02 PLAN-03 PLAN-04 PLAN-05 PLAN-06 PLAN-07; do
  grep -q "$id" examples/skills/feature-dev/criteria/plan.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/plan.md
git commit -m "feat(feature-dev): rewrite Phase 3 plan done-criteria"
```

---

## Task 12: criteria/monkey-test.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/monkey-test.md`

- [ ] **Step 1: Create file**

````markdown
---
name: monkey-test-done-criteria
audit: lite
phase: monkey-test
---

# Phase 6 (monkey-test) Done Criteria

(Only evaluated when `args.e2e` is true; Phase is skipped otherwise.)

- **MONKEY-01**: `docs/features/<topic>/monkey-test-report.md` exists and is
  committed.
- **MONKEY-02**: `docs/features/<topic>/monkey-test-results.json` exists and
  validates against the schema in
  `references/monkey-test-supplement.md`.
- **MONKEY-03**: Every scenario `id` from `scenarios.yml` has a matching
  entry in `results.json.scenarios` with status `PASS`, `FAIL`, or `SKIP`.
- **MONKEY-04**: Every FAIL whose severity is `critical` or `high` is
  described in detail in `monkey-test-report.md`'s primary section with
  expected-vs-actual and at least one screenshot.
- **MONKEY-05**: `SKIP` entries include a non-empty `skip_reason` referencing
  the `plan.md` task that is incomplete.
````

- [ ] **Step 2: Verify IDs**

```bash
for id in MONKEY-01 MONKEY-02 MONKEY-03 MONKEY-04 MONKEY-05; do
  grep -q "$id" examples/skills/feature-dev/criteria/monkey-test.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/monkey-test.md
git commit -m "feat(feature-dev): add Phase 6 monkey-test done-criteria"
```

---

## Task 13: criteria/dogfood.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/dogfood.md`

- [ ] **Step 1: Create file**

````markdown
---
name: dogfood-done-criteria
audit: lite
phase: dogfood
---

# Phase 7 (dogfood) Done Criteria

(Only evaluated when `args.e2e` is true; Phase is skipped otherwise.)

- **DOGFOOD-01**: `docs/features/<topic>/dogfood-report/report.md` exists
  and is committed.
- **DOGFOOD-02**: Every item in `design.md`'s `Must-Verify Checklist` has
  a verification status (`PASS`, `FAIL`, `N/A`) in
  `dogfood-report/report.md` under
  `Must-Verify Checklist Verification`.
- **DOGFOOD-03**: Every `FAIL` scenario in `monkey-test-results.json` is
  addressed in the `Known Issues Re-encountered` section of the report.
- **DOGFOOD-04**: Either:
  - ≥ 5 new issues are documented with severity, reproduction steps, and
    evidence (screenshot/video path under `screenshots/` or `videos/`), OR
  - The report explicitly states "No critical or high issues found" with
    a rationale paragraph.
- **DOGFOOD-05**: The report's `Summary` section counts (new issues by
  severity, known issues re-encountered, must-verify coverage) are
  consistent with the detail sections.
````

- [ ] **Step 2: Verify**

```bash
for id in DOGFOOD-01 DOGFOOD-02 DOGFOOD-03 DOGFOOD-04 DOGFOOD-05; do
  grep -q "$id" examples/skills/feature-dev/criteria/dogfood.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/dogfood.md
git commit -m "feat(feature-dev): add Phase 7 dogfood done-criteria"
```

---

## Task 14: criteria/integrate.md

**Files:**
- Create: `examples/skills/feature-dev/criteria/integrate.md`

- [ ] **Step 1: Create file**

````markdown
---
name: integrate-done-criteria
audit: lite
phase: integrate
---

# Phase 8 (integrate) Done Criteria

- **INT-01**: User explicitly selected (A) merge or (B) PR per the
  `references/worktrunk-supplement.md` prompt.
- **INT-02** (A selected): a merge commit containing the feature branch
  exists on the parent branch, the pre-merge hook succeeded, and the
  worktree has been removed (`wt list` does not list
  `feature/<YYYY-MM-DD-topic>`).
- **INT-03** (B selected): a PR exists on origin with a non-empty body
  whose sections (`Summary`, `Changes`, `Testing`, `Must-Verify Checklist`,
  `Spec and Plan`) are populated from the template with no literal
  `<...>` placeholder remaining.
- **INT-04**: All phase produces artifacts for the feature are:
  - (A) present in the parent branch at the merge commit, OR
  - (B) present at the PR head commit.
- **INT-05**: No uncommitted changes remain in the worktree (A only
  applicable before the `wt remove`).
````

- [ ] **Step 2: Verify**

```bash
for id in INT-01 INT-02 INT-03 INT-04 INT-05; do
  grep -q "$id" examples/skills/feature-dev/criteria/integrate.md || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/feature-dev/criteria/integrate.md
git commit -m "feat(feature-dev): add Phase 8 integrate done-criteria"
```

---

## Task 15: pipeline.yml

**Files:**
- Modify (replace): `examples/skills/feature-dev/pipeline.yml`

- [ ] **Step 1: Back up the old pipeline for reference**

```bash
cp examples/skills/feature-dev/pipeline.yml /tmp/feature-dev-pipeline-old.yml
```
Expected: file copied (just for your own reference during development; not committed).

- [ ] **Step 2: Write new `pipeline.yml`**

Overwrite `examples/skills/feature-dev/pipeline.yml`:

````yaml
name: feature-dev
version: 1
description: "Quality-gated development pipeline (8 phases)"

args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E testing phases (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in code-review"

phases:
  - id: design
    description: "Generate design document via interactive brainstorming"
    invoke:
      skill: /brainstorming
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Design document with explored context and test perspectives"
    gate:
      - file_exists: "docs/features/*/design.md"
    validate: ./criteria/design.md
    confirm: true
    max_retries: 3

  - id: test-scenarios
    description: "Design comprehensive test cases and agent-browser scenarios"
    invoke:
      skill: /test-scenarios
      args:
        e2e: "args.e2e"
    consumes:
      - design_doc
    produces:
      - name: test_strategy
        path: "docs/features/*/test-strategy.md"
        description: "Human-readable test strategy (ISTQB/ISO 25010)"
      - name: scenarios
        path: "docs/features/*/scenarios.yml"
        description: "Agent-browser replay scenarios (Given/When/Then YAML)"
        when: "args.e2e"
    gate:
      - file_exists: "docs/features/*/test-strategy.md"
    validate: ./criteria/test-scenarios.md
    confirm: true
    max_retries: 3

  - id: plan
    description: "Generate implementation plan from design and test strategy"
    invoke:
      skill: /writing-plans
    consumes:
      - design_doc
      - test_strategy
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Task-level implementation plan (TDD)"
    gate:
      - file_exists: "docs/features/*/plan.md"
    validate: ./criteria/plan.md
    confirm: true
    max_retries: 3

  - id: execute
    description: "Execute implementation plan via TDD subagents"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - plan_doc
    validate: ../../criteria/execute.md
    confirm: true
    max_retries: 3

  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /code-review
      args:
        codex: "args.codex"
    consumes:
      - design_doc
      - plan_doc
    validate: ../../criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  - id: monkey-test
    description: "Replay pre-defined scenarios via agent-browser"
    when: "args.e2e"
    invoke:
      skill: /monkey-test
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - plan_doc
    produces:
      - name: monkey_test_report
        path: "docs/features/*/monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/features/*/monkey-test-results.json"
    gate:
      - file_exists: "docs/features/*/monkey-test-report.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3

  - id: dogfood
    description: "Exploratory testing via agent-browser with feature context"
    when: "args.e2e"
    invoke:
      skill: /dogfood
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - monkey_test_report
      - monkey_test_results
      - plan_doc
    produces:
      - name: dogfood_report
        path: "docs/features/*/dogfood-report/report.md"
    gate:
      - file_exists: "docs/features/*/dogfood-report/report.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create)"
    invoke:
      skill: /worktrunk
    consumes:
      - design_doc
      - plan_doc
    validate: ./criteria/integrate.md
    confirm: true
    max_retries: 3
````

- [ ] **Step 3: Lint the pipeline**

```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
```
Expected: exit code 0; output "Pipeline passed all lint checks" (or equivalent success message used by `belt lint`).

If lint fails, read the diagnostic, fix the YAML, re-lint.

- [ ] **Step 4: Verify all 8 phase IDs are present**

```bash
for id in design test-scenarios plan execute code-review monkey-test dogfood integrate; do
  grep -q "^  - id: $id" examples/skills/feature-dev/pipeline.yml || echo "MISSING $id"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 5: Verify regate target on code-review**

```bash
awk '/^  - id: code-review$/,/^  - id: /' examples/skills/feature-dev/pipeline.yml | grep -q "regate: \[execute\]" && echo "OK" || echo "FAIL"
```
Expected: `OK`.

- [ ] **Step 6: Commit**

```bash
git add examples/skills/feature-dev/pipeline.yml
git commit -m "feat(feature-dev): rewrite pipeline.yml for 8-phase flow"
```

---

## Task 16: SKILL.md

**Files:**
- Modify (replace): `examples/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Write new SKILL.md**

Overwrite `examples/skills/feature-dev/SKILL.md`:

````markdown
---
name: feature-dev
description: >-
  Quality-gated development pipeline (8 phases). Design → test scenarios → plan →
  execute → code review → monkey test (E2E scripted) → dogfood (E2E exploratory) →
  integrate. Web UI testing phases are conditional on --e2e.
user-invocable: true
---

# feature-dev

Belt pipeline for quality-gated development. 8 phases driven by belt-agent.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in code-review |

## Phase-Specific Invocation Rules

### Phase 1: design

- **INVOKE 1**: Read `./references/brainstorming-supplement.md` into context.
- **INVOKE 2**: Skill tool `/brainstorming`.
- The supplement injects parallel exploration (code-explorer / code-architect /
  impact-analyzer), implicit-rules extraction, required design sections, and
  worktree creation order.

### Phase 2: test-scenarios

- **INVOKE**: Skill tool `/test-scenarios` with `e2e` passed through from args.
- Produces `test-strategy.md` always; produces `scenarios.yml` when e2e.

### Phase 3: plan

- **INVOKE 1**: Read `./references/writing-plans-supplement.md`.
- **INVOKE 2**: Skill tool `/writing-plans`.
- The supplement enforces path override and Must-Verify / scenarios cross-referencing.

### Phase 4: execute

- **INVOKE**: Skill tool `/subagent-driven-development`.
- Orchestrator must reconstruct plan tasks into self-contained implementation
  specs before dispatching `feature-implementer` subagents. Do not forward
  broad research verbatim.

### Phase 5: code-review

- **INVOKE**: Skill tool `/code-review` with `codex` passed through.
- On fix commits, Phase 4 validate is re-verified per belt regate semantics.
  `max_retries: 3` limits the review-fix loop.

### Phase 6: monkey-test (when e2e)

- **INVOKE 1**: Read `./references/monkey-test-supplement.md`.
- **INVOKE 2**: Skill tool `/monkey-test`.

### Phase 7: dogfood (when e2e)

- **INVOKE 1**: Read `./references/dogfood-supplement.md`.
- **INVOKE 2**: Skill tool `/dogfood`.
- The supplement injects prior-phase artifacts as exploration hints.

### Phase 8: integrate

- **INVOKE 1**: Read `./references/worktrunk-supplement.md`.
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`) and
  execute accordingly via `/worktrunk`.

## Red Flags

- **Never skip the Phase 1 supplement load**: parallel exploration and the
  required design sections depend on it.
- **Never pass --iterations to /code-review**: single-pass review by design.
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through
  `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are
  phase-produced; manual edits break belt's phase-start mtime filter.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — Phase 1 overrides
- `./references/writing-plans-supplement.md` — Phase 3 overrides
- `./references/monkey-test-supplement.md` — Phase 6 context injection
- `./references/dogfood-supplement.md` — Phase 7 overrides and context injection
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
````

- [ ] **Step 2: Verify SKILL.md authoring principle compliance (no Phase Map)**

```bash
# Authoring principle forbids enumerating phases as a table with gate/regate.
# Verify no "| Phase |" or "Phase Map" heading.
grep -i "^## Phase Map" examples/skills/feature-dev/SKILL.md && echo "FAIL: phase map present" || echo "OK"
```
Expected: `OK`.

- [ ] **Step 3: Verify all 8 phase sections present**

```bash
for p in "Phase 1: design" "Phase 2: test-scenarios" "Phase 3: plan" "Phase 4: execute" "Phase 5: code-review" "Phase 6: monkey-test" "Phase 7: dogfood" "Phase 8: integrate"; do
  grep -q "^### $p" examples/skills/feature-dev/SKILL.md || echo "MISSING: $p"
done
echo "DONE"
```
Expected: `DONE` only.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "feat(feature-dev): rewrite SKILL.md for 8-phase flow"
```

---

## Task 17: belt.toml

**Files:**
- Modify (replace): `examples/skills/feature-dev/belt.toml`

- [ ] **Step 1: Write belt.toml**

Overwrite `examples/skills/feature-dev/belt.toml`:

````toml
pipeline = "./pipeline.yml"
````

(Minimal; matches existing examples/skills/*/belt.toml convention.)

- [ ] **Step 2: Verify**

```bash
test "$(cat examples/skills/feature-dev/belt.toml)" = 'pipeline = "./pipeline.yml"' && echo "OK" || echo "FAIL"
```
Expected: `OK`.

- [ ] **Step 3: Lint (repeat with new belt.toml + SKILL.md present)**

```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
```
Expected: exit code 0.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/belt.toml
git commit -m "chore(feature-dev): reset belt.toml to canonical form"
```

---

## Task 18: belt-core Integration Test — Phase Transitions (Happy Path)

**Files:**
- Create: `crates/belt-core/tests/feature_dev_refresh.rs`

- [ ] **Step 1: Write failing test**

Create `crates/belt-core/tests/feature_dev_refresh.rs`:

```rust
//! Integration tests for the refreshed feature-dev pipeline (8 phases).

use std::path::PathBuf;

use belt_core::{
    expander::expand_pipeline,
    parser::parse_pipeline,
};

fn feature_dev_pipeline_path() -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("examples/skills/feature-dev/pipeline.yml")
}

#[test]
fn feature_dev_has_eight_phases() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");

    let expected: &[&str] = &[
        "design",
        "test-scenarios",
        "plan",
        "execute",
        "code-review",
        "monkey-test",
        "dogfood",
        "integrate",
    ];

    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(got, expected, "phase IDs must match spec order");
}

#[test]
fn feature_dev_expands_cleanly() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");
    let expanded = expand_pipeline(&pipeline, &feature_dev_pipeline_path())
        .expect("expand feature-dev pipeline (no sub-pipelines remain in this refresh)");
    // Refresh deletes all `uses:`/`invoke.pipeline:` references; the expanded
    // phases must equal the top-level phases 1:1.
    assert_eq!(expanded.phases.len(), pipeline.phases.len());
}
```

- [ ] **Step 2: Run test to verify it fails meaningfully**

```bash
cargo test -p belt-core --test feature_dev_refresh
```
Expected: If `pipeline.yml` from Task 15 is committed correctly, both tests should PASS. If either fails, fix the pipeline, not the test.

- [ ] **Step 3: Re-run to confirm**

```bash
cargo test -p belt-core --test feature_dev_refresh -- --nocapture
```
Expected: `test result: ok. 2 passed`.

- [ ] **Step 4: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs
git commit -m "test(belt-core): add feature-dev refresh phase structure test"
```

---

## Task 19: belt-core Integration Test — Conditional Skipping

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs`

- [ ] **Step 1: Add failing test for conditional phases**

Append to `crates/belt-core/tests/feature_dev_refresh.rs`:

```rust
#[test]
fn monkey_test_and_dogfood_are_conditional_on_e2e() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");

    let monkey = pipeline
        .phases
        .iter()
        .find(|p| p.id == "monkey-test")
        .expect("monkey-test phase exists");
    let dogfood = pipeline
        .phases
        .iter()
        .find(|p| p.id == "dogfood")
        .expect("dogfood phase exists");

    assert_eq!(
        monkey.when.as_deref(),
        Some("args.e2e"),
        "monkey-test must be gated by args.e2e"
    );
    assert_eq!(
        dogfood.when.as_deref(),
        Some("args.e2e"),
        "dogfood must be gated by args.e2e"
    );
}

#[test]
fn scenarios_produce_is_conditional_on_e2e() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");

    let test_scenarios = pipeline
        .phases
        .iter()
        .find(|p| p.id == "test-scenarios")
        .expect("test-scenarios phase exists");

    let scenarios_artifact = test_scenarios
        .produces
        .iter()
        .find(|a| a.name == "scenarios")
        .expect("scenarios artifact declared");

    assert_eq!(
        scenarios_artifact.when.as_deref(),
        Some("args.e2e"),
        "scenarios.yml produce must be gated by args.e2e"
    );
}
```

- [ ] **Step 2: Run the new tests**

```bash
cargo test -p belt-core --test feature_dev_refresh
```
Expected: `test result: ok. 4 passed`.

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs
git commit -m "test(belt-core): cover conditional e2e gating of feature-dev phases"
```

---

## Task 20: belt-core Integration Test — Regate Topology and Args

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs`

- [ ] **Step 1: Add failing test for regate**

Append:

```rust
#[test]
fn code_review_regates_execute_only() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");

    let code_review = pipeline
        .phases
        .iter()
        .find(|p| p.id == "code-review")
        .expect("code-review phase exists");

    let regate = code_review.regate.clone().unwrap_or_default();
    assert_eq!(
        regate,
        vec!["execute".to_string()],
        "code-review.regate must target only [execute] (no smoke-test/doc-audit)"
    );
    assert_eq!(code_review.max_retries, Some(3));
}

#[test]
fn top_level_args_are_e2e_and_codex_only() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("parse feature-dev pipeline");

    let mut names: Vec<&str> = pipeline.args.keys().map(|k| k.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["codex", "e2e"], "args must be exactly {{codex, e2e}}");

    let e2e = pipeline.args.get("e2e").expect("e2e arg");
    assert_eq!(e2e.ty, "bool");
    assert_eq!(e2e.default.as_deref(), Some("false"));

    let codex = pipeline.args.get("codex").expect("codex arg");
    assert_eq!(codex.ty, "bool");
    assert_eq!(codex.default.as_deref(), Some("false"));
}
```

> NOTE: If `Pipeline.args` is not a `BTreeMap<String, ArgSpec>` in belt-core,
> adjust the assertion style to match the actual type while preserving intent
> (exactly two args named `e2e` and `codex`, both bool, both default false).
> Do NOT change the pipeline.yml to make the test pass.

- [ ] **Step 2: Run**

```bash
cargo test -p belt-core --test feature_dev_refresh
```
Expected: `test result: ok. 6 passed`.

If API mismatch blocks you, inspect `crates/belt-core/src/model.rs` for the
actual field names/types, adjust the test only (not the pipeline).

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs
git commit -m "test(belt-core): cover regate topology and args shape"
```

---

## Task 21: /test-scenarios Skill (dotfiles)

**Files:**
- Create: `~/.dotfiles/claude/skills/test-scenarios/SKILL.md`

- [ ] **Step 1: Enter the dotfiles repo**

```bash
cd ~/.dotfiles
git status
```
Expected: clean working tree or untracked `.claude/` only. If dirty, stash or commit unrelated changes before continuing.

- [ ] **Step 2: Create the skill directory**

```bash
mkdir -p claude/skills/test-scenarios
```

- [ ] **Step 3: Write SKILL.md**

Write `~/.dotfiles/claude/skills/test-scenarios/SKILL.md`:

````markdown
---
name: test-scenarios
description: >-
  Generate a comprehensive test strategy (ISTQB test-design techniques + ISO 25010
  quality characteristics) plus, when `--e2e` is true, an agent-browser-replayable
  scenarios.yml in Given/When/Then YAML. Designed for feature-dev Phase 2. Reads a
  design document and outputs artifacts under docs/features/<topic>/.
user-invocable: true
---

# test-scenarios

Derive test cases and (optionally) executable E2E scenarios from a design doc.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | When true, also emit `scenarios.yml` |

## Inputs

1. A design document at `docs/features/<topic>/design.md` (required).
   Honor the sections:
   `Prerequisites`, `Impact Scope`, `Impact Analysis`, `Must-Verify Checklist`,
   `Test Perspectives`.

## Outputs

Always:
- `docs/features/<topic>/test-strategy.md` — human-readable.

When `--e2e`:
- `docs/features/<topic>/scenarios.yml` — machine-readable.

## `test-strategy.md` Required Sections

- **Test Design Techniques** (ISTQB): equivalence partitioning, boundary-value
  analysis, decision tables, state-transition testing. For each technique,
  list ≥ 1 concrete application to this feature.
- **Quality Characteristics** (ISO 25010): functional suitability, performance
  efficiency, compatibility, usability, reliability, security, maintainability,
  portability. For each, state: `Relevance: in-scope | out-of-scope (reason)`.
- **Priority Matrix**: a table mapping each characteristic to criticality
  (`critical | high | medium | low`) for this feature.
- **Non-Functional Requirements**: at least one measurable acceptance
  criterion (e.g., "Login responds in < 500ms p95").
- **Must-Verify Mapping**: a table with one row per item in the design's
  Must-Verify Checklist; each row links to one or more test entries in the
  sections above.

## `scenarios.yml` Schema

```yaml
scenarios:
  - id: <kebab-case>
    category: <string>         # e.g. authentication, payment, ui-navigation
    severity: critical | high | medium | low
    given: <string>            # natural language precondition
    when: <string>             # natural language action
    then: <string>             # natural language expected outcome
    preconditions: [<string>]  # optional, list
    postconditions: [<string>] # optional, list
```

- Generate ≥ 3 scenarios.
- Cover at least one scenario per `category` inferred from Test Perspectives.
- At least one scenario at `severity: critical` for the feature's core success path.
- Severity must track the design's Must-Verify Checklist priority.

## Interaction Model

1. Read `design.md`.
2. Propose the `test-strategy.md` outline to the user; get approval to write.
3. Write `test-strategy.md`.
4. When `--e2e`:
   a. Propose the `scenarios.yml` scenario list (titles + severities);
      get approval.
   b. For each approved scenario, draft the Given/When/Then; confirm per
      scenario.
   c. Write `scenarios.yml`.
5. Commit the outputs.

## Red Flags

- Never mix human-readable prose into `scenarios.yml`. It is machine-oriented.
- Never produce GitHub Issue templates. (That is `/breakdown-test`'s job.)
- Never write files outside `docs/features/<topic>/`.
````

- [ ] **Step 4: Symlink into ~/.claude/skills/**

```bash
ln -s ~/.dotfiles/claude/skills/test-scenarios ~/.claude/skills/test-scenarios
```

Skip this step if the user's dotfiles setup script symlinks skills automatically; verify:
```bash
ls -la ~/.claude/skills/test-scenarios
```
Expected: a symlink to `~/.dotfiles/claude/skills/test-scenarios`.

- [ ] **Step 5: Verify frontmatter parses**

```bash
head -8 ~/.claude/skills/test-scenarios/SKILL.md
```
Expected output includes:
```
---
name: test-scenarios
description: >-
...
user-invocable: true
---
```

- [ ] **Step 6: Commit in dotfiles**

```bash
cd ~/.dotfiles
git add claude/skills/test-scenarios/SKILL.md
git commit -m "feat(skills): add test-scenarios skill for feature-dev Phase 2"
cd -
```

---

## Task 22: /monkey-test Skill (dotfiles)

**Files:**
- Create: `~/.dotfiles/claude/skills/monkey-test/SKILL.md`

- [ ] **Step 1: Create the skill directory**

```bash
cd ~/.dotfiles
mkdir -p claude/skills/monkey-test
```

- [ ] **Step 2: Write SKILL.md**

Write `~/.dotfiles/claude/skills/monkey-test/SKILL.md`:

````markdown
---
name: monkey-test
description: >-
  Replay pre-defined Given/When/Then scenarios via agent-browser. Designed for
  feature-dev Phase 6. Consumes scenarios.yml and produces a human-readable
  report plus machine-readable results.json. Reads design.md, test-strategy.md,
  and plan.md as hints to resolve ambiguity and decide SKIP verdicts.
allowed-tools: Bash(agent-browser:*), Bash(npx agent-browser:*)
user-invocable: true
---

# monkey-test

Scripted E2E regression testing via agent-browser.

## Inputs

- `docs/features/<topic>/scenarios.yml` (required)
- `docs/features/<topic>/design.md` (hint — disambiguates natural-language steps)
- `docs/features/<topic>/test-strategy.md` (hint — severity calibration)
- `docs/features/<topic>/plan.md` (hint — SKIP decision for incomplete tasks)

## Outputs

- `docs/features/<topic>/monkey-test-report.md` — human-readable
- `docs/features/<topic>/monkey-test-results.json` — machine-readable
- `docs/features/<topic>/monkey-test-screenshots/` — step screenshots

## results.json Schema

```json
{
  "scenarios": [
    {
      "id": "string",
      "status": "PASS | FAIL | SKIP",
      "severity": "critical | high | medium | low",
      "duration_ms": 1234,
      "error": "string (FAIL only)",
      "skip_reason": "string (SKIP only)",
      "screenshots": ["docs/features/<topic>/monkey-test-screenshots/<id>-step1.png", "..."]
    }
  ],
  "summary": {
    "total": 10,
    "passed": 8,
    "failed": 1,
    "skipped": 1
  }
}
```

## Execution Flow

1. Read `scenarios.yml`.
2. For each scenario:
   a. Check `plan.md`: if the implementing task is marked incomplete,
      status = `SKIP` with `skip_reason`.
   b. Otherwise launch agent-browser (restore auth-state from
      `docs/features/<topic>/monkey-test-screenshots/auth-state.json` if
      present).
   c. Interpret `given`: set up preconditions (navigate, fixture setup).
      When wording is ambiguous, consult `design.md` definitions.
   d. Interpret `when`: perform actions. Capture a screenshot per action.
   e. Interpret `then`: evaluate assertions. Capture a final screenshot.
   f. Record result. On FAIL, include expected-vs-actual in `error`.
3. Write `results.json` conforming to the schema.
4. Write `monkey-test-report.md` with:
   - Summary (totals)
   - Critical and High FAIL details (expected vs actual, screenshots)
   - Full per-scenario table

## Report Template (report.md)

```markdown
# Monkey Test Report: <feature-name>

## Summary
- Total scenarios: N
- Passed: N
- Failed: N
- Skipped: N
- Duration: XX min

## Critical and High Failures
<per-failure: id, severity, expected vs actual, screenshots>

## Full Results
| id | status | severity | duration_ms | notes |
| ... | ... | ... | ... | ... |
```

## Red Flags

- Never skip writing `results.json` — downstream Phase 7 (dogfood) depends on it.
- Never auto-retry FAIL scenarios silently — report them.
- Never write files outside `docs/features/<topic>/`.
- Never fabricate screenshots — absence of agent-browser output = SKIP with
  a clear `skip_reason` ("agent-browser unavailable").

## Difference from /dogfood

- This skill is **scripted** — replays scenarios to verify known behavior.
- `/dogfood` is **exploratory** — finds new issues without a script.
- Both use agent-browser. Use this when scenarios.yml exists; use dogfood to
  discover issues scenarios.yml cannot foresee.
````

- [ ] **Step 3: Symlink**

```bash
ln -s ~/.dotfiles/claude/skills/monkey-test ~/.claude/skills/monkey-test
ls -la ~/.claude/skills/monkey-test
```
Expected: symlink to dotfiles.

- [ ] **Step 4: Verify frontmatter**

```bash
head -10 ~/.claude/skills/monkey-test/SKILL.md
```
Expected output includes `name: monkey-test` and `allowed-tools: Bash(agent-browser:*)...`.

- [ ] **Step 5: Commit in dotfiles**

```bash
cd ~/.dotfiles
git add claude/skills/monkey-test/SKILL.md
git commit -m "feat(skills): add monkey-test skill for feature-dev Phase 6"
cd -
```

---

## Task 23: Full Workspace Verification (belt)

**Files:** (none modified, verification only)

Back in the belt worktree:

- [ ] **Step 1: Return to belt worktree**

```bash
cd $(wt path 2026-04-14-feature-dev-refresh 2>/dev/null || pwd)
pwd
```
Expected: path includes `belt-wt/2026-04-14-feature-dev-refresh` or your chosen worktree root.

- [ ] **Step 2: `cargo fmt` for changed packages only**

```bash
cargo fmt --package belt-core
```
Expected: no output (or only formatting changes applied).

- [ ] **Step 3: `cargo clippy` on belt-core**

```bash
cargo clippy --package belt-core -- -D warnings
```
Expected: `Finished ... target(s)` with no warnings.

- [ ] **Step 4: Full test run on belt-core**

```bash
cargo test --package belt-core
```
Expected: all tests pass including the 6 new ones from Tasks 18-20.

- [ ] **Step 5: `belt lint` on all example pipelines (ensure no regression)**

```bash
for toml in examples/skills/*/belt.toml; do
  dir=$(dirname "$toml")
  echo "Linting $dir..."
  cargo run -p belt --quiet -- lint "$dir/pipeline.yml" || echo "FAIL: $dir"
done
```
Expected: every pipeline lints clean.

- [ ] **Step 6: Commit any formatting changes**

```bash
git status
# If clean, no commit needed. Otherwise:
git add -u
git commit -m "chore(belt-core): apply cargo fmt"
```

---

## Task 24: Delete Obsolete Files from Old feature-dev

**Files:**
- Delete: `examples/skills/feature-dev/criteria/spec-review.md` (doesn't exist after Task 2, verify)
- Delete: `examples/skills/feature-dev/criteria/plan-review.md` (doesn't exist after Task 2, verify)
- Delete: `examples/skills/feature-dev/criteria/doc-audit.md` (doesn't exist after Task 2, verify)
- Delete: `examples/skills/feature-dev/references/evidence-plan-protocol.md` (doesn't exist after Task 2, verify)
- Delete: `examples/skills/feature-dev/references/fix-dispatch-strategy.md` (doesn't exist after Task 2, verify)

(Task 2 already removed the entire `criteria/` and `references/` dirs, so this
task is a safety check only.)

- [ ] **Step 1: Verify no stale files remain**

```bash
ls examples/skills/feature-dev/criteria/
```
Expected: only the files created in Tasks 9-14:
`design.md  dogfood.md  integrate.md  monkey-test.md  plan.md  test-scenarios.md`

```bash
ls examples/skills/feature-dev/references/
```
Expected: only the files created in Tasks 3-8:
`brainstorming-supplement.md  dogfood-supplement.md  monkey-test-supplement.md  path-convention.md  worktrunk-supplement.md  writing-plans-supplement.md`

- [ ] **Step 2: No commit needed**

(If any stray file surfaces, `git rm` it with an explicit commit message.)

---

## Task 25: Superseded Notice on Old Spec

**Files:**
- Modify: `docs/specs/2026-04-07-feature-dev-belt-migration.md`

- [ ] **Step 1: Add superseded frontmatter note**

Open `docs/specs/2026-04-07-feature-dev-belt-migration.md` and insert a
`superseded_by` field at the very top frontmatter (before the first `#`):

If the file has no frontmatter, add one. Otherwise, extend existing
frontmatter. Example:

```markdown
---
superseded_by: docs/specs/2026-04-14-feature-dev-refresh-design.md
superseded_date: 2026-04-14
superseded_scope: >-
  The 19→10 phase collapse remains accurate, but the phase set has been
  replaced by the 8-phase refresh (design → test-scenarios → plan → execute
  → code-review → monkey-test → dogfood → integrate) per the new design.
---

# (existing content preserved below)
```

- [ ] **Step 2: Verify the note is at the top**

```bash
head -8 docs/specs/2026-04-07-feature-dev-belt-migration.md
```
Expected: shows the `superseded_by` field.

- [ ] **Step 3: Commit**

```bash
git add docs/specs/2026-04-07-feature-dev-belt-migration.md
git commit -m "docs(specs): mark 2026-04-07 feature-dev migration spec as superseded"
```

---

## Task 26: Final Full-Repo Verification

**Files:** (none modified; verification only)

- [ ] **Step 1: Clippy across workspace**

```bash
cargo clippy --workspace -- -D warnings
```
Expected: no warnings.

- [ ] **Step 2: Tests across workspace**

```bash
cargo test --workspace
```
Expected: all tests pass.

- [ ] **Step 3: `belt lint` on every example pipeline**

```bash
for toml in examples/skills/*/belt.toml; do
  dir=$(dirname "$toml")
  cargo run -p belt --quiet -- lint "$dir/pipeline.yml" || {
    echo "FAIL: $dir"; exit 1;
  }
done
echo "ALL LINTS PASSED"
```
Expected: `ALL LINTS PASSED` on last line.

- [ ] **Step 4: Git log summary**

```bash
git log --oneline main..HEAD
```
Expected: ~20+ commits spanning Task 2 through Task 25, in dependency order.

- [ ] **Step 5: `git status` final**

```bash
git status
```
Expected: clean working tree.

No commit in this task (verification only).

---

## Task 27: Integration (deferred to Phase 8 of the pipeline itself)

This task is NOT executed as part of the implementation plan. After all
previous tasks pass, the user runs the new feature-dev pipeline on itself
(or a different feature) to dogfood the flow. The integration step
(`wt merge` vs `gh pr create`) is then performed per the Phase 8 user
prompt.

- [ ] **Step 1: Announce completion to user**

Print to conversation:

```
Implementation complete. All 26 implementation tasks are committed.

Next steps (for user to run manually):
1. `cd` to the worktree.
2. `cargo test --workspace` — sanity check.
3. `belt-agent init examples/skills/feature-dev/` — start the new pipeline
   on a trial feature.
4. When the trial run reaches Phase 8, select (A) or (B) to merge this
   refresh into main.
```

- [ ] **Step 2: Confirm user reviewed the changes**

Wait for user to confirm they will run steps 2-4. Do not run them
automatically — merging is the user's call.

---

## Self-Review

### Spec Coverage

| Spec section | Plan task(s) |
|---|---|
| §1 Background/Motivation | (informational; no task) |
| §2 Diff vs current | Task 15 (pipeline.yml rewrite), Task 16 (SKILL.md rewrite) |
| §3 Flow | Task 15 |
| §4 Args schema | Task 15, Task 20 (tests) |
| §5 pipeline.yml | Task 15 |
| §6 Directory structure | Task 2 (scaffold), Tasks 3-17 |
| §7 Path convention | Task 3 |
| §8 Supplements | Tasks 3, 4, 5, 6, 7, 8 |
| §9 New skills | Task 21 (test-scenarios), Task 22 (monkey-test) |
| §10 Done-criteria | Tasks 9-14 |
| §11 SKILL.md structure | Task 16 |
| §12 Data flow / artifact chain | Task 15 (consumes/produces), Tasks 18-20 (tests) |
| §13 Error handling / regate | Task 15 (regate: [execute]), Task 20 (test) |
| §14 Pipeline-self test strategy | Tasks 18, 19, 20, 23, 26 |
| §15 Open questions (deferred) | Resolved inline: new skills go to dotfiles (Tasks 21, 22); `<topic>` slug rule specified in Task 3 |
| §16 Decision summary | (informational) |
| §17 Migration/atomic cutover | Task 2 (dir wipe), Tasks 15, 16, 17 (file replace), Task 24 (verification), Task 25 (spec superseded) |
| §18 References | (informational) |

All spec sections are covered.

### Placeholder Scan

No `TBD`, `TODO`, `appropriate error handling`, or `fill in details` in plan
text. All code blocks contain complete content. All commands are exact.

### Type Consistency

- `design_doc` / `plan_doc` / `test_strategy` / `scenarios` /
  `monkey_test_report` / `monkey_test_results` / `dogfood_report` — used
  consistently across pipeline.yml (Task 15), tests (Tasks 18-20),
  supplements (Tasks 3-8).
- `args.e2e` / `args.codex` — used consistently in pipeline.yml and tests.
- Phase IDs (`design`, `test-scenarios`, `plan`, `execute`, `code-review`,
  `monkey-test`, `dogfood`, `integrate`) — consistent across pipeline.yml,
  SKILL.md, tests, criteria filenames.
- Criteria ID prefixes (`DESIGN-`, `TEST-`, `PLAN-`, `MONKEY-`, `DOGFOOD-`,
  `INT-`) — consistent.

---

## Execution Handoff

**Plan complete and saved to `docs/plans/2026-04-14-feature-dev-refresh-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `executing-plans`, batch execution with checkpoints.

**Which approach?**
