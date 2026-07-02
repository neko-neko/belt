---
name: fix-plan-review
max_retries: 3
audit: lite
---

## Criteria

### FIX-PLAN-REVIEW-01: Review artifact (findings.json) exists
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Verify the file exists at the resolved_path
  3. Parse as JSON and confirm a `findings` array field is present
- **pass_condition**: File exists AND parses as valid JSON AND contains a `findings` array
- **fail_diagnosis_hint**: `/belt:spec-review` invocation interrupted or artifact path drift. Re-invoke the skill from the fix-plan-review phase
- **depends_on_artifacts**: [findings]

### FIX-PLAN-REVIEW-02: Fix plan and RCA Report are consistent
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Cross-reference the RCA Report's Fix Strategy list with the fix plan document's task list
  2. Verify component names, file paths, and data types used in the fix plan document match the definitions in the RCA Report
  3. Verify that task completion conditions in the fix plan document do not deviate from Fix Strategy items
  4. Verify interfaces defined in the RCA Report (function signatures, API endpoints, etc.) are correctly referenced in the fix plan document
- **pass_condition**: Zero mismatches in component names / paths / types, zero deviations, zero reference inconsistencies
- **fail_diagnosis_hint**: Compare inconsistent entries side-by-side. If a review fix updated only one document, trace cause via `git log --oneline -- docs/features/`
- **depends_on_artifacts**: [docs/features/*/rca-report.md, docs/features/*/fix-plan.md]

### FIX-PLAN-REVIEW-03: No unresolved blocker findings in review artifact
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Parse the JSON at that resolved_path
  3. Filter findings where `severity == "blocker"`
  4. For each blocker finding, verify either (a) a resolution comment / fix commit is referenced in the fix plan, OR (b) the finding has been explicitly rejected by user triage
- **pass_condition**: Zero unresolved blocker findings
- **fail_diagnosis_hint**: User triage (accept/reject for each finding) is incomplete, or fix commits have not landed. Re-run the `/belt:spec-review` fix phase with accepted blocker findings
- **depends_on_artifacts**: [findings, docs/features/*/fix-plan.md]

### FIX-PLAN-REVIEW-04: Grill-me group findings each have a resolution
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Enumerate grill-me group findings at that resolved_path
  3. For each grill-me finding, verify a `resolution` of `accept`, `reject`, or `accept_current` has been recorded by the user
  4. List grill-me findings lacking a recorded resolution
- **pass_condition**: Step 4 list is empty — every grill-me finding has one of the three recognised resolutions
- **fail_diagnosis_hint**: Resume the grill-me dialogue in the main context. Do NOT let the orchestrator auto-resolve; each finding needs explicit user judgement
- **depends_on_artifacts**: [findings]

### FIX-PLAN-REVIEW-05: Selection-group findings have a recorded user selection
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Enumerate selection-group findings at that resolved_path
  3. For each selection-group finding, verify the user recorded a numbered selection (either applied or explicitly skipped)
  4. List selection-group findings with no recorded user selection
- **pass_condition**: Step 4 list is empty — every selection-group finding has a recorded user selection (applied or skipped)
- **fail_diagnosis_hint**: Return to the selection prompt in the main context. The user must pick by number or explicitly skip each selection-group finding
- **depends_on_artifacts**: [findings]

### FIX-PLAN-REVIEW-06: Applied changes are confined to the fix-plan document(s)
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git diff --name-only` against the parent phase baseline
  2. Verify every modified file is a fix-plan document (e.g. `docs/features/*/fix-plan.md`) — no source, test, or unrelated doc changes leaked in
- **pass_condition**: All diff entries fall under the fix-plan document scope; zero out-of-scope files modified
- **fail_diagnosis_hint**: An out-of-scope edit was made during fix-plan-review. Revert unrelated changes or move them into a separate phase (execute, rca, etc.)
- **depends_on_artifacts**: [docs/features/*/fix-plan.md]

### FIX-PLAN-REVIEW-07: Internal markdown links in the fix-plan still resolve
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Extract internal markdown links (`[text](./path)` / `[text](#anchor)`) from the updated fix-plan document(s)
  2. For each link target, verify the referenced file/heading exists
  3. List any broken anchors or dangling relative paths
- **pass_condition**: Step 3 list is empty — every internal link resolves
- **fail_diagnosis_hint**: Fix the broken link targets (typo in path, renamed heading, etc.) or update the link. Broken cross-references degrade downstream agents' navigation
- **depends_on_artifacts**: [docs/features/*/fix-plan.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output. Record
quality/warning-level findings even for criteria that PASS. Observations
accumulate in the pipeline's audit trail.
