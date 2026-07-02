---
name: spec-review
max_retries: 3
audit: lite
---

## Criteria

### SPEC-REVIEW-01: Strategy structure survives the review
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the post-review `test-strategy.md`
  2. Verify the required sections (`Test Design Techniques` / `Quality Characteristics` / `Priority Matrix`) remain intact (structural parity with TEST-SCENARIOS-02)
- **pass_condition**: All three sections still present after applied fixes
- **fail_diagnosis_hint**: A review fix deleted or renamed a required section — restore it from git history
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-02: Finding triage is complete
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the merged findings (locate the `findings` artifact via `belt-agent status`)
  2. Verify both the grill-me group and the selection group are fully processed (every finding has a recorded resolution or selection)
- **pass_condition**: Zero unhandled findings
- **fail_diagnosis_hint**: Resume the grill-me dialogue / selection prompt in the main context; the orchestrator must not auto-resolve
- **depends_on_artifacts**: [findings]

### SPEC-REVIEW-03: Only user-approved findings are reflected
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate applied changes to `test-strategy.md` / `scenarios.yml`
  2. Verify each traces to a user-approved finding (grill-me: `accept` or `accept_current`; selection: picked by number)
- **pass_condition**: Zero applied changes without a user-approved finding
- **fail_diagnosis_hint**: Revert unapproved edits; re-run triage if attribution is unclear
- **depends_on_artifacts**: [findings, docs/features/*/test-strategy.md]

### SPEC-REVIEW-04: Applied changes are confined to the deliverable files
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git diff --name-only` against the parent phase baseline
  2. Verify only `docs/features/<topic>/test-strategy.md` (and `scenarios.yml` when `args.e2e` is true) are modified — no source, test, or unrelated doc changes
- **pass_condition**: All diff entries fall within the deliverable scope; zero out-of-scope files
- **fail_diagnosis_hint**: Revert out-of-scope edits or move them to the owning phase
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-05: Scenarios are in review scope when --e2e
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `args.e2e=false`, PASS (vacuously satisfied)
  2. Verify `scenarios.yml` was in scope for the review (scenarios are referenced in the findings)
- **pass_condition**: Non-e2e run, OR scenarios referenced in at least one finding context
- **fail_diagnosis_hint**: Re-dispatch the review with `scenarios.yml` included in the reviewed spec set
- **depends_on_artifacts**: [findings, docs/features/*/scenarios.yml]

### SPEC-REVIEW-06: Modified deliverables are committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/features/` and confirm zero output lines.
- **pass_condition**: Zero unstaged/uncommitted deliverable changes
- **fail_diagnosis_hint**: Commit the applied review fixes
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-07: Merged findings.json exists and parses
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Verify the file exists, parses as JSON, and contains a `findings` array
- **pass_condition**: File exists AND valid JSON AND `findings` array present
- **fail_diagnosis_hint**: The `/belt:spec-review` merge step was interrupted — re-invoke from the spec-review phase
- **depends_on_artifacts**: [findings]

### SPEC-REVIEW-08: Internal markdown links still resolve
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Extract internal links (`[text](./path)` / `[text](#anchor)`) from the updated deliverables
  2. Verify each target file/heading exists; list broken links
- **pass_condition**: Step 2 list is empty
- **fail_diagnosis_hint**: Fix the broken targets (path typo, renamed heading) or update the link
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
