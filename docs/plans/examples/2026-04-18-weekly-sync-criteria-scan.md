---
phase: 2
name: scan
max_retries: 3
audit: required
---

## Criteria

### SCAN-01: Both Linear and output-adapter scans completed without error
- severity: blocker
- verify_type: automated
- verification:
  1. Check artifacts/scan/phase-2-linear.json exists and contains a
     non-empty `tickets` array (or explicitly empty with `"tickets": []`).
  2. Check artifacts/scan/phase-2-output.json exists and contains a
     non-empty `items` array (or explicitly empty with `"items": []`).
  3. Parse both files as JSON and verify structural validity.
- pass_condition: Steps 1-3 all pass; both JSON files are valid.
- fail_diagnosis_hint: If Linear scan failed, re-run with --verbose. If
  adapter scan failed, check gh CLI authentication and project permissions.
- uses_evidence: [E-LINEAR-SCAN, E-OUTPUT-ADAPTER-SCAN]
- depends_on_artifacts: [artifacts/scan/]

### SCAN-02: Scan narrative note exists with 4 required sections
- severity: blocker
- verify_type: inspection
- verification:
  1. Read belt-agent status and locate artifact scan_notes resolved_path.
  2. Verify file exists at resolved_path.
  3. Verify frontmatter contains `phase: scan` and `run_id: <run_id>`.
  4. Verify 4 required sections exist: ## Decisions, ## Concerns,
     ## Directives, ## Observations.
  5. Verify Observations records ticket count and external link count.
- pass_condition: Steps 1-5 all pass.
- fail_diagnosis_hint: If any section is missing, re-open the note and fill
  it (at minimum `(none)` placeholder). See
  plugins/belt-agent/references/narrative-convention.md for schema.
- depends_on_artifacts: [scan_notes]
