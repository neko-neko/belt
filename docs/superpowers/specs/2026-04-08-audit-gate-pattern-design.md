# Audit Gate Pattern: Reusable Audit Mechanism Sub-Pipeline

## Summary

Extract the repeated audit gate pattern from `feature-dev` and `debug-flow` pipelines into a reusable `audit-gate` sub-pipeline. The audit-gate captures the **mechanism** (phase-auditor dispatch, done-criteria evaluation, verdict, fix cycle, re-audit) as a typed, composable building block.

## Motivation

Both `feature-dev` (9 audit phases) and `debug-flow` (7 audit phases) repeat an identical structural pattern for every audit phase:

```yaml
- id: X-audit
  description: "Audit X against done-criteria"
  config:
    audit: required
    criteria: "X"
  gate:
    - has_output: true
  validate:
    - "All criteria in references/done-criteria/X.md pass"
  confirm: true
  max_retries: 3
  regate: [...]  # pipeline-specific
```

This repetition is:
- **Error-prone**: changing the audit mechanism requires modifying 16 phases across 2 pipelines.
- **Undiscoverable**: the audit pattern is implicit — a new pipeline author must reverse-engineer it from examples.
- **Not composable**: the pattern cannot be referenced via `uses:`, the primary composition mechanism in belt.

## Design

### audit-gate Sub-Pipeline

A single-phase sub-pipeline defining the audit mechanism as a type.

**`audit-gate/pipeline.yml`**:

```yaml
name: audit-gate
description: "Reusable audit gate — dispatches phase-auditor against done-criteria"
version: 1
phases:
  - id: check
    description: "Phase-auditor dispatch against done-criteria"
    config:
      audit: required
    gate:
      - has_output: true
    validate:
      - "All audit criteria pass"
    confirm: true
    max_retries: 3
```

### Consumer Integration

Three injection points via belt's sub-pipeline expansion rules:

| Injection Point | Mechanism | Expander Rule |
|----------------|-----------|---------------|
| `criteria` | Parent's `config.criteria` | Merged into last sub-phase (parent wins) |
| `regate` | Parent's `regate: [...]` | Appended to last sub-phase |
| `when` | Parent's `when:` | Propagated to all sub-phases |

**Consumer usage patterns**:

```yaml
# Basic audit
- id: design-audit
  uses: ../audit-gate/pipeline.yml
  config:
    criteria: "design"

# Audit with regate
- id: code-review-audit
  uses: ../audit-gate/pipeline.yml
  config:
    criteria: "code-review"
  regate: [execute, smoke-test, doc-audit]

# Conditional audit
- id: smoke-test-audit
  when: "args.smoke"
  uses: ../audit-gate/pipeline.yml
  config:
    criteria: "smoke-test"
```

### Expanded Phase IDs

After expansion, phase IDs change from `X-audit` to `X-audit/check`. This is transparent to the SKILL.md orchestrator because audit phase detection uses `config.audit == "required"`, not the phase ID. Regate targets reference work phases (leaf), which are unaffected.

### Fixed Values

`confirm: true` and `max_retries: 3` are defined in the sub-phase and cannot be overridden by the parent (expander does not inherit these fields). All current audit phases use these values. If a consumer needs different values, they define a leaf phase directly instead of using audit-gate.

## Done-Criteria Schema

### Format Definition (`_schema.md`)

```markdown
---
name: <criteria-name>          # matches config.criteria
max_retries: 3
audit: required
---

## Criteria

### <PREFIX>-<NN>: <criterion title>
- **severity**: blocker | quality
- **verify_type**: automated | inspection
- **verification**:
  <specific verification steps>
- **pass_condition**: <quantitative pass condition>
- **fail_diagnosis_hint**: <investigation guidance for auditor on FAIL>
- **depends_on_artifacts**: [<dependent artifact paths>]
- **forward_check**: <impact note for downstream phases>

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
```

**Field specification**:

| Field | Required | Description |
|-------|----------|-------------|
| `severity` | Yes | `blocker` = causes FAIL, `quality` = warning only |
| `verify_type` | Yes | `automated` = command-executable, `inspection` = LLM judgment |
| `verification` | Yes | Verification steps. Concrete commands/tool calls for automated |
| `pass_condition` | Yes | Pass condition expressed quantitatively |
| `fail_diagnosis_hint` | Yes | Guidance for auditor to determine fix strategy on FAIL |
| `depends_on_artifacts` | No | Artifact paths this criterion depends on |
| `forward_check` | No | Note to prevent redundant verification in downstream phases |

**ID naming convention**: `<PHASE_PREFIX>-<NN>` (e.g., `EXECUTE-01`, `CODE-REVIEW-03`). Prefix corresponds to the uppercased done-criteria filename.

### Standard Catalog

| File | Content | Shareability |
|------|---------|-------------|
| `execute.md` | Implementation completion (9 criteria: code changes exist, tests pass, boundary compliance, traceability, etc.) | Generic version provided. Consumers customize artifact references ("plan document" / "RCA report") |
| `code-review.md` | Review completion (7-perspective execution, fix application, regression, impact handling) | **Usable as-is** (identical between feature-dev and debug-flow) |
| `smoke-test.md` | Smoke test results (scenario coverage, evidence collection, category coverage) | Generic version provided. Consumers customize artifact references |
| `test-review.md` | Test review completion (3-perspective execution, test perspective coverage) | Generic version provided. Consumers customize artifact references |

