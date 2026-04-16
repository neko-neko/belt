---
name: dogfood
max_retries: 3
audit: required
---

## Criteria

### DOGFOOD-01: Dogfood report file exists (directory form)
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-dogfood-report/report.md")`
- **pass_condition**: At least one match (directory `docs/plans/<topic>-dogfood-report/` must contain `report.md`)
- **fail_diagnosis_hint**: `/dogfood` default output is `./dogfood-output/`; `dogfood-supplement.md` must override to `docs/plans/<topic>-dogfood-report/`. Confirm Phase 7 supplement was loaded
- **depends_on_artifacts**: [docs/plans/*-dogfood-report/report.md]

### DOGFOOD-02: Root Cause mechanism does not re-emerge in exploration
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the Root Cause section of `*-rca-report.md`, extract the specific mechanism description
  2. In dogfood report, confirm an explicit statement that the mechanism was re-verified (e.g., "after fix, the XXX condition no longer triggers")
  3. If the exploration uncovered the same mechanism re-manifesting in a different code path, flag FAIL
- **pass_condition**: Re-verification statement present AND no re-emergence of the mechanism
- **fail_diagnosis_hint**: Fix is incomplete or has asymmetric coverage. Re-examine RCA Symmetry Check output and Fix Strategy

### DOGFOOD-03: Fix scope exploration documented
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. The dogfood report explicitly lists the Fix Impact Scope (derived from `fix_plan_doc`)
  2. Exploration covered Impact Scope areas + Symmetry pairs (from RCA-08)
- **pass_condition**: Impact Scope listed AND exploration coverage matches

### DOGFOOD-04: Evidence (screenshots / videos OR CLI-only rationale)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Check `docs/plans/<topic>-dogfood-report/screenshots/` or `.../videos/` for at least one evidence file
  2. If Impact Scope contains zero UI files (CLI / API / backend-only fix), accept a rationale paragraph in `report.md` explaining CLI-only exploration scope (per spec "dogfood graceful degradation for UI-less bug fixes")
- **pass_condition**: ≥1 evidence file under screenshots/ or videos/, OR rationale paragraph present for CLI-only fixes
- **fail_diagnosis_hint**: For UI-touching fixes, `/dogfood` should emit screenshots by default. For CLI-only fixes, ensure the rationale paragraph was added per supplement instructions
- **depends_on_artifacts**: [docs/plans/*-dogfood-report/]

### DOGFOOD-05: Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**: `git status --porcelain -- docs/plans/*-dogfood-report/` returns empty
- **pass_condition**: zero uncommitted changes under the dogfood report directory

### DOGFOOD-06: Narrative note captures exploratory regression results
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-dogfood.md`
  2. Verify frontmatter contains `phase: dogfood` and `run_id: <run_id>`
  3. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify Observations records Symmetry-Pair probe results and Impact Scope coverage
  5. Verify Concerns flags any Root Cause mechanism re-emergence signals; Directives carries forward regression guards for integrate
- **pass_condition**: Steps 1-5 all pass
- **fail_diagnosis_hint**: If Observations missing Symmetry-Pair results, re-derive from RCA report's symmetry analysis. If Concerns empty, explicitly affirm that no regression signals surfaced (or re-explore). See `plugins/belt-agents/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-dogfood.md]

## Observation Collection

The belt-agents:phase-auditor MUST include `observations[]` in its verdict output.
