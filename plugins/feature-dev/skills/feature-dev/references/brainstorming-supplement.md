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

1. `belt-agent:code-explorer` — trace existing code flow related to the feature area;
   report dependencies, patterns, constraints.
2. `belt-agent:code-architect` — identify architecture patterns, conventions, reusable
   components in the feature area.
3. `belt-agent:impact-analyzer` — reverse-trace the change target; report shared state,
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
