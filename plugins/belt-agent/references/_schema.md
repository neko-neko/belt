---
name: _schema
description: >-
  Done-criteria schema for files consumed by the orchestrator
  when evaluating validate: file criteria. Defines frontmatter fields,
  criterion structure, and severity semantics.
---

# Done-Criteria Schema

This document defines the format for done-criteria files (originally introduced by the now-retired audit-gate pattern).
Each done-criteria file is consumed by the orchestrator when it evaluates a phase's validate: file reference (the audit-gate pattern is retired).

## File Format

Done-criteria files use markdown with YAML frontmatter:

### Frontmatter (required)

```yaml
---
name: <criteria-name>          # Must match the config.criteria value
max_retries: 3                 # Must match audit-gate's max_retries
audit: required                # Audit mode
---
```

### Body Structure

```markdown
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

The orchestrator MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
```

## Field Specification

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `severity` | Yes | `blocker` or `quality` | `blocker` causes verdict FAIL. `quality` produces warnings only |
| `verify_type` | Yes | `automated` or `inspection` | `automated`: verifiable by command execution. `inspection`: requires LLM judgment |
| `verification` | Yes | text | Step-by-step verification procedure. For `automated`, include concrete commands |
| `pass_condition` | Yes | text | Quantitative pass condition (counts, existence, patterns) |
| `fail_diagnosis_hint` | Yes | text | Guidance for the orchestrator to determine root cause and fix strategy on FAIL |
| `depends_on_artifacts` | No | list | Artifact paths this criterion depends on |
| `forward_check` | No | text | Note to prevent redundant verification in downstream audit phases |

## Naming Conventions

- **Criterion ID**: `<PHASE_PREFIX>-<NN>` where prefix is the uppercased criteria name
  - `execute` → `EXECUTE-01`, `EXECUTE-02`, ...
  - `code-review` → `CODE-REVIEW-01`, `CODE-REVIEW-02`, ...
- **File name**: matches `config.criteria` value with `.md` extension
  - `config.criteria: "execute"` → `execute.md`

## Verdict Rules

The orchestrator applies these rules to produce a verdict:

- **PASS**: All `blocker` criteria pass. Quality warnings are reported but do not block
- **FAIL**: At least one `blocker` criterion fails
- **FAIL with escalation**: Fundamental issue that retries cannot fix. Triggers immediate PAUSE

## See also

- [`evidence-schema.md`](./evidence-schema.md) — evidence schema (the sibling type-only core for evidence-catalog files; criteria may reference evidence via `uses_evidence`).
