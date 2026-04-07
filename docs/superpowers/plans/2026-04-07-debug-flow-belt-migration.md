# debug-flow Belt Pipeline Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port debug-flow to `examples/skills/debug-flow/` as a self-contained belt pipeline example.

**Architecture:** 13 new files — pipeline.yml (15 phases), SKILL.md (~50 lines), 7 done-criteria, 3 reference docs. Reuses existing sub-pipelines (code-review, test-review, implementation-review) via `uses:`. No belt-core changes.

**Tech Stack:** YAML (pipeline), Markdown (SKILL.md, done-criteria, references), belt CLI (`belt lint` for verification)

**Spec:** `docs/superpowers/specs/2026-04-07-debug-flow-belt-migration.md`

---

### Task 1: Create belt.toml and pipeline.yml

**Files:**
- Create: `examples/skills/debug-flow/belt.toml`
- Create: `examples/skills/debug-flow/pipeline.yml`

- [ ] **Step 1: Create belt.toml**

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 2: Create pipeline.yml**

```yaml
name: debug-flow
description: "Quality-gated debugging orchestrator"
version: 1
args:
  e2e:        { type: bool, default: false }
  smoke:      { type: bool, default: false }
  codex:      { type: bool, default: false }
  ui:         { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm:      { type: bool, default: false }

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
    description: "Audit RCA report against done-criteria"
    config:
      audit: required
      criteria: "rca"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/rca.md pass"
    confirm: true
    max_retries: 3

  # ─── Fix Plan ───
  - id: fix-plan
    description: "Create fix plan from RCA report"
    config:
      skill: "/writing-plans"
    gate:
      - file_exists: "docs/plans/*-fix-plan.md"

  - id: fix-plan-audit
    description: "Audit fix plan"
    config:
      audit: required
      criteria: "fix-plan"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/fix-plan.md pass"
    confirm: true
    max_retries: 3

  # ─── Fix Plan Review ───
  - id: fix-plan-review
    uses: ../implementation-review/pipeline.yml

  - id: fix-plan-review-audit
    description: "Audit fix plan review completion"
    config:
      audit: required
      criteria: "fix-plan-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/fix-plan-review.md pass"
    confirm: true
    max_retries: 3

  # ─── Execute ───
  - id: execute
    description: "TDD implementation following the fix plan"
    config:
      skill: "/subagent-driven-development"
    gate:
      - cmd: "make test"
    max_retries: 3

  - id: execute-audit
    description: "Audit implementation against fix plan"
    config:
      audit: required
      criteria: "execute"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/execute.md pass"
    confirm: true
    max_retries: 3

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    config:
      skill: "/smoke-test"
    gate:
      - file_exists: "smoke-test-report.md"

  - id: smoke-test-audit
    description: "Audit smoke test results"
    when: "args.smoke"
    config:
      audit: required
      criteria: "smoke-test"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/smoke-test.md pass"
    confirm: true
    max_retries: 3

  # ─── Code Review ───
  - id: code-review
    uses: ../code-review/pipeline.yml

  - id: code-review-audit
    description: "Audit code review and verify regression"
    config:
      audit: required
      criteria: "code-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/code-review.md pass"
    confirm: true
    max_retries: 3
    regate: [execute, smoke-test]

  # ─── Test Review (conditional) ───
  - id: test-review
    when: "args.e2e"
    uses: ../test-review/pipeline.yml

  - id: test-review-audit
    description: "Audit test review and verify regression"
    when: "args.e2e"
    config:
      audit: required
      criteria: "test-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/test-review.md pass"
    confirm: true
    max_retries: 3
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

- [ ] **Step 3: Run belt lint to verify pipeline**

Run: `cargo run -p belt -- lint examples/skills/debug-flow/pipeline.yml`
Expected: lint passes with no errors. Sub-pipeline `uses:` paths resolve to existing files.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/belt.toml examples/skills/debug-flow/pipeline.yml
git commit -m "feat(example): add debug-flow pipeline.yml and belt.toml"
```

---

### Task 2: Create SKILL.md

**Files:**
- Create: `examples/skills/debug-flow/SKILL.md`

- [ ] **Step 1: Create SKILL.md**

