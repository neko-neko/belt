# Review Sub-pipeline Migration Design

**Date**: 2026-04-07
**Status**: Draft
**Linear**: BELT-20 (parent)
**Depends on**: feature-dev belt migration (2026-04-07-feature-dev-belt-migration.md)

## Summary

Migrate the 4 review child skills (spec-review, implementation-review, code-review, test-review) from dotfiles SKILL.md-only skills into belt sub-pipelines under `examples/skills/`. Each review skill gets an independent 2-phase pipeline (review → fix) referenced from feature-dev via `uses:`. Agent configurations are declared in pipeline.yml `config` for self-documenting structure.

## Goals

1. **Self-contained examples**: feature-dev and all review child skills exist in `examples/skills/`
2. **Declarative agent config**: `config.agents` declares which subagent_types each skill uses, readable from pipeline.yml alone
3. **No belt-core changes**: existing config passthrough (`HashMap<String, Value>`) is sufficient
4. **Lean Orchestrator preservation**: feature-dev delegates dispatch to child SKILL.md

## Non-Goals

- Migrating doc-audit, handover (remain SKILL.md-only)
- Shared review-cycle template (4 skills are independently defined)
- belt-core changes for `with:` args resolution
- Modifying audit phases or regate topology in feature-dev

## Design

### Directory Structure

```
examples/skills/
├── feature-dev/              # orchestrator (pipeline.yml updated)
│   ├── pipeline.yml          # uses: references to 4 review + smoke-test
│   ├── belt.toml
│   ├── SKILL.md              # dispatch delegation rule added
│   └── references/           # unchanged
├── spec-review/              # NEW
│   ├── pipeline.yml
│   ├── belt.toml
│   └── SKILL.md
├── implementation-review/    # NEW
│   ├── pipeline.yml
│   ├── belt.toml
│   └── SKILL.md
├── code-review/              # NEW
│   ├── pipeline.yml
│   ├── belt.toml
│   └── SKILL.md
├── test-review/              # NEW
│   ├── pipeline.yml
│   ├── belt.toml
│   └── SKILL.md
├── smoke-test/               # existing (unchanged)
├── linear-refresh/           # existing (unchanged)
├── linear-add/               # existing (unchanged)
└── linear-cleanup/           # existing (unchanged)
```

### belt.toml

Each review skill has a minimal `belt.toml`:

```toml
pipeline = "pipeline.yml"
```

> **Note**: The existing `feature-dev/belt.toml` uses `pipeline_file` instead of `pipeline`. This is a pre-existing key name mismatch (belt-core config.rs expects `pipeline`). Out of scope for this spec but should be fixed separately.

### 2-Phase Review Pipeline Pattern

All 4 review skills share a common 2-phase structure:

```yaml
name: <skill-name>
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }      # spec-review, implementation-review only
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "<N>-perspective <domain> review"
    config:
      agents: ["<subagent-type-1>", "<subagent-type-2>", ...]
      ui_agent: "<subagent-type-ui>"       # optional
      skills: ["/<skill-name>"]            # optional (code-review only)
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

**Phase behavior**:
- `review`: orchestrator reads `config.agents`, dispatches each as `Agent(subagent_type=<name>)` in parallel. N-way voting with `config.iterations`. User approves triage results (`confirm: true`).
- `fix`: feature-implementer applies accepted fixes. Gate verifies output was produced.

### Config Key Convention

| Category | Key | Type | Meaning |
|----------|-----|------|---------|
| Structural | `agents` | `string[]` | subagent_type names to dispatch via Agent tool |
| Structural | `ui_agent` | `string` | Additional agent when `args.ui` is true |
| Structural | `skills` | `string[]` | Additional skills to invoke via Skill tool |
| Runtime | `codex` | `"args.codex"` | Codex parallel review flag (resolved by SKILL.md) |
| Runtime | `iterations` | `"args.iterations"` | N-way voting count (resolved by SKILL.md) |
| Runtime | `swarm` | `"args.swarm"` | Team parallel mode (resolved by SKILL.md) |

Structural keys are the sub-pipeline's identity. Runtime keys reference top-level pipeline args via `"args.X"` string literals, resolved by the orchestrator SKILL.md.

### Per-Skill Pipeline Definitions

#### spec-review

```yaml
name: spec-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "4-perspective spec review"
    config:
      agents:
        - spec-review-requirements
        - spec-review-design-judgment
        - spec-review-feasibility
        - spec-review-consistency
      ui_agent: spec-review-ui-design
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

#### implementation-review

```yaml
name: implementation-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "3-perspective plan review"
    config:
      agents:
        - implementation-review-clarity
        - implementation-review-feasibility
        - implementation-review-consistency
      ui_agent: implementation-review-ui-spec
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

#### code-review

```yaml
name: code-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "7-perspective code review"
    config:
      agents:
        - code-review-quality
        - code-review-security
        - code-review-performance
        - code-review-test
        - code-review-ai-antipattern
        - code-review-impact
      skills:
        - "/simplify"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

No `ui_agent`. Additional `skills: ["/simplify"]` for Skill tool invocation.

