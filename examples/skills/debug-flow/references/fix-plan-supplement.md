# Fix Plan Supplement (Phase 2 override for `/writing-plans`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 2 (INVOKE 1 = Read this file; INVOKE 2 = `/writing-plans`).

## Output path override

Write the fix plan to:

```
docs/plans/YYYY-MM-DD-<topic>-fix-plan.md
```

Path convention: see `./path-convention.md`.

## Consumed inputs

- `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` (produced by Phase 1)

Read this file in full before authoring the fix plan. Extract the `## Fix Strategy` section as the driver of task decomposition.

## Mandatory traceability

Every task in the fix plan MUST map to at least one Fix Strategy item in the RCA Report (blocker per FIX-PLAN-02). Include a task-to-Fix Strategy mapping table at the top of the fix plan document. Example:

| Task # | Fix Strategy ID |
|---|---|
| 1 | FS-1 |
| 2 | FS-2 |
| 3 | FS-2 |

## Task granularity

- Each task MUST have ≤10 steps (per FIX-PLAN-03)
- Each task MUST span <3 modules (per FIX-PLAN-03)
- If a task exceeds either limit, split it

## Given/When/Then test cases

Every task MUST include at least one test case in Given/When/Then format (per FIX-PLAN-05). Example:

```markdown
**Test case:**
- Given: user has expired session
- When: GET /dashboard
- Then: 302 redirect to /login
```

Then clauses MUST contain verifiable expected values (numeric thresholds, pattern-matchable assertions, boolean state).

## Verifiable completion conditions

Every task MUST have a completion condition expressible as:
- File existence check (e.g., "file X exists at path Y"), OR
- Command output numeric comparison (e.g., "exit code == 0"), OR
- Pattern match (e.g., "grep returns ≥1 line"), OR
- Boolean state assertion (e.g., "feature flag is enabled")

Reject subjective terms (「適切に」「十分に」「correct」).

## RCA artifacts reference

- Include a reference line to the consumed RCA Report at the top of the fix plan (e.g., "Based on: `docs/plans/YYYY-MM-DD-<topic>-rca-report.md`")
- If `--e2e` and `rca-scenarios.yml` exists, reference it so monkey-test phase can extend the scenarios list with fix-specific Given/When/Then entries