**Generic version policy**: Context-dependent artifact references (e.g., `docs/plans/*-design.md`) are replaced with generic expressions (e.g., `the primary plan document`). Consumers copy and concretize.

## Shared Mechanism Spec (audit-protocol.md)

The audit-protocol.md defines the dispatch procedure, verdict format, and failure handling. The protocol is structurally identical between feature-dev and debug-flow — only example values differ (artifact names in templates and sample JSON).

The shared version genericizes three example locations:

| Current (feature-dev) | Current (debug-flow) | Shared |
|---|---|---|
| `design docs, plan docs, code changes, etc.` | `RCA reports, fix plan docs, code changes, etc.` | `artifacts from the work phase` |
| `"id": "DESIGN-01"` | `"id": "RCA-01"` | `"id": "CRITERIA-01"` |
| `"DESIGN-05: alternatives section is brief"` | `"RCA-05: reproduction test steps..."` | `"CRITERIA-05: section could be more detailed"` |

The protocol's six sections are preserved:

1. **Overview** — audit phase purpose
2. **Auditor Dispatch** — `config.audit == "required"` detection, done-criteria loading, phase-auditor launch, verdict write
3. **Audit Context Template** — information injected into phase-auditor prompt
4. **Verdict Format** — JSON schema (verdict, criteria_results, summary, observations, escalation)
5. **Verdict Rules** — PASS/FAIL/FAIL-with-escalation determination
6. **Failure Handling** — fix-dispatch, re-audit cycle, max_retries exhaustion PAUSE

Consumers reference audit-gate's audit-protocol.md directly. Consumer-specific files (fix-dispatch-strategy.md, evidence-plan-protocol.md) remain with the consumer.

## Directory Structure

```
examples/skills/
│
├── audit-gate/                          # NEW
│   ├── pipeline.yml
│   ├── references/
│   │   └── audit-protocol.md
│   └── done-criteria/
│       ├── _schema.md
│       ├── execute.md
│       ├── code-review.md
│       ├── smoke-test.md
│       └── test-review.md
│
├── feature-dev/                         # MODIFIED
│   ├── pipeline.yml                     #   audit phases → uses: ../audit-gate/...
│   ├── SKILL.md                         #   audit-protocol path updated
│   ├── belt.toml                        #   unchanged
│   └── references/
│       ├── fix-dispatch-strategy.md     #   unchanged (pipeline-specific)
│       ├── evidence-plan-protocol.md    #   unchanged (pipeline-specific)
│       └── done-criteria/              #   unchanged (consumer-managed)
│           ├── design.md
│           ├── spec-review.md
│           ├── plan.md
│           ├── plan-review.md
│           ├── execute.md
│           ├── code-review.md
│           ├── smoke-test.md
│           ├── test-review.md
│           └── doc-audit.md
│
├── debug-flow/                          # MODIFIED
│   ├── pipeline.yml                     #   audit phases → uses: ../audit-gate/...
│   ├── SKILL.md                         #   audit-protocol path updated
│   ├── belt.toml                        #   unchanged
│   └── references/
│       ├── fix-dispatch-strategy.md     #   unchanged
│       ├── evidence-plan-protocol.md    #   unchanged
│       └── done-criteria/
│           ├── rca.md
│           ├── fix-plan.md
│           ├── fix-plan-review.md
│           ├── execute.md
│           ├── code-review.md
│           ├── smoke-test.md
│           └── test-review.md
│
├── code-review/                         # unchanged
├── spec-review/                         # unchanged
├── test-review/                         # unchanged
├── implementation-review/               # unchanged
└── smoke-test/                          # unchanged
```

## Change Summary

| Change Type | Target | Count |
|------------|--------|-------|
| NEW (directory) | `audit-gate/` | 1 |
| NEW (files) | pipeline.yml, audit-protocol.md, _schema.md, 4 done-criteria | 7 |
| MODIFIED | feature-dev/pipeline.yml, debug-flow/pipeline.yml, 2 SKILL.md | 4 |
| DELETE | feature-dev/references/audit-protocol.md, debug-flow/references/audit-protocol.md | 2 |

## Reduction Effect

- feature-dev audit phase definitions: ~72 lines → ~32 lines (55% reduction)
- debug-flow audit phase definitions: ~56 lines → ~25 lines (55% reduction)

## Lint Compatibility

- Regate targets (execute, smoke-test, doc-audit) are all leaf phases — lint regate reference check unaffected
- `uses:` file existence check — passes if `../audit-gate/pipeline.yml` exists
- Expanded IDs `X-audit/check` — engine processes normally

## Constraints and Trade-offs

### confirm / max_retries not overridable

These fields are defined on the sub-phase and not inherited from the parent (expander design). All current audit phases use `confirm: true` and `max_retries: 3`. If a consumer needs different values, they bypass audit-gate and define a leaf phase directly.

### Done-criteria duplication

Common done-criteria (e.g., code-review.md) exist both in audit-gate's catalog and in consumer directories. This is intentional: consumers own their done-criteria and may evolve them independently. The audit-gate catalog serves as canonical templates, not as a shared library.

### No nested uses:

The expander does not support nested `uses:` references. This design only sub-pipelines the audit phases (which are leaf by nature), so this constraint is not triggered.

## Out of Scope

- `audit: lite` mode — not used in any current pipeline
- Work phase sub-pipelining — existing review sub-pipelines (code-review, spec-review, etc.) are unchanged
- Template expansion for done-criteria — belt's `with:` field is unimplemented
- fix-dispatch-strategy.md / evidence-plan-protocol.md sharing — pipeline-specific, stays with consumers