#### test-review

```yaml
name: test-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }
phases:
  - id: review
    description: "3-perspective test review"
    config:
      agents:
        - test-review-coverage
        - test-review-quality
        - test-review-design-alignment
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"
    confirm: true
  - id: fix
    description: "Fix accepted findings"
    gate:
      - has_output: true
```

No `ui_agent`, no `skills`. The `test-review-design-alignment` agent requires a design spec path, resolved at runtime by the orchestrator from the output directory or evidence plan.

### Feature-dev Integration

#### pipeline.yml Changes

Review phases change from `config.skill` to `uses:`:

```yaml
# Before
- id: spec-review
  description: "4-perspective spec review"
  config:
    skill: "/spec-review"
    codex: "args.codex"
    iterations: "args.iterations"
    swarm: "args.swarm"
    ui: "args.ui"

# After
- id: spec-review
  uses: ../spec-review/pipeline.yml
```

Same pattern for `implementation-review`, `code-review`, `test-review`.

#### Expanded Phase Structure

After sub-pipeline expansion:

```
design                        leaf
design-audit                  leaf
spec-review/review            expanded
spec-review/fix               expanded
spec-review-audit             leaf (unchanged)
plan                          leaf
plan-audit                    leaf
plan-review/review            expanded
plan-review/fix               expanded
plan-review-audit             leaf (unchanged)
execute                       leaf
execute-audit                 leaf
doc-audit                     leaf (when: args.doc)
doc-audit-audit               leaf
smoke-test/...                expanded (existing)
smoke-test-audit              leaf
code-review/review            expanded
code-review/fix               expanded
code-review-audit             leaf (unchanged)
test-review/review            expanded (when: args.e2e)
test-review/fix               expanded (when: args.e2e)
test-review-audit             leaf (unchanged)
integrate                     leaf
```

#### Regate Topology: Unchanged

```yaml
code-review-audit:
  regate: [execute, smoke-test, doc-audit]
test-review-audit:
  regate: [execute]
```

All regate targets are feature-dev-level phase IDs. Sub-pipeline expansion does not affect them.

#### SKILL.md Dispatch Delegation

Feature-dev SKILL.md adds one generic dispatch rule:

```
Sub-pipeline phase (id contains "/"):
  → Load SKILL.md from the sub-pipeline's skill directory
  → Follow that SKILL.md's dispatch rules for the sub-phase
```

Flow example:
1. `belt-agent next` returns `spec-review/review` with config
2. feature-dev SKILL.md recognizes `spec-review/` prefix
3. Loads `../spec-review/SKILL.md`
4. spec-review SKILL.md reads `config.agents`, dispatches via Agent tool

Feature-dev does not know child skills' agent configurations. Lean Orchestrator pattern preserved.

### Review SKILL.md Pattern

Each review skill's SKILL.md follows the Authoring Principle (3 responsibilities):

**1. Dispatch Rules**:
- `review` phase: read `config.agents` → dispatch parallel agents → N-way voting → triage → user confirms
- `fix` phase: dispatch feature-implementer with accepted findings → verify gate
- Conditional: if `args.ui` and `config.ui_agent` → add to dispatch list
- Conditional: if `config.skills` → invoke via Skill tool
- Conditional: if `config.codex` → add Codex parallel review
- Conditional: if `config.swarm` → use TeamCreate for agent team

**2. Domain Constraints**:
- Voting threshold: majority (>50% of iterations must agree)
- Severity: blocker (fix required) / warning (user choice) / info (no action)
- No modifications to original code intent

**3. Red Flags**:
- Do not filter findings before user presentation
- Do not auto-approve triage
- Do not combine review and fix in a single step

Per-skill additions:
- **code-review**: `/simplify` runs after review, before triage. Findings are independent improvement suggestions.
- **test-review**: `test-review-design-alignment` needs design spec path, resolved from output directory `*-design.md` or evidence plan.

### What Does NOT Change

| Component | Reason |
|-----------|--------|
| belt-core | Config is already `HashMap<String, Value>` passthrough |
| belt-agent SKILL.md | Protocol layer is generic; review-specific knowledge is in skill SKILL.md |
| feature-dev/references/done-criteria/ | Audit phases stay in feature-dev; done-criteria unchanged |
| feature-dev/references/audit-protocol.md | phase-auditor dispatch is independent of review sub-pipelines |
| feature-dev/references/fix-dispatch-strategy.md | Fix dispatch table uses audit phase IDs, unaffected by expansion |

## Risks

| Risk | Mitigation |
|------|------------|
| `with:` may not resolve `"args.X"` references from parent pipeline | Runtime keys use `"args.X"` string literals resolved by SKILL.md, not by belt-core. No belt-core change needed. |
| Sub-pipeline expansion increases total phase count | Minimal impact: 4 skills x 2 phases = 8 additional expanded phases. belt handles this without performance concern. |
| Review SKILL.md duplication across 4 skills | Acceptable for 4 files. Common pattern is documented in this spec. Shared template (Approach 3) was considered and rejected for simplicity. |
