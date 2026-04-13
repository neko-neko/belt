---
name: doc-audit
max_retries: 3
audit: required
---

## Criteria

### DOC-AUDIT-01: Doc audit report exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Check for the existence of a doc-audit report file in the output directory or working directory.
- **pass_condition**: At least one doc-audit report file exists
- **fail_diagnosis_hint**: Verify that `/doc-audit` skill was invoked and completed. Check if the report was written to the correct directory.
- **depends_on_artifacts**: []

### DOC-AUDIT-02: All broken dependency links are resolved
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Parse all markdown files with `depends-on` frontmatter
  2. Verify each declared dependency path exists via Glob
  3. Check for undeclared dependencies (file path references in body text without `depends-on` entry)
- **pass_condition**: Zero broken dependency links. Zero undeclared dependencies detected.
- **fail_diagnosis_hint**: Run `doc-check` to identify and fix broken dependencies. Undeclared deps need `depends-on` frontmatter additions.
- **depends_on_artifacts**: [docs/]

### DOC-AUDIT-03: No stale documentation detected
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Compare doc-audit findings against the current code state
  2. Verify that documentation references (function names, file paths, API endpoints) match the current implementation
  3. Check that documented behavior matches actual behavior for changed modules
- **pass_condition**: Zero stale documentation findings unresolved. All flagged items either fixed or confirmed current.
- **fail_diagnosis_hint**: Cross-reference the doc-audit report's stale signals with `git diff` to identify which docs need updating. Focus on docs whose `depends-on` targets were modified.
- **depends_on_artifacts**: [docs/, src/]

### DOC-AUDIT-04: User-approved fixes applied
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. List all findings from the doc-audit report that were approved by the user for fixing
  2. Verify each approved fix has been applied (check git diff or file content)
  3. Ensure no approved fix was skipped or partially applied
- **pass_condition**: All user-approved fixes fully applied. Zero approved-but-unapplied items.
- **fail_diagnosis_hint**: Check the doc-audit report for the list of approved findings and cross-reference with recent commits. Partially applied fixes may need manual completion.
- **depends_on_artifacts**: [docs/]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