```markdown
---
name: debug-flow
description: >-
  Quality-gated debugging orchestrator. Drives an 8-phase pipeline
  (rca → fix-plan → fix-plan-review → execute → smoke-test →
  code-review → test-review → integrate) via belt-agent CLI.
  Conditional phases: --e2e (test-review), --smoke (smoke-test).
  Passthrough flags: --codex, --ui, --iterations N, --swarm.
user-invocable: true
argument-hint: "[--e2e] [--smoke] [--codex] [--ui] [--iterations N] [--swarm]"
---

# Debug Flow Orchestrator

Quality-gated debugging pipeline driven by belt-agent.
belt handles phase transitions, gates, regate, and conditional skipping.
The orchestrator investigates root cause, plans fixes, and drives
implementation through quality gates.

## Dispatch Rules

| config pattern | Action |
|---|---|
| `config.skill: "/systematic-debugging"` | Parallel exploration: dispatch code-explorer, code-architect, impact-analyzer subagents. Synthesize findings into RCA Report. Create worktree. Write reproduction test (must FAIL). Commit RCA Report |
| `config.skill: "/writing-plans"` | Expand RCA Report's Fix Strategy into fix plan at `docs/plans/*-fix-plan.md` |
| `config.skill` (other) | Invoke the skill. Pass `config.codex`, `config.iterations`, `config.swarm`, `config.ui` as options (resolved from `args`) |
| Sub-pipeline phase (id contains `/`) | Load SKILL.md from the sub-pipeline's skill directory (resolved from `uses:` path in pipeline.yml). Follow that SKILL.md's dispatch rules for the current sub-phase. Runtime args (`codex`, `iterations`, `swarm`, `ui`) come from top-level pipeline args |
| `config.audit: "required"` | Read `references/done-criteria/{config.criteria}.md`. Dispatch `phase-auditor` subagent per `references/audit-protocol.md`. Write verdict to output_dir. PASS → `step --confirm`. FAIL → fix per `references/fix-dispatch-strategy.md`, re-audit |

## Coordinator Discipline

The orchestrator owns understanding of root cause and fix strategy.
Do NOT delegate synthesis to subagents.

- Research → Synthesis → Implementation → Verification
- After receiving exploration results, reconstruct root cause yourself
- Parallel exploration for read-only investigation; serialize write operations

## Evidence Plan

Generated after `rca-audit` completes. Re-evaluated after `fix-plan-review-audit`
if RCA report hash changed. Details: `references/evidence-plan-protocol.md`.

## Red Flags — NEVER DO

- Skip the `rca` phase or start fixing without root cause
- Proceed without a reproduction test
- Delegate root cause synthesis to subagents
- Filter or omit review findings
- Choose merge/PR/keep/discard on behalf of the user
- Proceed past a FAIL verdict without fix + re-audit or user intervention
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/debug-flow/SKILL.md
git commit -m "feat(example): add debug-flow SKILL.md orchestrator protocol"
```

---

### Task 3: Create rca.md done-criteria

**Files:**
- Create: `examples/skills/debug-flow/references/done-criteria/rca.md`

- [ ] **Step 1: Create rca.md**

```markdown
---
name: rca
max_retries: 3
audit: required
---

## Criteria

### RCA-01: RCA Report file exists with required sections
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Search for an RCA Report file using `Glob("docs/plans/*-rca-report.md")`
  2. Verify the file contains 5 section headings: `## Symptom`, `## Investigation Record`, `## Root Cause`, `## Reproduction Test`, `## Fix Strategy`
- **pass_condition**: At least one Glob result found, and all 5 section headings are present
- **fail_diagnosis_hint**: Verify that the RCA phase executor output the report under `docs/plans/`. Confirm the filename matches the `*-rca-report.md` pattern and section headings use `##` level with exact titles
- **depends_on_artifacts**: [docs/plans/]
- **forward_check**: Fix Plan phase uses the RCA Report's Fix Strategy as input

### RCA-02: Investigation Record has substantive content in 4 subsections
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  Read each Investigation Record subsection and verify:
  1. Code Flow Trace: at least one call chain documented (file path + function name pair)
  2. Architecture Context: at least one relevant pattern, convention, or implicit rule described
  3. Impact Scope: at least one affected file or module listed
  4. Symmetry Check: determination of whether the change target has a "pair" (if none, rationale required)
- **pass_condition**: All 4 subsections meet their criteria. Zero heading-only or generic-content-only subsections
- **fail_diagnosis_hint**: Identify which subsection is deficient. Cross-reference with code-explorer / code-architect / impact-analyzer output to fill gaps
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]

### RCA-03: Impact Scope file paths exist in the codebase
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Extract file paths from the Impact Scope subsection using regex
  2. Verify each path exists using `Glob`
- **pass_condition**: All extracted paths exist. Zero non-existent paths
- **fail_diagnosis_hint**: List non-existent paths and determine if they are typos or deleted files. Verify the codebase state matches the RCA Report's analysis point
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]

### RCA-04: At least 1 excluded hypothesis recorded
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the Excluded Hypotheses section (within Investigation Record or as a standalone section)
  2. Verify each hypothesis entry contains: hypothesis statement, verification method, rejection reason
  3. If the first hypothesis was correct, verify documentation of why alternatives were excluded
- **pass_condition**: At least 1 excluded hypothesis with all 3 elements present
- **fail_diagnosis_hint**: If zero hypotheses are recorded, the investigation may not have considered alternative causes. List potential alternative root causes and document why each was excluded
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]

### RCA-05: Reproduction test exists and its result is FAIL
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Extract the test file path from the Reproduction Test section
  2. Verify the test file exists using `Glob`
  3. Execute the test command and confirm the specific reproduction test FAILs
- **pass_condition**: Test file exists and test execution shows the reproduction test as FAIL
- **fail_diagnosis_hint**: If the test file does not exist, the RCA executor did not create it. If the test PASSes, the test does not correctly capture the bug — review the Root Cause mechanism and fix the assertion
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, tests/]

### RCA-06: Root Cause contains specific file path, line number, and mechanism
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the Root Cause section
  2. Check for at least one file path (string containing `/` or `.`)
  3. Check for at least one line number (`:` + digits or "line" + digits)
  4. Check for at least one mechanism explanation (why the code at that location causes the problem)
- **pass_condition**: File path, line number, and mechanism explanation all present
- **fail_diagnosis_hint**: If missing, use code-explorer output to identify the exact fault location. Write the mechanism as "input X passes through Y and produces state Z because of [specific code behavior]"
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]

### RCA-07: RCA Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/plans/*-rca-report.md` and confirm the file is not in the uncommitted changes list.
- **pass_condition**: `git status --porcelain` output does not contain the RCA Report path (zero output lines)
- **fail_diagnosis_hint**: If uncommitted, `git add` + `git commit` was not executed. Check the final step of the RCA phase executor
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]

### RCA-08: Symmetry Check evaluates asymmetry risk
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  Read the Symmetry Check subsection and verify documentation of:
  1. Whether the change target has a "pair" (determination result; if none, rationale required)
  2. If pairs exist: file paths, function names, and pair type for each
  3. Symmetry comparison of filter/scope conditions
  4. Asymmetry risk assessment and impact scope
- **pass_condition**: All 4 dimensions documented. If "no pair" is determined, the rationale is specific (not generic)
- **fail_diagnosis_hint**: If Symmetry Check is empty or incomplete, reference impact-analyzer output to identify paired paths. For "no pair" determinations, verify against Reverse Dependencies and Shared State analysis
- **depends_on_artifacts**: [docs/plans/*-rca-report.md]
- **forward_check**: Fix Plan must include tasks for paired paths if asymmetry risk is identified

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/debug-flow/references/done-criteria/rca.md
git commit -m "feat(example): add debug-flow rca done-criteria"
```

---

### Task 4: Create fix-plan.md and fix-plan-review.md done-criteria

**Files:**
- Create: `examples/skills/debug-flow/references/done-criteria/fix-plan.md`
- Create: `examples/skills/debug-flow/references/done-criteria/fix-plan-review.md`

- [ ] **Step 1: Create fix-plan.md**

Adapted from `examples/skills/feature-dev/references/done-criteria/plan.md`.
Key changes: glob pattern `*-fix-plan.md`, traceability references RCA Report Fix Strategy instead of design requirements, `depends_on_artifacts` references `*-rca-report.md`.

```markdown
---
name: fix-plan
max_retries: 3
audit: required
---

## Criteria

### FIX-PLAN-01: Fix plan document file exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Search for a fix plan document file using `Glob("docs/plans/*-fix-plan.md")`.
- **pass_condition**: At least one Glob result found
- **fail_diagnosis_hint**: Verify that the Phase executor output the fix plan document under `docs/plans/`. Confirm the filename matches the `*-fix-plan.md` pattern
- **depends_on_artifacts**: [docs/plans/]

### FIX-PLAN-02: Traceability from RCA Report Fix Strategy to tasks
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all fix items from the RCA Report's Fix Strategy section
  2. Enumerate all tasks from the fix plan document's task section
  3. For each fix item, verify at least one corresponding task exists in the fix plan
  4. List any fix items that have no corresponding task
- **pass_condition**: Step 3: all fix items have at least one corresponding task. Step 4: the list is empty (zero items)
- **fail_diagnosis_hint**: Identify fix items without corresponding tasks and add the missing tasks to the fix plan. Create a mapping table of RCA Fix Strategy items to fix plan task IDs to visualize gaps
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md]

### FIX-PLAN-03: Task granularity is sub-agent executable
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Count the description length (lines) and number of steps in each task of the fix plan
  2. Verify each task has 10 or fewer steps
  3. Verify no single task contains multiple independent feature changes (detect tasks spanning 3 or more modules)
- **pass_condition**: All tasks have 10 or fewer steps and each task's change scope spans fewer than 3 modules. Zero tasks exceed these limits
- **fail_diagnosis_hint**: Identify tasks exceeding the step count as split candidates. For tasks spanning many modules, consider splitting by module boundary
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### FIX-PLAN-04: Task dependencies are explicit and consistent (no cycles)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Extract all task IDs and each task's dependency target IDs from the fix plan
  2. Verify all referenced dependency target IDs exist within the fix plan (no dangling references)
  3. Build a dependency graph and verify no circular dependencies exist (no paths like A->B->C->A)
  4. Check for tasks without explicit dependencies that actually depend on another task's output
- **pass_condition**: Step 2: zero dangling ID references. Step 3: zero circular paths. Step 4: zero implicit dependencies
- **fail_diagnosis_hint**: If circular dependencies are detected, consider splitting tasks or reversing the dependency direction. For dangling references, determine whether it is a typo or a missing task. For implicit dependencies, make the input/output relationship between tasks explicit
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### FIX-PLAN-05: Test cases are specified in Given/When/Then format
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Search the fix plan's test case section with `Grep` for the Given/When/Then pattern
  2. Verify each test case contains all three elements: Given (preconditions), When (action), Then (expected result)
  3. Verify that Then clauses contain verifiable expected values such as numeric thresholds or pattern-matchable assertions
- **pass_condition**: All test cases include all 3 elements of Given/When/Then (step 2), and Then clauses contain verifiable expected values (step 3). Zero test cases are missing elements
- **fail_diagnosis_hint**: Identify test cases missing Given/When/Then elements and supplement with specific preconditions, actions, and expected results by referencing the RCA Report's Root Cause and Reproduction Test sections
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]
- **forward_check**: During the Execute phase, test cases should be directly translatable from Given/When/Then into test code

### FIX-PLAN-06: Fix plan document is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/plans/*-fix-plan.md` and confirm the fix plan is not in the uncommitted changes list.
- **pass_condition**: `git status --porcelain` output does not contain the fix plan path (zero output lines)
- **fail_diagnosis_hint**: If the fix plan is uncommitted, `git add` + `git commit` may not have been executed. Check the final step of the Phase executor for the commit operation
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 2: Create fix-plan-review.md**

Adapted from `examples/skills/feature-dev/references/done-criteria/plan-review.md`.
Key changes: references RCA Report instead of design document.

```markdown
---
name: fix-plan-review
max_retries: 3
audit: required
---

## Criteria

### FIX-PLAN-REVIEW-01: Review executed across all 3 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/fix-plan-review-review.json` or review log) and confirm execution records exist for all 3 perspectives (clarity, feasibility, consistency).
- **pass_condition**: Execution records exist for all 3 perspectives. Recorded perspective count is 3
- **fail_diagnosis_hint**: Identify the missing perspective and check the /implementation-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### FIX-PLAN-REVIEW-02: All consensus findings are resolved
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Extract findings with severity: consensus from the review results. For each finding, search for a corresponding fix commit or resolution statement in the fix plan document.
- **pass_condition**: Zero unresolved consensus findings
- **fail_diagnosis_hint**: Identify unresolved finding IDs and check the relevant sections of the fix plan. If fixes are not reflected, verify that the /implementation-review feedback loop completed
- **depends_on_artifacts**: [artifacts/reviews/, docs/plans/*-fix-plan.md]

### FIX-PLAN-REVIEW-03: Fix plan and RCA Report are consistent
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Cross-reference the RCA Report's Fix Strategy items with the fix plan's task list
  2. Verify component names, file paths, and data types used in the fix plan match the analysis in the RCA Report
  3. Verify that task completion conditions in the fix plan do not deviate from RCA findings (no additional features not justified by the root cause, no ignored impact scope items)
  4. Verify interfaces and code locations referenced in the fix plan match the RCA Report's Code Flow Trace and Root Cause sections
- **pass_condition**: Step 2: zero mismatches. Step 3: zero deviations. Step 4: zero reference inconsistencies
- **fail_diagnosis_hint**: Compare the inconsistent entries side-by-side between the RCA Report and fix plan. Check for cases where a review fix updated only one document. Trace the cause using `git log --oneline -- docs/plans/`
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md]

### FIX-PLAN-REVIEW-04: Each task's completion condition is verifiably specified
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Extract the completion condition (done condition / acceptance criteria) from each task in the fix plan
  2. Check that no completion condition contains subjective terms ("appropriate", "sufficient", "adequate", "correct")
  3. Verify each completion condition is expressed in one of the following forms: file existence check, command output numeric comparison, pattern match, or boolean state assertion
  4. Verify no task is missing a completion condition
- **pass_condition**: Step 2: zero completion conditions with subjective terms. Step 3: zero completion conditions failing to meet a verifiable form. Step 4: zero tasks without completion conditions
- **fail_diagnosis_hint**: Rewrite completion conditions containing subjective terms using numeric thresholds or pattern matches. For tasks missing conditions, derive them from the corresponding RCA Fix Strategy item. Convert unverifiable conditions into forms like "command X exit code is 0" or "file Y contains string Z"
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]
- **forward_check**: Completion conditions are at sufficient granularity for the Execute phase executor to self-evaluate task completion

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 3: Commit**

```bash
git add examples/skills/debug-flow/references/done-criteria/fix-plan.md examples/skills/debug-flow/references/done-criteria/fix-plan-review.md
git commit -m "feat(example): add debug-flow fix-plan and fix-plan-review done-criteria"
```

---

### Task 5: Create execute.md and test-review.md done-criteria

**Files:**
- Create: `examples/skills/debug-flow/references/done-criteria/execute.md`
- Create: `examples/skills/debug-flow/references/done-criteria/test-review.md`

- [ ] **Step 1: Create execute.md**

Adapted from `examples/skills/feature-dev/references/done-criteria/execute.md`.
Key changes: EXECUTE-02 traceability chain is RCA → fix-plan → implementation (3-tier), EXECUTE-06 references RCA Report Impact Scope, EXECUTE-07 references RCA Report, EXECUTE-09 references RCA Report.

```markdown
---
name: execute
max_retries: 3
audit: required
---

## Criteria

### EXECUTE-01: Code changes exist for every task
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all task IDs from the fix plan document
  2. Retrieve the list of changed files using `git diff --name-only`
  3. For each task ID, verify at least one corresponding code change exists among the changed files
  4. List task IDs that have no corresponding code changes
- **pass_condition**: Step 4 list is empty (all tasks have corresponding code changes)
- **fail_diagnosis_hint**: Identify task IDs without code changes and check the corresponding tasks in the fix plan. Determine whether it is a missing implementation or a documentation-only task whose changes do not appear in git diff
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md]

### EXECUTE-02: Build/compilation succeeds
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's build command (`npm run build`, `cargo build`, `go build ./...`, etc.) and record the exit code. Identify the build command from package.json, Cargo.toml, go.mod, Makefile, etc.
- **pass_condition**: Build command exit code is 0
- **fail_diagnosis_hint**: Check the first error message in the build error log. Identify whether it is a type error, import resolution failure, or missing dependency package. Report the affected file and line number
- **depends_on_artifacts**: [src/, artifacts/build/]

### EXECUTE-03: No lint or type-check errors
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's linter/type checker (`npm run lint`, `eslint`, `tsc --noEmit`, `cargo clippy`, `ruff check`, etc.) and record the error count.
- **pass_condition**: Linter/type checker exit code is 0, with zero error-level findings
- **fail_diagnosis_hint**: Check the file path and line number of each error finding. For type errors, check for type definition inconsistencies. For lint errors, review coding convention violations. Determine whether `--fix` can auto-correct the issues
- **depends_on_artifacts**: [src/, artifacts/lint/]

### EXECUTE-04: Full test suite passes
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Execute the project's test command (`npm test`, `cargo test`, `pytest`, `go test ./...`, etc.) and record the test result summary (total, passed, failed, skipped).
- **pass_condition**: Test command exit code is 0 with zero failed tests
- **fail_diagnosis_hint**: Check the names and error messages of failed tests. Use `git diff -- tests/` to distinguish between regressions of existing tests and first-time failures of new tests. For regressions, run `git stash && test command` to compare against the baseline
- **depends_on_artifacts**: [tests/, artifacts/test-results/]

### EXECUTE-05: Test code exists for every planned test case
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all test cases from the fix plan's test case section
  2. Extract all test function/method names from the test code directories (tests/, __tests__/, spec/, etc.) using `Grep`
  3. For each planned test case, verify at least one corresponding test code implementation exists
  4. List planned test cases without corresponding test code
- **pass_condition**: Step 4 list is empty (all planned test cases have corresponding test code)
- **fail_diagnosis_hint**: Identify test cases without corresponding test code and check the naming conventions. If test function names differ from planned test case names, verify correspondence by content inspection
- **depends_on_artifacts**: [docs/plans/*-fix-plan.md, tests/]

### EXECUTE-06: Implementation respects component boundaries
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the component boundaries from the RCA Report's Impact Scope and Architecture Context subsections
  2. Retrieve changed files using `git diff --name-only` and identify which component each file belongs to
  3. Verify that changes do not introduce new direct dependencies (import/require) that cross component boundaries
- **pass_condition**: Zero new direct dependencies that cross identified boundaries
- **fail_diagnosis_hint**: Identify the boundary-violating import/require statements and cross-reference with the RCA Report's Architecture Context. Check for cases where dependencies should go through an interface layer instead of direct references
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, src/]

### EXECUTE-07: End-to-end traceability from RCA to fix plan to implementation
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all Fix Strategy items from the RCA Report
  2. Identify the fix plan task ID corresponding to each Fix Strategy item
  3. Identify the implementation file/function corresponding to each task ID (using `git diff` + code search)
  4. Verify no gaps exist in the RCA-to-task-to-implementation three-tier mapping
  5. Verify no "surplus implementation" exists that has no corresponding Fix Strategy item or task
- **pass_condition**: Step 4: zero gaps in the three-tier mapping. Step 5: zero implementation files without corresponding fix plan tasks
- **fail_diagnosis_hint**: Identify where the mapping gap occurs (between Fix Strategy-to-task or task-to-implementation). For surplus implementations, determine whether to add them to the RCA/fix-plan documents or remove the excess code
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md, src/]

### EXECUTE-08: Newly added tests are not tautological
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List new or modified test files using `git diff --name-only -- tests/ __tests__/ spec/`
  2. Extract assertion statements (expect, assert, should, etc.) from each test file
  3. Verify that assertions exercise actual implementation code logic (i.e., call the function/method under test and verify its result)
  4. Check that assertions are not: constant-to-constant comparisons (`expect(1).toBe(1)`), mock return value verifications only (bypassing real code paths), or empty tests (no assertions)
- **pass_condition**: Zero tautological tests matching step 4 criteria. Every test includes at least one assertion that exercises a real implementation code path
- **fail_diagnosis_hint**: Identify tautological tests by file path and line number. Determine whether it is excessive mocking, missing assertions, or failure to invoke the function under test. Rewrite assertions to verify the actual behavior of implementation code
- **depends_on_artifacts**: [tests/, src/]

### EXECUTE-09: Test cases cover both fix scope and impact scope
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all Fix Strategy items from the RCA Report and assign IDs
  2. Enumerate the impacted files/modules from the RCA Report's Impact Scope subsection
  3. For each Fix Strategy item, verify at least one corresponding test case exists
  4. For each impacted file/module, verify corresponding tests exist
  5. Verify no existing tests for impacted code have been deleted or disabled (detect additions of `.skip`, `.only`, test function commenting out, or deletion via `git diff`)
- **pass_condition**: Step 3: all Fix Strategy items have corresponding tests. Step 4: all impacted areas have corresponding tests. Step 5: zero deletions or disablements of existing tests
- **fail_diagnosis_hint**: For step 3 gaps, identify missing Fix Strategy items and add tests. For step 4 gaps, identify uncovered impacted files and assess whether existing tests exist or new ones are needed. For step 5 detections, verify whether the deletion/disablement was intentional; if not, restore the tests
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, tests/, src/]
- **forward_check**: Prevents "insufficient test coverage" findings during the Code Review phase

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 2: Create test-review.md**

Adapted from `examples/skills/feature-dev/references/done-criteria/test-review.md`.
Key change: TEST-REVIEW-03 references RCA Report instead of design document.

```markdown
---
name: test-review
max_retries: 3
audit: required
---

## Criteria

### TEST-REVIEW-01: Review executed across all 3 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/test-review-review.json` or review log) and confirm execution records exist for all 3 perspectives (coverage, quality, design-alignment).
- **pass_condition**: Execution records exist for all 3 perspectives. Recorded perspective count is 3
- **fail_diagnosis_hint**: Identify the missing perspective and check the /test-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### TEST-REVIEW-02: All user-approved findings have been fixed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all user-approved findings from the review results
  2. Identify the target file and line for each finding
  3. Read the target file location using `Read` and verify the fix corresponding to each finding has been applied
  4. List findings where the fix has not been applied
- **pass_condition**: Step 4 list is empty (all user-approved findings have fixes applied)
- **fail_diagnosis_hint**: Identify unapplied findings and check the target file at the relevant line. Determine whether the fix was omitted or applied in a different form. Use `git log --oneline` to check for the existence of fix commits
- **depends_on_artifacts**: [artifacts/reviews/, tests/]

### TEST-REVIEW-03: All RCA Report test perspectives are covered by test code
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all test perspectives from the RCA Report's Reproduction Test section and the fix plan's test case section
  2. Extract all test function/method names from the test code directories (tests/, __tests__/, spec/, etc.) using `Grep`
  3. For each test perspective, verify at least one corresponding test function exists
  4. Check whether any test functions exist that are not covered by any test perspective (reverse coverage check)
  5. List uncovered test perspectives
- **pass_condition**: Step 5 list is empty (all test perspectives have corresponding test code)
- **fail_diagnosis_hint**: Identify uncovered test perspectives and check for imbalances. If test perspective titles and test function names follow different conventions, verify correspondence by content inspection
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md, tests/]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 3: Commit**

```bash
git add examples/skills/debug-flow/references/done-criteria/execute.md examples/skills/debug-flow/references/done-criteria/test-review.md
git commit -m "feat(example): add debug-flow execute and test-review done-criteria"
```

---

### Task 6: Create smoke-test.md and code-review.md done-criteria

**Files:**
- Create: `examples/skills/debug-flow/references/done-criteria/smoke-test.md`
- Create: `examples/skills/debug-flow/references/done-criteria/code-review.md`

These are identical to feature-dev's versions (design decision D2: independent copies).

- [ ] **Step 1: Create smoke-test.md**

```markdown
---
name: smoke-test
max_retries: 3
audit: required
---

## Criteria

### SMOKE-TEST-01: All smoke test steps pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Read the smoke test result files (logs/reports under `artifacts/smoke-test/`) and check the PASS/FAIL status of each step. If result files do not exist, re-execute the smoke tests.
- **pass_condition**: All steps have PASS status. Zero FAIL steps
- **fail_diagnosis_hint**: Check the name and error message of each FAIL step. Distinguish between browser operation failures (selector mismatch, timeout) and application errors (HTTP 5xx, exceptions). If screenshots are available in `artifacts/smoke-test/screenshots/`, review the screen state
- **depends_on_artifacts**: [artifacts/smoke-test/]

### SMOKE-TEST-02: Flaky tests are undetected or reported
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the smoke test result logs and detect cases where the same step produced different results on re-execution (FAIL on first run then PASS on second, or vice versa). If detected, verify they are recorded in the flaky test report list.
- **pass_condition**: Zero flaky detections, or all detected flaky cases are recorded in the report list
- **fail_diagnosis_hint**: Identify the flaky steps and investigate whether the cause is timing-dependent (setTimeout, animation waits), external service-dependent (API response delays), or test data-dependent (random data)
- **depends_on_artifacts**: [artifacts/smoke-test/]

### SMOKE-TEST-03: Test scenarios reflect project characteristics
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the project characteristics from the Evidence Plan (project_type, UI presence, API presence, etc.)
  2. Read the list of smoke test scenarios
  3. Determine whether required scenario categories exist for each project characteristic:
     - web-frontend: At least one scenario each for navigation, form interactions, and responsive display
     - API: At least one scenario each for successful responses (2xx) and error responses (4xx/5xx)
     - DB: At least one scenario each for data write and data read operations
  4. Enumerate the main user flows from the RCA Report's Impact Scope and verify at least one scenario exists for each impacted flow
- **pass_condition**: Step 3: all required categories have at least one scenario. Step 4: all impacted flows have a corresponding scenario. Zero missing categories or unmatched flows
- **fail_diagnosis_hint**: Identify missing categories and add smoke test scenarios for them. For unmatched flows, reference the RCA Report to create scenarios. If the Evidence Plan's project characteristics do not match reality, consider updating the Evidence Plan
- **depends_on_artifacts**: [artifacts/smoke-test/, docs/plans/*-rca-report.md]

### SMOKE-TEST-04: Smoke test execution evidence is valid
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Mechanically verify the following 3 points:
  1. `smoke-test-report.md` exists in the working directory and conforms to the required format (Step 2 table contains "Scenario", "Perspective", "Result", and "Screenshot" columns)
  2. At least one `smoke-*.png` file exists
  3. The "Screenshot" column in the Step 2 table of the report references existing `smoke-*.png` files
- **pass_condition**: All 3 points are satisfied
- **fail_diagnosis_hint**:
  - Report missing: The smoke-test skill was not properly executed. Check whether existing test suites (rspec, jest, etc.) were run as a substitute. If so, that is an invalid execution and the smoke-test skill must be re-run correctly
  - Format non-compliant: Regenerate the report
  - Screenshots missing: The browser-use CLI may not have been executed. If it is an environment issue, report to the user with a PAUSE status
- **depends_on_artifacts**: [smoke-test-report.md, smoke-*.png]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 2: Create code-review.md**

```markdown
---
name: code-review
max_retries: 3
audit: required
---

## Criteria

### CODE-REVIEW-01: Review executed across all 7 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/code-review-review.json` or review log) and confirm execution records exist for all 7 perspectives (simplify, code-quality, code-security, code-performance, code-test, ai-antipattern, code-impact).
- **pass_condition**: Execution records exist for all 7 perspectives. Recorded perspective count is 7
- **fail_diagnosis_hint**: Identify the missing perspective and check the /code-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### CODE-REVIEW-02: All user-approved findings have been fixed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all user-approved findings from the review results
  2. Identify the target file and line for each finding
  3. Read the target file location using `Read` and verify the fix corresponding to each finding has been applied
  4. List findings where the fix has not been applied
- **pass_condition**: Step 4 list is empty (all user-approved findings have fixes applied)
- **fail_diagnosis_hint**: Identify unapplied findings and check the target file at the relevant line. Determine whether the fix was omitted or applied in a different form. Use `git log --oneline` to check for the existence of fix commits
- **depends_on_artifacts**: [artifacts/reviews/, src/]

### CODE-REVIEW-03: No uncommitted changes and branch is within 50 commits of latest main
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git status --porcelain` to detect uncommitted changes
  2. Run `git rev-list --count HEAD ^main` (or `^master` if main does not exist) to get the branch divergence commit count
- **pass_condition**: Step 1 output is empty (zero uncommitted changes) and step 2 commit count is 50 or fewer
- **fail_diagnosis_hint**: If uncommitted changes exist, check for missed `git add` + `git commit` execution. If divergence exceeds 50 commits, consider rebasing onto the main branch. Long-lived branches carry high conflict risk; recommend early merging or branch splitting
- **depends_on_artifacts**: []

### CODE-REVIEW-04: High-severity impact findings have undergone user review
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Extract findings from the review result file where category is code-impact and severity is high or critical
  2. If zero such findings exist, PASS
  3. If one or more such findings exist, verify each has one of the following records:
     a. Fixed: a corresponding code change is included in a commit
     b. User-approved deferral: user's approval statement exists in the conversation log
     c. User-approved rejection: rationale for false positive was presented to the user and the user approved the rejection
  4. Verify no findings were deferred or rejected by the orchestrator without user confirmation
- **pass_condition**: All findings in step 3 fall under a, b, or c, and step 4 finds zero user-unconfirmed deferrals/rejections
- **fail_diagnosis_hint**: Identify user-unconfirmed findings and PAUSE to request user judgment. Check whether the orchestrator auto-deferred any findings without user confirmation
- **depends_on_artifacts**: [artifacts/reviews/]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 3: Commit**

```bash
git add examples/skills/debug-flow/references/done-criteria/smoke-test.md examples/skills/debug-flow/references/done-criteria/code-review.md
git commit -m "feat(example): add debug-flow smoke-test and code-review done-criteria"
```

---

### Task 7: Create reference files (audit-protocol, evidence-plan-protocol, fix-dispatch-strategy)

**Files:**
- Create: `examples/skills/debug-flow/references/audit-protocol.md`
- Create: `examples/skills/debug-flow/references/evidence-plan-protocol.md`
- Create: `examples/skills/debug-flow/references/fix-dispatch-strategy.md`

- [ ] **Step 1: Create audit-protocol.md**

Identical to feature-dev's version.

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
- primary: {artifacts from the work phase — RCA report, fix plan, code changes, etc.}
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
      "id": "RCA-01",
      "passed": true,
      "severity": "blocker",
      "detail": "RCA Report found at docs/plans/2026-04-07-foo-rca-report.md"
    }
  ],
  "summary": {
    "total": 8,
    "passed": 8,
    "failed": 0,
    "blocking_issues": [],
    "quality_warnings": ["RCA-06: Root Cause line number is approximate"]
  },
  "observations": [
    {
      "type": "quality",
      "content": "Investigation Record is thorough but Symmetry Check could be more detailed"
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
- **FAIL with escalation**: The auditor identifies a fundamental issue that retries cannot fix (e.g., root cause analysis is fundamentally flawed). Set `escalation` to a non-null object. This triggers an immediate PAUSE regardless of remaining retries.

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

- [ ] **Step 2: Create evidence-plan-protocol.md**

Adapted from feature-dev: triggers reference `rca-audit` and `fix-plan-review-audit` instead of `design-audit` and `plan-review-audit`. No `doc-maintenance` activity.

```markdown
# Evidence Plan Protocol

## Overview

The Evidence Plan defines what evidence must be collected during pipeline execution
to support audit decisions. It is generated once and updated as the RCA evolves.

## Lifecycle

| Event | Action |
|-------|--------|
| `rca-audit` PASS | Generate Evidence Plan |
| `fix-plan-review-audit` PASS | Re-evaluate if RCA report hash changed since generation |
| `execute` and later phases | Inject collection requirements into executor prompts |

## Generation

After `rca-audit` passes, the orchestrator generates the Evidence Plan by analyzing:

1. The RCA Report's Root Cause, Impact Scope, and Fix Strategy
2. The done-criteria for all upcoming phases
3. Project characteristics (language, framework, UI presence, API presence)

The plan is written to the `rca-audit` output directory.

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
      "phases": ["fix-plan-review", "code-review", "test-review"],
      "collect": ["review findings JSON", "consensus findings count", "applied fixes"]
    },
    {
      "type": "smoke-test",
      "phases": ["smoke-test"],
      "collect": ["smoke-test-report.md", "screenshots", "flaky test list"]
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

- [ ] **Step 3: Create fix-dispatch-strategy.md**

```markdown
# Fix Dispatch Strategy

When an audit phase returns FAIL, the orchestrator applies fixes using the
executor appropriate for the failed work phase.

## Dispatch Table

| Work Phase | Fix Executor | Strategy |
|------------|-------------|----------|
| rca | Orchestrator | Re-scan codebase, re-run exploration agents for missing info |
| fix-plan | Orchestrator | Edit fix plan doc directly based on audit findings |
| fix-plan-review | Orchestrator | Edit fix plan doc directly based on audit findings |
| execute | `feature-implementer` subagent | Decompose fix instructions into TDD tasks, launch with full task context |
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

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/references/audit-protocol.md examples/skills/debug-flow/references/evidence-plan-protocol.md examples/skills/debug-flow/references/fix-dispatch-strategy.md
git commit -m "feat(example): add debug-flow reference files (audit, evidence, fix-dispatch)"
```

---

### Task 8: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Run belt lint**

Run: `cargo run -p belt -- lint examples/skills/debug-flow/pipeline.yml`
Expected: lint passes. All sub-pipeline `uses:` paths resolve. No errors.

- [ ] **Step 2: Verify file count and structure**

Run: `find examples/skills/debug-flow -type f | sort`
Expected output (13 files):
```
examples/skills/debug-flow/SKILL.md
examples/skills/debug-flow/belt.toml
examples/skills/debug-flow/pipeline.yml
examples/skills/debug-flow/references/audit-protocol.md
examples/skills/debug-flow/references/done-criteria/code-review.md
examples/skills/debug-flow/references/done-criteria/execute.md
examples/skills/debug-flow/references/done-criteria/fix-plan-review.md
examples/skills/debug-flow/references/done-criteria/fix-plan.md
examples/skills/debug-flow/references/done-criteria/rca.md
examples/skills/debug-flow/references/done-criteria/smoke-test.md
examples/skills/debug-flow/references/done-criteria/test-review.md
examples/skills/debug-flow/references/evidence-plan-protocol.md
examples/skills/debug-flow/references/fix-dispatch-strategy.md
```

- [ ] **Step 3: Cross-reference consistency check**

Verify that every `config.criteria` value in pipeline.yml has a corresponding file in `references/done-criteria/`:
- `rca` → `references/done-criteria/rca.md` ✓
- `fix-plan` → `references/done-criteria/fix-plan.md` ✓
- `fix-plan-review` → `references/done-criteria/fix-plan-review.md` ✓
- `execute` → `references/done-criteria/execute.md` ✓
- `smoke-test` → `references/done-criteria/smoke-test.md` ✓
- `code-review` → `references/done-criteria/code-review.md` ✓
- `test-review` → `references/done-criteria/test-review.md` ✓

Verify that all `uses:` paths resolve:
- `../implementation-review/pipeline.yml` → `examples/skills/implementation-review/pipeline.yml` ✓
- `../code-review/pipeline.yml` → `examples/skills/code-review/pipeline.yml` ✓
- `../test-review/pipeline.yml` → `examples/skills/test-review/pipeline.yml` ✓

Verify regate targets are valid phase IDs:
- `[execute, smoke-test]` → both are phase IDs in pipeline.yml ✓
- `[execute]` → valid phase ID ✓

- [ ] **Step 4: Verify done-criteria frontmatter consistency**

Each done-criteria file's `name` field must match the pipeline.yml `config.criteria` value:
- `rca.md` → `name: rca` ✓
- `fix-plan.md` → `name: fix-plan` ✓
- `fix-plan-review.md` → `name: fix-plan-review` ✓
- `execute.md` → `name: execute` ✓
- `smoke-test.md` → `name: smoke-test` ✓
- `code-review.md` → `name: code-review` ✓
- `test-review.md` → `name: test-review` ✓
