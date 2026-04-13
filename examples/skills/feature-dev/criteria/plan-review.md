---
name: plan-review
max_retries: 3
audit: required
---

## Criteria

### PLAN-REVIEW-01: Review executed across all 3 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/plan-review-review.json` or review log) and confirm execution records exist for all 3 perspectives (clarity, feasibility, consistency).
- **pass_condition**: Execution records exist for all 3 perspectives. Recorded perspective count is 3
- **fail_diagnosis_hint**: Identify the missing perspective and check the /implementation-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### PLAN-REVIEW-02: All consensus findings are resolved
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Extract findings with severity: consensus from the review results. For each finding, search for a corresponding fix commit or resolution statement in the plan document.
- **pass_condition**: Zero unresolved consensus findings
- **fail_diagnosis_hint**: Identify unresolved finding IDs and check the relevant sections of the plan document. If fixes are not reflected, verify that the /implementation-review feedback loop completed
- **depends_on_artifacts**: [artifacts/reviews/, docs/plans/*-plan.md]

### PLAN-REVIEW-03: Plan document and design document are consistent
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Cross-reference the design document's requirements list with the plan document's task list
  2. Verify component names, file paths, and data types used in the plan document match the definitions in the design document
  3. Verify that task completion conditions in the plan document do not deviate from design requirements (no additional features not in the design document, no ignored design constraints)
  4. Verify interfaces defined in the design document (function signatures, API endpoints, etc.) are correctly referenced in the plan document
- **pass_condition**: Step 2: zero mismatches in component names, paths, or types. Step 3: zero deviations. Step 4: zero reference inconsistencies
- **fail_diagnosis_hint**: Compare the inconsistent entries side-by-side between the design and plan documents. Check for cases where a review fix updated only one document. Trace the cause using `git log --oneline -- docs/plans/`
- **depends_on_artifacts**: [docs/plans/*-design.md, docs/plans/*-plan.md]

### PLAN-REVIEW-04: Each task's completion condition is verifiably specified
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Extract the completion condition (done condition / acceptance criteria) from each task in the plan document
  2. Check that no completion condition contains subjective terms ("appropriate", "sufficient", "adequate", "correct")
  3. Verify each completion condition is expressed in one of the following forms: file existence check, command output numeric comparison, pattern match, or boolean state assertion
  4. Verify no task is missing a completion condition
- **pass_condition**: Step 2: zero completion conditions with subjective terms. Step 3: zero completion conditions failing to meet a verifiable form. Step 4: zero tasks without completion conditions
- **fail_diagnosis_hint**: Rewrite completion conditions containing subjective terms using numeric thresholds or pattern matches. For tasks missing conditions, derive them from the corresponding design requirement. Convert unverifiable conditions into forms like "command X exit code is 0" or "file Y contains string Z"
- **depends_on_artifacts**: [docs/plans/*-plan.md]
- **forward_check**: Completion conditions are at sufficient granularity for the Execute phase executor to self-evaluate task completion

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
