# RCA Supplement (Phase 1 override for `/systematic-debugging`)

**Invoked by:** `SKILL.md` Phase 1 (INVOKE 1 = Read this file; INVOKE 2 = `/systematic-debugging`).

## Output path override

Write the RCA Report to:

```
docs/plans/YYYY-MM-DD-<topic>-rca-report.md
```

Path convention: see `./path-convention.md`.

## Required RCA Report sections

The report MUST contain these five top-level sections (`##` level):

1. `## Symptom` — User-observable symptom, reproduction steps, error messages
2. `## Investigation Record` — Four subsections:
   1. `### Code Flow Trace` — call chains (file path + function name pairs)
   2. `### Architecture Context` — relevant patterns, conventions, implicit rules
   3. `### Impact Scope` — affected files / modules (paths must exist per RCA-03)
   4. `### Symmetry Check` — whether change target has paired paths (required per RCA-08)
3. `## Root Cause` — file:line location + mechanism explanation (per RCA-06)
4. `## Reproduction Test` — test file path + assertion; test MUST currently FAIL (per RCA-05)
5. `## Fix Strategy` — ordered list of remediation steps

An additional section `## Excluded Hypotheses` (or equivalent within Investigation Record) MUST record at least one alternative root cause, its verification method, and rejection reason (per RCA-04).

## Parallel exploration order

Orchestrator dispatches exploration subagents in parallel, then synthesizes:

1. `belt-agents:code-explorer` — entry-point tracing and data flow
2. `belt-agents:code-architect` — architecture patterns and implicit contracts
3. `belt-agents:impact-analyzer` — reverse dependencies and shared state

**After** subagent results return, the orchestrator reconstructs the root cause **itself** (do NOT forward broad research verbatim into Reproduction Test / Fix Strategy). See bug-fix SKILL.md Red Flag "Never delegate root cause synthesis to subagents."

## Reproduction test requirement

Write a failing test that captures the bug mechanism (RCA-05 blocker). The test must:
- Be placed in an appropriate test directory for the project (see `tests/` / `spec/` conventions)
- Currently FAIL when run (before fix)
- Transition to PASS only after `execute` phase applies the fix

## `--e2e` additional output

When `args.e2e=true`, additionally produce:

```
docs/plans/YYYY-MM-DD-<topic>-rca-scenarios.yml
```

Content: Given/When/Then YAML with at least one scenario. The first scenario MUST correspond to the RCA Reproduction Test (see `monkey-test-supplement.md`).

Example format:

```yaml
scenarios:
  - name: "Reproduce login 500 error"
    given: "The user has an expired session cookie"
    when: "The user navigates to /dashboard"
    then: "The server returns 302 redirect to /login (not 500)"
  - name: "Regression: valid session still works"
    given: "The user has a fresh session cookie"
    when: "The user navigates to /dashboard"
    then: "The dashboard page renders successfully"
```
