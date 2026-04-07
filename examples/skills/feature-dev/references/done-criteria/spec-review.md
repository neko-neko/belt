---
name: spec-review
max_retries: 3
audit: required
---

## Criteria

### SPEC-REVIEW-01: Review executed across all 4 perspectives
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Read the review result file (`artifacts/reviews/spec-review-review.json` or review log) and confirm execution records exist for all 4 perspectives (requirements, design-judgment, feasibility, consistency).
- **pass_condition**: Execution records exist for all 4 perspectives. Recorded perspective count is 4
- **fail_diagnosis_hint**: Identify the missing perspective and check the /spec-review invocation options. Determine whether it was a missing perspective argument or a mid-execution interruption of the review agent
- **depends_on_artifacts**: [artifacts/reviews/]

### SPEC-REVIEW-02: All consensus findings are resolved
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Extract findings with severity: consensus from the review results. For each finding, search for a corresponding fix commit or resolution statement in the design document.
- **pass_condition**: Zero unresolved consensus findings
- **fail_diagnosis_hint**: Identify unresolved finding IDs and check the relevant sections of the design document. If fixes are not reflected, verify that the /spec-review feedback loop completed
- **depends_on_artifacts**: [artifacts/reviews/, docs/plans/*-design.md]

### SPEC-REVIEW-03: Review-driven fixes are reflected in the design document
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. List all approved findings from the review results
  2. For each finding, cross-reference the issue description with the relevant section of the design document
  3. Determine whether each issue has a corresponding change in the design document text
- **pass_condition**: All approved findings have corresponding changes in the design document. Zero unreflected findings
- **fail_diagnosis_hint**: Identify unreflected findings and compare the design document section side-by-side with the finding description. Determine whether it was an omission in applying the fix or an intentional skip
- **depends_on_artifacts**: [artifacts/reviews/, docs/plans/*-design.md]

### SPEC-REVIEW-04: Revised design document maintains internal consistency
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all cross-references within the design document (requirement IDs, component names, file paths, data types)
  2. Verify each reference target exists within the document and names match
  3. Confirm that every feature defined in the requirements section is mentioned in the design section's components
  4. Confirm that the test perspectives section targets align with the features in the requirements section
- **pass_condition**: Step 2: all reference targets exist with matching names. Step 3: all requirements have corresponding component descriptions. Step 4: test perspective targets match requirements. Zero inconsistencies
- **fail_diagnosis_hint**: Identify the inconsistent section names and reference source/target. Check for cases where a review fix updated one location but missed the corresponding reference. Trace the fix history using `git diff` to find the omission
- **depends_on_artifacts**: [docs/plans/*-design.md]
- **forward_check**: When the Plan phase decomposes design requirements into tasks, the requirements list must be uniquely enumerable

### SPEC-REVIEW-05: Requirements are specific enough to serve as input for the next phase
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate all requirements from the design document's requirements section
  2. Verify each requirement has a unique identifier (number, label, etc.)
  3. Confirm each requirement description includes "what" and "how" (at least one of: inputs/outputs, behavior, constraints)
  4. Assess whether each requirement's description alone is sufficient to derive tasks in the Plan phase
- **pass_condition**: All requirements have identifiers (step 2), each requirement includes at least one of inputs/outputs, behavior, or constraints (step 3), and zero requirements lack identifiers or have ambiguous descriptions
- **fail_diagnosis_hint**: Identify requirements lacking identifiers or those that specify only "what" without "how". Add inputs/outputs, behavior descriptions, or constraint definitions to the relevant sections of the design document
- **depends_on_artifacts**: [docs/plans/*-design.md]
- **forward_check**: Requirements are at sufficient granularity for the Plan phase to decompose them into tasks

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
