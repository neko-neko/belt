---
name: integrate
max_retries: 3
audit: required
---

## Criteria

### INTEGRATE-01: Integration method was chosen and executed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The SKILL.md Phase 8 A/B prompt was presented to the user
  2. Either `wt merge` (option A) or `gh pr create` (option B) was executed
  3. Execution logs (or git state) reflect the chosen method
- **pass_condition**: One of the two methods was executed per user choice

### INTEGRATE-02: All pre-merge checks pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. `cargo test` (if Rust changes) exit 0
  2. `cargo clippy --workspace -- -D warnings` exit 0
  3. `cargo fmt --check` exit 0 for modified packages
  4. belt lint for any modified pipeline.yml files exit 0
- **pass_condition**: All applicable checks exit 0

### INTEGRATE-03: Reproduction test PASSes on integrated branch
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. After `wt merge` to main / after PR creation, re-run the test identified in RCA-05 Reproduction Test
  2. Confirm it now PASSes (previously FAILed per RCA-05)
- **pass_condition**: Reproduction test PASSes on the integrated branch
- **fail_diagnosis_hint**: Merge introduced a regression, or reproduction test expectations drifted. Review integration diff

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
